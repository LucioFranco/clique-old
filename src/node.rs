use {
    crate::{
        codec::{Msg, MsgCodec},
        error::Result,
        rpc::{
            client,
            proto::{Peer, Push},
            server::MemberServer,
        },
        state::{NodeState, State},
    },
    futures::{
        channel::mpsc::{self, Receiver, Sender},
        join, SinkExt, StreamExt,
        future::{FutureExt, TryFutureExt}
    },
    log::{error, info, trace},
    std::{net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        net::{UdpFramed, UdpSocket},
        timer::Interval,
        prelude::{SinkExt as OtherSinkExt, StreamAsyncExt, Stream},
        await
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

    pub async fn join(&self, peers: Vec<SocketAddr>) -> Result<()> {
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

        let response = await!(request)?;

        let peers = {
            let body = response.into_inner();
            let from = body.from.unwrap();

            info!(
                "Connected to clique cluster via {}:{}",
                from.id, from.address
            );

            body.peers
                .into_iter()
                .map(|peer| {
                    (
                        peer.address.parse().unwrap(),
                        Uuid::parse_str(peer.id.as_str()).unwrap(),
                    )
                })
                .collect()
        };

        self.inner.peers_sync(peers);
        self.inner.update_state(NodeState::Connected);

        Ok(())
    }

    pub async fn serve(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel(1000);
        let socket = UdpSocket::bind(&self.addr)?;

        let addr = socket.local_addr()?;

        let udp_listener = self.listen_udp(socket, (tx.clone(), rx));
        let tcp_listener = self.listen_tcp(addr.clone());
        let gossiper = self.gossip(tx);
        let failures = self.failures(Duration::from_secs(1));

        join!(tcp_listener, udp_listener, gossiper, failures);

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


        let framed = UdpFramed::new(socket, MsgCodec);

        let (mut sink, mut stream) = framed.split();

        let message_receiver = async {
            while let Some(Ok(msg)) = await!(stream.next()) {
                trace!("Received: {:?}", msg);
                let tx = tx.clone();

                await!(self.process_message(tx, msg));
            }
        };

        let message_sender = async {
            while let Ok(Some(msg)) = await!(rx.next().unit_error().boxed().compat()) {
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
    ) {
        match msg {
            Msg::Ping(seq, broadcasts) => {
                self.inner.apply_broadcasts(broadcasts);

                let ack = {
                    let broadcasts = self.inner.broadcasts_mut().drain();
                    Msg::Ack(seq, broadcasts)
                };

                let msg = (ack, addr);
                if let Err(e) = await!(tx.send(msg)) {
                    error!("Error sending ack: {}", e);
                }
            }

            Msg::Ack(seq, broadcasts) => {
                self.inner.failures_mut().handle_ack(&seq);
                self.inner.apply_broadcasts(broadcasts);
            }
        };
    }

    async fn gossip(&self, tx: Sender<(Msg, SocketAddr)>) {
        use tokio::prelude::StreamAsyncExt;
        let mut interval = Interval::new_interval(Duration::from_secs(1));

        while let Some(_) = await!(interval.next()) {
            // Take a snapshot of the current set of peers
            let peers = self.inner.peers().clone();

            trace!("Sending heartbeats");

            let broadcasts = {
                // This acquires a lock so we need to make sure it gets dropped
                // before any `await!`.
                let mut broadcasts = self.inner.broadcasts_mut();
                broadcasts.drain()
            };

            let heartbeats = peers
                .iter()
                .collect::<Vec<_>>();

            let pings = {
                let mut msg = Vec::new();
                let mut failures = self.inner.failures_mut();

                for (_, peer) in heartbeats {
                    let addr = peer.addr();
                    let seq_num = failures.add(addr.clone());
                    msg.push((Msg::Ping(seq_num, broadcasts.clone()), addr));
                }
                msg
            };

            for ping in pings {
                let mut tx = tx.clone();
                await!(tx.send(ping)).expect("Unable to send ping");
            }
        }
    }

    async fn failures(&self, interval: Duration) {
        let mut interval = Interval::new_interval(interval);

        while let Some(_) = await!(interval.next()) {
            let failed_nodes = {
                let mut failures = self.inner.failures_mut();
                failures.gather()
            };
            
            if !failed_nodes.is_empty() {
                info!("Timeouts: {:?}", failed_nodes);
            }
        }
    }
}
