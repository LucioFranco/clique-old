use futures::{
    sync::mpsc::{self, Receiver, Sender},
    Future, Sink, Stream,
};
use log::{debug, error, info};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    net::{UdpFramed, UdpSocket},
    timer::Interval,
};

use client::join;
use codec::{Msg, MsgCodec};
use state::State;

pub struct Node {
    addr: SocketAddr,
    inner: Arc<RwLock<State>>,
}

impl Node {
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            addr,
            inner: Arc::new(RwLock::new(State::new())),
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

        join(&addr)
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
        use rpc::MemberServer;

        let inner = self.inner.clone();
        MemberServer::serve(inner, addr)
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

    fn process_message(
        inner: Arc<RwLock<State>>,
        tx: Sender<(Msg, SocketAddr)>,
        msg: Msg,
        addr: SocketAddr,
    ) {
        match msg {
            Msg::Ping => {
                tokio::spawn(tx.send((Msg::Ack, addr)).map(|_| ()).map_err(|_| ()));
            }

            Msg::Ack => {
                let inner = inner.read().unwrap();
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
                let inner = inner.read().unwrap();
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
