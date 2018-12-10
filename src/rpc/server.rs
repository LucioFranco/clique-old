use {
    crate::state::State,
    clique_proto::{server, Peer, Pull, Push},
    futures::future::{FutureExt, TryFutureExt},
    log::{error, info, trace},
    std::{net::SocketAddr, sync::Arc},
    tokio::{
        executor::DefaultExecutor,
        net::TcpListener,
        prelude::{Future, Stream},
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
    type JoinFuture =
        Box<dyn Future<Item = Response<Pull>, Error = tower_grpc::Error> + Send + 'static>;

    fn join(&mut self, request: Request<Push>) -> Self::JoinFuture {
        let inner = self.inner.clone();
        let addr = self.addr.clone();

        let fut = async move { await!(join(inner, request, addr)).unwrap() };

        Box::new(
            fut.unit_error()
                .boxed()
                .compat()
                .map_err(|e| tower_grpc::Error::Inner(e)),
        )
    }
}

pub async fn join(
    inner: Arc<State>,
    request: Request<Push>,
    addr: SocketAddr,
) -> Result<Response<Pull>, ()> {
    let from = request.into_inner().from.unwrap();

    info!("Join Request: {}:{}", from.id, from.address);

    let from_id = Uuid::parse_str(from.id.as_str()).unwrap();
    let from_addr = from.address.parse().unwrap();

    await!(inner.peer_join(from_id, from_addr));

    let peers = {
        let peers = await!(inner.peers().lock());

        peers
            .iter()
            .map(|(id, peer)| Peer {
                id: id.to_string(),
                address: peer.addr().to_string(),
            })
            .collect()
    };

    trace!("Pushing these peers: {:?}", peers);

    let current_peer = Peer {
        id: await!(inner.id().lock()).to_string(),
        address: addr.to_string(),
    };

    Ok(Response::new(Pull {
        from: Some(current_peer),
        peers,
    }))
}

#[cfg(test)]
mod test {
    use {
        super::MemberServer,
        crate::state::State,
        clique_proto::{server::Member, Peer, Push},
        futures::compat::Future01CompatExt,
        std::sync::Arc,
        tokio_async_await_test::async_test,
        tower_grpc::Request,
        uuid::Uuid,
    };

    #[async_test]
    async fn join() {
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

        let response = await!(server.join(request).compat()).unwrap();
        assert_eq!(response.into_inner().peers, vec![from]);
    }
}
