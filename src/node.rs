use futures::{
    sync::mpsc::{self, Sender},
    Future, Sink, Stream,
};
use log::{debug, error, info};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::{UdpFramed, UdpSocket},
    timer::Interval,
};

use codec::{Msg, MsgCodec};

pub struct Node {
    addr: SocketAddr,
    peers: Arc<Mutex<HashMap<SocketAddr, u8>>>,
}

impl Node {
    pub fn new(addr: SocketAddr) -> Self {
        Node {
            addr,
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn serve(&mut self, peers: Vec<SocketAddr>) -> impl Future<Item = (), Error = ()> {
        let (tx, rx) = mpsc::channel(1000);

        let socket = UdpSocket::bind(&self.addr).expect("Unable to bind socket for serve");

        info!("Listening on: {}", &self.addr);

        let (sink, stream) = UdpFramed::new(socket, MsgCodec).split();

        let sink = sink
            .send_all(
                rx.map(|(msg, addr)| {
                    debug!("Sending: {:?} to: {:?}", msg, addr);
                    (msg, addr)
                }).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::Other, "rx shouldn't have an error")
                }),
            ).map(|_| ())
            .map_err(|e| println!("Error: {:?}", e));

        let tx_2 = tx.clone();
        let peers_clone = self.peers.clone();
        let stream = stream
            .for_each(move |(msg, addr)| {
                debug!("Got Message: {:?} from: {:?}", msg, addr);

                match msg {
                    Msg::Ping => {
                        let tx = tx_2.clone();
                        tokio::spawn(tx.send((Msg::Ack, addr)).map(|_| ()).map_err(|_| ()));
                    }

                    Msg::Ack => {
                        let mut peers = peers_clone.lock().expect("unable to acquire lock");

                        if let None = peers.get(&addr) {
                            info!("Added peer: {} with {}", 1, addr);
                            peers.insert(addr, 1);
                        }
                    }
                }

                Ok(())
            }).map_err(|e| error!("Error: {:?}", e));

        let messages = peers
            .iter()
            .map(|addr| (Msg::Ping, *addr))
            .map(|msg| {
                let tx = tx.clone();
                tx.send(msg)
            }).collect::<Vec<_>>();

        let connect = futures::future::join_all(messages)
            .map(|_| ())
            .map_err(|_| ());

        let heartbeats = self.heartbeats(tx.clone());

        connect
            .join4(sink, stream, heartbeats)
            .map(|_| ())
            .map_err(|_| ())
    }

    fn heartbeats(&self, tx: Sender<(Msg, SocketAddr)>) -> impl Future<Item = (), Error = ()> {
        let peers = self.peers.clone();
        Interval::new_interval(Duration::from_secs(3))
            .for_each(move |_| {
                debug!("sending heartbeat");
                let peers = peers.lock().expect("unable to acquire lock");

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
