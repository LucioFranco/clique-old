use {
    crate::{
        codec::{Msg, MsgCodec},
        error::Result,
        peer::Peer,
        rpc::proto::{Peer as PeerProto, Push},
        rpc::{client::Client, server::MemberServer},
        state::{NodeState, State},
    },
    futures::{
        channel::mpsc::{self, Receiver, Sender},
        compat::{Future01CompatExt, Sink01CompatExt, Stream01CompatExt},
        join, SinkExt, StreamExt,
    },
    log::{error, info, trace},
    std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration},
    tokio::{
        net::{UdpFramed, UdpSocket},
        prelude::Stream as Stream01,
        timer::Interval,
    },
    uuid::Uuid,
};

/// `Node` is none thread safe representation of the current node. It maintains
/// all the state within clique.
pub struct Node {
    addr: SocketAddr,
    inner: Arc<State>,
}

impl Node {
    /// Create a new `Node` that listens on the provided [SocketAddr]
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            addr,
            inner: Arc::new(State::new()),
        }
    }

    /// Get a snapshot of the current list of peers. This currently allocates a new
    /// [HashMap] everytime its called to avoid lock contention. This function is
    /// async due to it aquiring a lock.
    pub async fn peers(&self) -> HashMap<SocketAddr, Peer> {
        let peers = await!(self.inner.peers().lock());
        peers
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect::<HashMap<SocketAddr, Peer>>()
    }

    /// Join the cluster via a list of Peers. Currently, only selects the
    /// first node in the list.
    pub async fn join(&self, peers: Vec<SocketAddr>) -> Result<()> {
        // TODO: add proper address selection process
        let join_addr = peers.into_iter().next().expect("One addrs required");

        let local_addr = self.addr;
        let uri: http::Uri = format!("http://{}:{}", local_addr.ip(), local_addr.port())
            .parse()
            .unwrap();

        let from_address = local_addr;

        let id = await!(self.inner.id().lock()).to_string();

        let mut client = await!(Client::connect(&join_addr, uri))?;

        let from = PeerProto {
            id: id.to_string(),
            address: from_address.to_string(),
        };

        let push = Push {
            from: Some(from.clone()),
            peers: vec![from],
        };

        let response = await!(client.join(push))?;

        let peers = {
            let body = response;
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
                .filter(|(addr, _id)| addr == &self.addr)
                .collect()
        };

        await!(self.inner.peers_sync(peers));
        await!(self.inner.update_state(NodeState::Connected));

        Ok(())
    }

    /// Spawna long running future that will start both the listeners
    /// on `TCP` and `UDP`. It will also start the gossiper and failure
    /// detection portions of the application.
    pub async fn serve(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel(1000);
        let socket = UdpSocket::bind(&self.addr)?;

        let addr = socket.local_addr()?;

        let udp_listener = self.listen_udp(socket, (tx.clone(), rx));
        let tcp_listener = self.listen_tcp(addr);
        let gossiper = self.gossip(tx.clone());
        let failures = self.failures(Duration::from_secs(1));

        join!(tcp_listener, udp_listener, gossiper, failures);

        Ok(())
    }

    async fn listen_tcp(&self, addr: SocketAddr) {
        if let Err(e) = await!(MemberServer::serve(self.inner.clone(), &addr).compat()) {
            error!("Error listening for rpc calls: {}", e);
        }
    }

    async fn listen_udp(
        &self,
        socket: UdpSocket,
        channel: (Sender<(Msg, SocketAddr)>, Receiver<(Msg, SocketAddr)>),
    ) {
        info!("Listening on: {}", self.addr);

        let (tx, mut rx) = channel;

        let framed = UdpFramed::new(socket, MsgCodec);

        let (mut sink, mut stream) = {
            let (sink, stream) = framed.split();
            (sink.compat_sink(), stream.compat())
        };

        let message_receiver = async {
            while let Some(Ok(msg)) = await!(stream.next()) {
                trace!("Received: {:?}", msg);
                let tx = tx.clone();

                await!(self.process_message(tx, msg));
            }
        };

        let message_sender = async {
            if let Err(e) = await!(sink.send_all(&mut rx)) {
                error!("Error sending message: {}", e);
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
                await!(self.inner.apply_broadcasts(broadcasts));

                let ack = {
                    let mut broadcasts = await!(self.inner.broadcasts().lock());
                    let broadcasts = broadcasts.drain();
                    Msg::Ack(seq, broadcasts)
                };

                let msg = (ack, addr);
                if let Err(e) = await!(tx.send(msg)) {
                    error!("Error sending ack: {}", e);
                }
            }

            Msg::Ack(seq, broadcasts) => {
                let failures = self.inner.failures();
                let mut failures = await!(failures.lock());
                failures.handle_ack(seq);
                await!(self.inner.apply_broadcasts(broadcasts));
            }
            _ => unimplemented!(),
        };
    }

    async fn gossip(&self, tx: Sender<(Msg, SocketAddr)>) {
        use tokio::prelude::StreamAsyncExt;
        let mut interval = Interval::new_interval(Duration::from_secs(1));

        while let Some(_) = await!(interval.next()) {
            trace!("Sending heartbeats");

            let broadcasts = {
                // This acquires a lock so we need to make sure it gets dropped
                // before any `await!`.
                let mut broadcasts = await!(self.inner.broadcasts().lock());
                broadcasts.drain()
            };

            let heartbeats = {
                let peers = await!(self.inner.peers().lock());
                peers.clone().into_iter().collect::<Vec<_>>()
            };

            let pings = {
                let mut msg = Vec::new();
                let failures = self.inner.failures();
                let mut failures = await!(failures.lock());

                for (_, peer) in heartbeats {
                    let addr = peer.addr();
                    let seq_num = failures.add(addr);
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
        let mut interval = Interval::new_interval(interval).compat();

        while let Some(_) = await!(interval.next()) {
            let failed_nodes = {
                let mut failures = await!(self.inner.failures().lock());
                failures.gather()
            };


            let mut peers = await!(self.inner.peers().lock());
            for failed_node in failed_nodes {
                peers.get_mut(&failed_node);
            }

        }
    }
}
