use futures::{
    sync::mpsc::{self, Receiver, Sender},
    Future, Sink, Stream,
};
use log::{error, info, trace};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    executor::DefaultExecutor,
    net::{TcpStream, UdpFramed, UdpSocket},
    timer::Interval,
};
use tower_grpc::Request;
use tower_h2::client::Connection;
use uuid::Uuid;

use crate::codec::{Msg, MsgCodec};
use crate::rpc::{
    proto::{client::Member, Peer, Push},
    MemberServer,
};
use crate::state::{NodeState, State};

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

        let inner = self.inner.clone();
        let from_address = local_addr.clone();

        let id = inner.id().to_string();
        let connect = TcpStream::connect(&join_addr)
            .and_then(move |socket| {
                // Bind the HTTP/2.0 connection
                Connection::handshake(socket, DefaultExecutor::current())
                    .map_err(|_| panic!("failed HTTP/2.0 handshake"))
            })
            .map(move |conn| {
                use tower_http::add_origin;

                let conn = add_origin::Builder::new().uri(uri).build(conn).unwrap();

                Member::new(conn)
            })
            .and_then(move |mut client| {
                let from = Peer {
                    id: id.to_string(),
                    address: from_address.to_string(),
                };

                client
                    .join(Request::new(Push {
                        from: Some(from.clone()),
                        peers: vec![from],
                    }))
                    .map_err(|e| panic!("gRPC request failed; err={:?}", e))
            })
            .and_then(move |response| {
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
                inner.peers_sync(peers);
                inner.update_state(NodeState::Connected);

                let from = body.from.unwrap();
                info!(
                    "Connected to clique cluster via {}:{}",
                    from.id, from.address
                );

                Ok(())
            })
            .map_err(|e| {
                error!("Error encountered during join rpc: {:?}", e);
            });

        await!(connect)?;

        Ok(())
    }

    pub async fn serve(&self) -> Result<(), ()> {
        let (tx, rx) = mpsc::channel(1000);
        let socket = UdpSocket::bind(&self.addr).expect("Unable to bind socket for serve");

        let addr = socket.local_addr().unwrap();

        let udp_listener = self.listen_udp(socket, (tx.clone(), rx));
        let tcp_listener = self.listen_tcp(&addr);

        let gossiper = self.gossip(tx.clone());

        let serve = tcp_listener.join3(udp_listener, gossiper).map(|_| ());

        await!(serve)?;

        Ok(())
    }

    fn listen_tcp(&self, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        MemberServer::serve(self.inner.clone(), addr)
    }

    fn listen_udp(
        &self,
        socket: UdpSocket,
        channel: (Sender<(Msg, SocketAddr)>, Receiver<(Msg, SocketAddr)>),
    ) -> impl Future<Item = (), Error = ()> {
        let (tx, rx) = channel;

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

        stream
            .join(sink)
            .map(|_| ())
            .map_err(|e| error!("Error with UDP: {:?}", e))
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
