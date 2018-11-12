#[allow(dead_code)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/clique.proto.rs"));
}

use self::proto::{server, JoinRequest, Peer};
//use futures::{future, Future, Stream};
use futures::*;
use log::error;
use state::State;
use std::{net::SocketAddr, sync::Arc};
use tokio::executor::DefaultExecutor;
use tokio::net::{TcpListener, TcpStream};
use tower_grpc::{Request, Response};
use tower_h2::{client::Connection, Server};

#[derive(Debug, Clone)]
pub struct MemberServer {
    inner: Arc<State>,
}

impl MemberServer {
    pub fn new(state: Arc<State>) -> Self {
        MemberServer { inner: state }
    }

    pub fn serve(state: Arc<State>, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        let h2 = Server::new(
            server::MemberServer::new(MemberServer::new(state)),
            Default::default(),
            DefaultExecutor::current(),
        );

        let bind = TcpListener::bind(&addr).unwrap();
        let fut = bind
            .incoming()
            .for_each(move |sock| {
                let fut = h2.serve(sock).map_err(|err| error!("h2 error: {:?}", err));
                tokio::spawn(fut);
                Ok(())
            }).map_err(|err| error!("server error: {:?}", err));

        fut
    }

    // pub fn connect(addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
    //     let uri: http::Uri = format!("http://localhost:50051").parse().unwrap();

    //     TcpStream::connect(&addr)
    //         .and_then(move |socket| {
    //             // Bind the HTTP/2.0 connection
    //             Connection::handshake(socket, DefaultExecutor::current())
    //                 .map_err(|_| panic!("failed HTTP/2.0 handshake"))
    //         }).map(move |conn| {
    //             use self::proto::client::Member;
    //             use tower_http::add_origin;

    //             let conn = add_origin::Builder::new().uri(uri).build(conn).unwrap();

    //             Member::new(conn)
    //         })
    //         .and_then(|response| {
    //             //println!("RESPONSE = {:?}", response);
    //             Ok(())
    //         }).map_err(|e| {
    //             println!("ERR = {:?}", e);
    //         })
    // }
}

impl server::Member for MemberServer {
    type JoinFuture = future::FutureResult<Response<Peer>, tower_grpc::Error>;

    fn join(&mut self, request: Request<JoinRequest>) -> Self::JoinFuture {
        let state = self.inner.clone();
        unimplemented!()
    }
}
