use {
    crate::{
        codec::Msg,
        error::Result,
        handlers::PingHandler,
        peer::Peer,
        rpc::proto::{Peer as PeerProto, Push},
        rpc::{client::Client, server::MemberServer},
        state::{NodeState, State},
        task::spawn,
        transport::Transport,
    },
    futures::{channel::mpsc::Receiver, compat::Future01CompatExt, join, StreamExt},
    log::{debug, error, info, trace},
    std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration},
    tokio::timer::Interval,
    uuid::Uuid,
};

/// `Node` is none thread safe representation of the current node. It maintains
/// all the state within clique.
pub struct Node<T> {
    addr: SocketAddr,
    transport: T,
    inner: Arc<State>,
}

impl<T: Transport> Node<T> {
    /// Create a new `Node` that listens on the provided [SocketAddr]
    pub fn new(addr: SocketAddr) -> Self {
        let (listen_fut, transport) = T::serve(addr);

        let local_addr = transport.local_addr();
        let state = Arc::new(State::new());

        let state_clone = state.clone();
        spawn(
            async move {
                let tcp_listener = Self::listen_tcp(local_addr, state_clone);
                join!(tcp_listener, listen_fut);
            },
        );

        Node {
            addr,
            inner: state,
            transport,
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

            let mut peers = body
                .peers
                .into_iter()
                .map(|peer| {
                    (
                        peer.address.parse().unwrap(),
                        Uuid::parse_str(peer.id.as_str()).unwrap(),
                    )
                })
                //.filter(|(addr, _id)| addr == &self.addr)
                .collect::<Vec<_>>();

            let from = (
                from.address.parse().unwrap(),
                Uuid::parse_str(from.id.as_str()).unwrap(),
            );
            peers.push(from);
            peers
        };

        await!(self.inner.peers_sync(peers));
        await!(self.inner.update_state(NodeState::Connected));

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let messages = self.transport.recv();

        let gossip = self.gossip();
        let process = self.process_messages(messages);

        join!(gossip, process);

        Ok(())
    }

    async fn listen_tcp(addr: SocketAddr, inner: Arc<State>) {
        if let Err(e) = await!(MemberServer::serve(inner, &addr).compat()) {
            error!("Error listening for rpc calls: {}", e);
        }
    }

    async fn process_messages(&self, mut messages: Receiver<(Msg, SocketAddr)>) {
        while let Some((msg, addr)) = await!(messages.next()) {
            match msg {
                Msg::Ping(seq, broadcasts) => {
                    await!(self.inner.apply_broadcasts(broadcasts));

                    let ack = {
                        let mut broadcasts = await!(self.inner.broadcasts().lock());
                        let broadcasts = broadcasts.get();
                        Msg::Ack(seq, broadcasts)
                    };

                    let msg = (ack, addr);
                    if let Err(_e) = await!(self.transport.send(msg)) {
                        //error!("Error sending ack: {}", e);
                    }
                }

                Msg::Ack(seq, broadcasts) => {
                    let acks = self.inner.acks();
                    let mut acks = await!(acks.lock());
                    acks.handle_ack(seq, self.inner.clone());
                    await!(self.inner.apply_broadcasts(broadcasts));
                }
                _ => unimplemented!(),
            };
        }
    }

    async fn gossip(&self) {
        use tokio::prelude::StreamAsyncExt;
        let mut interval = Interval::new_interval(Duration::from_secs(1));

        while let Some(_) = await!(interval.next()) {
            trace!("Sending heartbeats");

            let broadcasts = await!(self.inner.broadcasts().lock()).get();

            if let Some(peer) = await!(self.inner.get_peer()) {
                debug!("Probing peer: {}", peer);

                let mut acks = await!(self.inner.acks().lock());

                let seq_num = acks.add(peer.addr(), PingHandler);
                let msg = (Msg::Ping(seq_num, broadcasts.clone()), peer.addr());

                if let Err(_e) = await!(self.transport.send(msg)) {
                    //error!("Error sending ping: {}", e);
                }
            }
        }
    }
}
