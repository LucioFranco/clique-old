use {
    crate::{
        client,
        codec::{Msg, MsgCodec},
        rpc::{
            proto::{Peer, Push},
            MemberServer,
        },
        state::{NodeState, State},
    },
    futures::{
        sync::mpsc::{self, Receiver, Sender},
        Sink, Stream,
    },
    futures_util::join,
    log::{error, info, trace},
    std::{net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        net::{UdpFramed, UdpSocket},
        prelude::*,
        timer::Interval,
    },
    tower_grpc::Request,
    uuid::Uuid,
};

pub struct Node {
    addr: SocketAddr,
    inner: Arc<State>,
}

impl Node {
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            addr,
            inner: Arc::new(State::new()),
        }
    }

    pub async fn join(&self, peers: Vec<SocketAddr>) -> Result<(), ()> {
        // TODO: add proper address selection process
        let join_addr = peers.into_iter().next().expect("One addrs required");

        let local_addr = self.addr.clone();
        let uri: http::Uri = format!("http://{}:{}", local_addr.ip(), local_addr.port())
            .parse()
            .unwrap();

        let from_address = local_addr.clone();

        let id = self.inner.id().to_string();

        let mut client = await!(client::connect(&join_addr, uri)).unwrap();

        let from = Peer {
            id: id.to_string(),
            address: from_address.to_string(),
        };

        let request = client.join(Request::new(Push {
            from: Some(from.clone()),
            peers: vec![from],
        }));

        let response = await!(request).unwrap();

        let body = response.into_inner();

        let peers = body
            .peers
            .into_iter()
            .map(|peer| {
                (
                    peer.address.parse().unwrap(),
                    Uuid::parse_str(peer.id.as_str()).unwrap(),
                )
            })
            .collect();

        self.inner.peers_sync(peers);
        self.inner.update_state(NodeState::Connected);

        let from = body.from.unwrap();

        info!(
            "Connected to clique cluster via {}:{}",
            from.id, from.address
        );

        Ok(())
    }

    pub async fn serve(&self) -> Result<(), std::io::Error> {
        let (tx, rx) = mpsc::channel(1000);
        let socket = UdpSocket::bind(&self.addr)?;

        let addr = socket.local_addr()?;

        let udp_listener = self.listen_udp(socket, (tx.clone(), rx));
        let tcp_listener = self.listen_tcp(addr.clone());
        let gossiper = self.gossip(tx);

        join!(tcp_listener, udp_listener, gossiper);

        Ok(())
    }

    async fn listen_tcp(&self, addr: SocketAddr) {
        if let Err(e) = await!(MemberServer::serve(self.inner.clone(), &addr)) {
            error!("Error listening for rpc calls: {}", e);
        }
    }

    async fn listen_udp(
        &self,
        socket: UdpSocket,
        (tx, mut rx): (Sender<(Msg, SocketAddr)>, Receiver<(Msg, SocketAddr)>),
    ) {
        info!("Listening on: {}", self.addr);

        let (mut sink, mut stream) = UdpFramed::new(socket, MsgCodec).split();

        let message_receiver = async {
            while let Some(Ok(msg)) = await!(stream.next()) {
                trace!("Received: {:?}", msg);
                let tx = tx.clone();

                if let Err(e) = await!(self.process_message(tx, msg)) {
                    error!("Receiving message: {}", e);
                }
            }
        };

        let message_sender = async {
            while let Some(Ok(msg)) = await!(rx.next()) {
                trace!("Sending: {:?} to: {:?}", msg.0, msg.1);

                if let Err(e) = await!(sink.send_async(msg)) {
                    error!("Sending message: {}", e);
                }
            }
        };

        join!(message_receiver, message_sender);
    }

    async fn process_message(
        &self,
        mut tx: Sender<(Msg, SocketAddr)>,
        (msg, addr): (Msg, SocketAddr),
    ) -> Result<(), mpsc::SendError<(Msg, SocketAddr)>> {
        match msg {
            Msg::Ping(broadcasts) => {
                self.inner.apply_broadcasts(broadcasts);

                let ack = {
                    let broadcasts = self.inner.broadcasts_mut().drain();
                    Msg::Ack(broadcasts)
                };

                let msg = (ack, addr);
                await!(tx.send_async(msg))
            }

            Msg::Ack(broadcasts) => {
                self.inner.apply_broadcasts(broadcasts);
                Ok(())
            }
        }
    }

    async fn gossip(&self, tx: Sender<(Msg, SocketAddr)>) {
        let mut interval = Interval::new_interval(Duration::from_secs(1));

        while let Some(_) = await!(interval.next()) {
            // Take a snapshot of the current set of peers
            let peers = self.inner.peers().clone();

            trace!("Sending heartbeats");

            let broadcasts = self.inner.broadcasts_mut().drain().clone();

            let heartbeats = peers
                .iter()
                .map(move |(_id, peer)| {
                    let ping = Msg::Ping(broadcasts.clone());
                    (ping, peer.addr())
                })
                .collect::<Vec<_>>();

            for heartbeat in heartbeats {
                let tx = tx.clone();
                await!(tx.send(heartbeat)).expect("Unable to send heartbeat");
            }
        }
    }
}
