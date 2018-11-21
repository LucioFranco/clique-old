#[allow(dead_code)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/clique.proto.rs"));
}

pub use self::proto::{client, server, Peer, Pull, Push};
use futures::{future, Future, Stream};
use log::{error, trace};
use state::State;
use std::{net::SocketAddr, sync::Arc};
use tokio::executor::DefaultExecutor;
use tokio::net::TcpListener;
use tower_grpc::{Request, Response};
use tower_h2::Server;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MemberServer {
    addr: SocketAddr,
    inner: Arc<State>,
}

impl MemberServer {
    pub fn new(addr: SocketAddr, inner: Arc<State>) -> Self {
        MemberServer { addr, inner }
    }

    pub fn serve(state: Arc<State>, addr: &SocketAddr) -> impl Future<Item = (), Error = ()> {
        let new_service = server::MemberServer::new(MemberServer::new(addr.clone(), state));
        let mut h2 = Server::new(new_service, Default::default(), DefaultExecutor::current());

        TcpListener::bind(&addr)
            .expect("Unable to bind tcp")
            .incoming()
            .for_each(move |sock| {
                let fut = h2.serve(sock).map_err(|err| error!("h2 error: {:?}", err));
                tokio::spawn(fut);
                Ok(())
            }).map_err(|err| error!("server error: {:?}", err))
    }
}

impl server::Member for MemberServer {
    type JoinFuture = future::FutureResult<Response<Pull>, tower_grpc::Error>;

    fn join(&mut self, request: Request<Push>) -> Self::JoinFuture {
        trace!("Join Request: {:?}", request);

        let from = request.into_inner().from.unwrap();

        let from_id = Uuid::parse_str(from.id.as_str()).unwrap();
        let from_addr = from.address.parse().unwrap();
        self.inner.insert_peer(from_id, from_addr);

        let peers = self
            .inner
            .peers()
            .iter()
            .map(|(id, addr)| Peer {
                id: id.to_string(),
                address: addr.to_string(),
            }).collect();

        trace!("Pushing these peers: {:?}", peers);

        let current_peer = Peer {
            id: self.inner.id().to_string(),
            address: self.addr.to_string(),
        };

        futures::future::ok(Response::new(Pull {
            from: Some(current_peer),
            peers,
        }))
    }
}
