use {
    crate::{
        rpc::proto::{server, Peer, Pull, Push},
        state::State,
    },
    log::{error, info, trace},
    std::{net::SocketAddr, sync::Arc},
    tokio::{
        executor::DefaultExecutor,
        net::TcpListener,
        prelude::{future, Future, Stream},
    },
    tower_grpc::{Request, Response},
    tower_h2::Server,
    uuid::Uuid,
};

#[derive(Debug, Clone)]
pub struct MemberServer {
    addr: SocketAddr,
    inner: Arc<State>,
}

impl MemberServer {
    pub fn new(addr: SocketAddr, inner: Arc<State>) -> Self {
        MemberServer { addr, inner }
    }

    pub fn serve(
        state: Arc<State>,
        addr: &SocketAddr,
    ) -> impl Future<Item = (), Error = std::io::Error> {
        let new_service = server::MemberServer::new(MemberServer::new(addr.clone(), state));
        let mut h2 = Server::new(new_service, Default::default(), DefaultExecutor::current());

        TcpListener::bind(&addr)
            .expect("Unable to bind tcp")
            .incoming()
            .for_each(move |sock| {
                let fut = h2.serve(sock).map_err(|err| error!("h2 error: {:?}", err));
                tokio::spawn(fut);
                Ok(())
            })
    }
}

impl server::Member for MemberServer {
    type JoinFuture = future::FutureResult<Response<Pull>, tower_grpc::Error>;

    fn join(&mut self, request: Request<Push>) -> Self::JoinFuture {
        info!("Join Request: {:?}", request);

        let from = request.into_inner().from.unwrap();

        let from_id = Uuid::parse_str(from.id.as_str()).unwrap();
        let from_addr = from.address.parse().unwrap();
        self.inner.peer_join(from_id, from_addr);

        let peers = self
            .inner
            .peers()
            .iter()
            .map(|(id, peer)| Peer {
                id: id.to_string(),
                address: peer.addr().to_string(),
            })
            .collect();

        trace!("Pushing these peers: {:?}", peers);

        let current_peer = Peer {
            id: self.inner.id().to_string(),
            address: self.addr.to_string(),
        };

        future::ok(Response::new(Pull {
            from: Some(current_peer),
            peers,
        }))
    }
}

#[cfg(test)]
mod test {
    use {
        super::MemberServer,
        crate::{
            rpc::proto::{server::Member, Peer, Push},
            state::State,
        },
        std::sync::Arc,
        tokio::prelude::Future,
        tower_grpc::Request,
        uuid::Uuid,
    };

    #[test]
    fn join() {
        let addr = "127.0.0.1:1234".to_string();
        let state = Arc::new(State::new());
        let mut server = MemberServer::new(addr.parse().unwrap(), state);

        let from = Peer {
            id: Uuid::new_v4().to_string(),
            address: addr,
        };

        let request = Request::new(Push {
            from: Some(from.clone()),
            peers: vec![from.clone()],
        });

        let response = server.join(request).wait().unwrap();
        assert_eq!(response.into_inner().peers, vec![from]);
    }
}
