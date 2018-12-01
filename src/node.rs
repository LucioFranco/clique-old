use futures::{
    sync::mpsc::{self, Sender},
    Future, Sink, Stream,
};
use futures_util::join;
use log::{error, info, trace};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{UdpFramed, UdpSocket},
    timer::Interval,
};
use tower_grpc::Request;
use uuid::Uuid;

use crate::{
    client,
    codec::{Msg, MsgCodec},
    rpc::{
        proto::{Peer, Push},
        MemberServer,
    },
    state::{NodeState, State},
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

    pub async fn serve(&self) -> Result<(), ()> {
        let socket = UdpSocket::bind(&self.addr).expect("Unable to bind socket for serve");

        let addr = socket.local_addr().unwrap();

        let (tx, udp_listener) = self.listen_udp(socket);

        let udp_listener = async {
            await!(udp_listener).expect("Error gossiping");
        };

        let tcp_listener = self.listen_tcp(addr.clone());

        let gossiper = self.gossip(tx);
        let gossiper = async {
            await!(gossiper).expect("Error gossiping");
        };

        join!(tcp_listener, udp_listener, gossiper);

        Ok(())
    }

    async fn listen_tcp(&self, addr: SocketAddr) {
        let inner = self.inner.clone();
        await!(MemberServer::serve(inner, &addr)).expect("Error listening for rpc");
    }

    fn listen_udp(
        &self,
        socket: UdpSocket,
    ) -> (
        Sender<(Msg, SocketAddr)>,
        impl Future<Item = (), Error = ()>,
    ) {
        let (tx, rx) = mpsc::channel(1000);
        let tx2 = tx.clone();

        info!("Listening on: {}", self.addr);

        let (sink, stream) = UdpFramed::new(socket, MsgCodec).split();

        let rx = rx
            .map(|(msg, addr)| {
                trace!("Sending: {:?} to: {:?}", msg, addr);
                (msg, addr)
            })
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "rx shouldn't have an error")
            });

        let sink = sink.send_all(rx);

        let inner = self.inner.clone();
        let stream = stream.for_each(move |(msg, addr)| {
            Node::process_message(inner.clone(), tx.clone(), msg, addr);

            Ok(())
        });

        let fut = stream
            .join(sink)
            .map(|_| ())
            .map_err(|e| error!("Error with UDP: {:?}", e));

        (tx2, fut)
    }

    fn process_message(
        state: Arc<State>,
        tx: Sender<(Msg, SocketAddr)>,
        msg: Msg,
        addr: SocketAddr,
    ) {
        match msg {
            Msg::Ping(broadcasts) => {
                state.apply_broadcasts(broadcasts);

                let ack = {
                    let broadcasts = state.broadcasts_mut().drain();
                    Msg::Ack(broadcasts)
                };

                tokio::spawn(tx.send((ack, addr)).map(|_| ()).map_err(|_| ()));
            }

            Msg::Ack(broadcasts) => {
                state.apply_broadcasts(broadcasts);
            }
        }
    }

    fn gossip(&self, tx: Sender<(Msg, SocketAddr)>) -> impl Future<Item = (), Error = ()> {
        let inner = self.inner.clone();
        // TODO: Set the interval from config
        Interval::new_interval(Duration::from_secs(1))
            .for_each(move |_| {
                let peers = inner.peers();

                trace!("Sending heartbeats");

                let broadcasts = inner.broadcasts_mut().drain();

                let heartbeats = peers
                    .iter()
                    .map(move |(_id, peer)| {
                        let ping = Msg::Ping(broadcasts.clone());
                        (ping, peer.addr())
                    })
                    .map(|msg| {
                        let tx = tx.clone();
                        tx.send(msg)
                    })
                    .collect::<Vec<_>>();

                tokio::spawn(
                    futures::future::join_all(heartbeats)
                        .map(|_| ())
                        .map_err(|_| ()),
                );

                Ok(())
            })
            .map_err(|_| ())
    }
}
