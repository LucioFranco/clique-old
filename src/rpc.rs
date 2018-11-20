#[allow(dead_code)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/clique.proto.rs"));
}

pub use self::proto::{client, server, JoinRequest, Peer};
//use futures::{future, Future, Stream};
use futures::*;
use log::error;
use state::NodeState;
use state::State;
use std::{net::SocketAddr, sync::Arc};
use tokio::executor::DefaultExecutor;
use tokio::net::TcpListener;
use tower_grpc::{Request, Response};
use tower_h2::Server;

#[derive(Debug, Clone)]
pub struct MemberServer {
    inner: Arc<State>,
}

impl MemberServer {
    pub fn new(state: Arc<State>) -> Self {
        MemberServer { inner: state }
    }

    pub fn serve(state: Arc<State>, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        let new_service = server::MemberServer::new(MemberServer::new(state));
        let mut h2 = Server::new(new_service, Default::default(), DefaultExecutor::current());

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
}

impl server::Member for MemberServer {
    type JoinFuture = future::FutureResult<Response<Peer>, tower_grpc::Error>;

    fn join(&mut self, request: Request<JoinRequest>) -> Self::JoinFuture {
        println!("{:?}", request);

        self.inner.update_state(NodeState::Connected);

        let peers = self
            .inner
            .peers()
            .iter()
            .map(|(k, _)| k.to_string())
            .collect();

        futures::future::ok(Response::new(Peer { peers: peers }))
    }
}
