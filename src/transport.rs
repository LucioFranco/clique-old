use {
    crate::{codec::Msg, task::spawn_with_handle},
    std::{future::Future, marker::Unpin},
};

pub trait MakeTransport {
    type Transport: Transport;

    fn make();
}

pub trait Transport {
    type Error: std::fmt::Debug;

    type SendFuture: Future<Output = Result<(), Self::Error>>;

    fn send(&self, message: (Msg, SocketAddr)) -> Self::SendFuture;

    type ServeFuture: Future<Output = ()> + Send + 'static;

    fn serve(addr: SocketAddr) -> (Self::ServeFuture, Self);

    fn recv(&self) -> Receiver<(Msg, SocketAddr)>;

    fn local_addr(&self) -> SocketAddr;
}

use crate::codec::MsgCodec;
use futures::channel::mpsc::{channel, Receiver, SendError, Sender};
use futures::compat::{Sink01CompatExt, Stream01CompatExt};
use futures::future::FutureObj;
use futures::future::RemoteHandle;
use futures::{join, SinkExt, StreamExt, TryStreamExt};
use log::info;
use std::net::SocketAddr;
use std::sync::Mutex;
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::Stream as Stream01;

pub struct Net {
    addr: SocketAddr,
    sender: Sender<(Msg, SocketAddr)>,
    receiver: Mutex<Option<Receiver<(Msg, SocketAddr)>>>,
}

impl Unpin for Net {}
impl Transport for Net {
    type Error = NetError;

    type SendFuture = FutureObj<'static, Result<(), Self::Error>>;

    fn send(&self, message: (Msg, SocketAddr)) -> Self::SendFuture {
        let mut sender = self.sender.clone();

        FutureObj::new(Box::new(
            async move { await!(sender.send(message)).map_err(NetError::from) },
        ))
    }

    fn recv(&self) -> Receiver<(Msg, SocketAddr)> {
        let mut lock = self.receiver.lock().unwrap();
        let recv = lock.take().unwrap();
        drop(lock);
        recv
    }

    type ServeFuture = RemoteHandle<()>;

    fn serve(addr: SocketAddr) -> (Self::ServeFuture, Self) {
        let socket = UdpSocket::bind(&addr).unwrap();

        let addr = socket.local_addr().unwrap();

        info!("Listening on: {}", addr);

        let framed = UdpFramed::new(socket, MsgCodec);

        let (mut sink, stream) = {
            let (sink, stream) = framed.split();
            let sink = sink.compat_sink().sink_err_into::<NetError>();
            let stream = stream.compat().err_into::<NetError>();

            (sink, stream)
        };

        let (mut recv_tx, recv_rx) = {
            let (tx, rx) = channel(1000);
            (tx.sink_err_into::<NetError>(), rx)
        };
        let (sender_tx, mut sender_rx) = channel(1000);

        let transport = Net {
            addr,
            sender: sender_tx.clone(),
            receiver: Mutex::new(Some(recv_rx)),
        };

        let fut = async move {
            let recv = stream.forward(&mut recv_tx);
            let sender = sink.send_all(&mut sender_rx);

            join!(recv, sender);
        };

        let handle = spawn_with_handle(fut).unwrap();

        (handle, transport)
    }

    fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

#[derive(Debug)]
pub enum NetError {
    Channel(SendError),
    Bincode(bincode::Error),
    Io(std::io::Error),
}

impl From<SendError> for NetError {
    fn from(e: SendError) -> NetError {
        NetError::Channel(e)
    }
}

impl From<bincode::Error> for NetError {
    fn from(e: bincode::Error) -> NetError {
        NetError::Bincode(e)
    }
}

impl From<std::io::Error> for NetError {
    fn from(err: std::io::Error) -> Self {
        NetError::Io(err)
    }
}
