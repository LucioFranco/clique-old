use futures::{
    sync::mpsc::{self, Receiver, Sender},
    Future, Sink, Stream,
};
use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};
use tokio::{
    codec::Decoder,
    net::{TcpListener, TcpStream, UdpFramed, UdpSocket},
    timer::Interval,
};

use codec::{Join, JoinCodec, Msg, MsgCodec};
use state::State;

pub struct Node {
    addr: SocketAddr,
    inner: Arc<State>,
}

impl Node {
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            addr,
            inner: Arc::new(State::default()),
        }
    }

    pub fn join<A>(&self, peers: A) -> impl Future<Item = (), Error = ()>
    where
        A: ToSocketAddrs,
    {
        let addr = peers
            .to_socket_addrs()
            .unwrap()
            .next()
            .expect("One addrs required");

        info!("Sending Cluster Join Request to: {}", addr);

        let inner = self.inner.clone();
        let id = inner.id.clone();
        TcpStream::connect(&addr)
            .and_then(move |stream| {
                let (writer, reader) = JoinCodec.framed(stream).split();

                writer
                    .send(Join::Request(id.to_string()))
                    .map(|writer| (reader, writer))
            }).map(move |(reader, _writer)| {
                let inner = inner.clone();

                let response = reader
                    .map(move |message| {
                        info!("Received Join response message: {:?}", message);

                        match message {
                            Join::Peers(incoming_peers) => {
                                let peers = inner.peers.clone();
                                let mut peers =
                                    peers.write().expect("Unable to acquire peers lock");

                                let peers_encoded = incoming_peers
                                    .iter()
                                    .map(|(k, v)| {
                                        let id = uuid::Uuid::parse_str(v.as_str()).unwrap();
                                        (k.clone(), id)
                                    }).collect::<HashMap<SocketAddr, uuid::Uuid>>();

                                peers.clone_from(&peers_encoded);
                                info!("Got Peers!: {:?}", peers);
                            }
                            m => warn!("Received unexpected message: {:?}", m),
                        };
                    }).into_future()
                    .map(|_| ());

                tokio::spawn(response.map_err(|_| ()))
            }).map(|_| ())
            .map_err(|e| error!("Error attempting to join cluster: {:?}", e))
    }

    pub fn serve(&mut self) -> impl Future<Item = (), Error = ()> {
        let (tx, rx) = mpsc::channel(1000);
        let socket = UdpSocket::bind(&self.addr).expect("Unable to bind socket for serve");

        let addr = socket.local_addr().unwrap();

        let udp_listener = self.listen_udp(socket, (tx.clone(), rx));
        let tcp_listener = self.listen_tcp(&addr);

        let gossiper = self.gossip(tx.clone());

        tcp_listener.join3(udp_listener, gossiper).map(|_| ())
    }

    fn listen_tcp(&self, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        let socket = TcpListener::bind(addr).expect("Unable to bind tcp socket for server");
        let inner = self.inner.clone();

        socket
            .incoming()
            .for_each(move |stream| {
                let remote_addr = stream.peer_addr().expect("Unable to get remote peer addr");
                info!("Accepted new TCP Connection from: {}", remote_addr);
                let (writer, reader) = JoinCodec.framed(stream).split();

                let (tx, rx) = mpsc::channel(1);

                let inner = inner.clone();
                let response = reader
                    .map(move |message| {
                        let inner = inner.clone();
                        let response = Node::process_rpc(inner, (message, remote_addr));

                        let tx = tx.clone();
                        tokio::spawn(tx.send(response).map(|_| ()).map_err(|_| ()));
                    }).into_future();

                tokio::spawn(response.map(|_| ()).map_err(|_| ()));

                let msg = writer
                    .send_all(
                        rx.map(|message| {
                            info!("Sending: {:?}", message);
                            message
                        }).map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "rx shouldn't have an error",
                            )
                        }),
                    ).then(|res| {
                        if let Err(e) = res {
                            println!("failed to process connection; error = {:?}", e);
                        }

                        Ok(())
                    });

                tokio::spawn(msg);

                Ok(())
            }).map_err(|_| ())
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
                debug!("Sending: {:?} to: {:?}", msg, addr);
                (msg, addr)
            }).map_err(|_| {
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

    fn process_rpc(inner: Arc<State>, request: (Join, SocketAddr)) -> Join {
        let peers = inner.peers.clone();

        let (request, addr) = request;
        match request {
            Join::Request(id) => {
                info!("Cluster Join Request from: {}", id);
                let mut peers = peers.write().expect("Unable to acquire peer lock");
                let id = uuid::Uuid::parse_str(id.as_str()).unwrap();
                peers.insert(addr, id);

                Join::Peers(
                    peers
                        .clone()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect::<HashMap<SocketAddr, String>>(),
                )
            }
            _ => Join::Done,
        }
    }

    fn process_message(
        inner: Arc<State>,
        tx: Sender<(Msg, SocketAddr)>,
        msg: Msg,
        addr: SocketAddr,
    ) {
        match msg {
            Msg::Ping => {
                tokio::spawn(tx.send((Msg::Ack, addr)).map(|_| ()).map_err(|_| ()));
            }

            Msg::Ack => {
                let mut peers = inner.peers.write().expect("unable to acquire lock");

                if let None = peers.get(&addr) {
                    let new_id = uuid::Uuid::new_v4();
                    info!("Added peer: {} with {}", new_id, addr);
                    peers.insert(addr, new_id);
                }
            }
        }
    }

    fn gossip(&self, tx: Sender<(Msg, SocketAddr)>) -> impl Future<Item = (), Error = ()> {
        let inner = self.inner.clone();
        Interval::new_interval(Duration::from_secs(3))
            .for_each(move |_| {
                debug!("sending heartbeat");
                let peers = inner.peers.read().expect("unable to acquire lock");

                let heartbeats = peers
                    .iter()
                    .map(|(addr, _id)| (Msg::Ping, *addr))
                    .map(|msg| {
                        let tx = tx.clone();
                        tx.send(msg)
                    }).collect::<Vec<_>>();

                tokio::spawn(
                    futures::future::join_all(heartbeats)
                        .map(|_| ())
                        .map_err(|_| ()),
                );

                Ok(())
            }).map_err(|_| ())
    }
}
