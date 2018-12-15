use {
    crate::{
        error::Result,
        rpc::proto::{server, Peer, Pull, Push},
        state::State,
    },
    futures::future::TryFutureExt,
    log::{error, info, trace},
    pin_utils::unsafe_pinned,
    std::{
        future::Future,
        marker::Unpin,
        net::SocketAddr,
        pin::Pin,
        sync::Arc,
        task::{LocalWaker, Poll},
    },
    tokio::{
        executor::DefaultExecutor,
        net::TcpListener,
        prelude::{Future as Future01, Stream},
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
    ) -> impl Future01<Item = (), Error = std::io::Error> {
        let new_service = server::MemberServer::new(MemberServer::new(*addr, state));
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

    pub async fn join(
        request: Request<Push>,
        addr: SocketAddr,
        inner: Arc<State>,
    ) -> Result<Response<Pull>> {
        // let inner = self.inner.clone();
        // let addr = self.addr;

        let from = request.into_inner().from.unwrap();

        info!("Join Request: {}:{}", from.id, from.address);

        let from_id = Uuid::parse_str(from.id.as_str())?;
        let from_addr = from.address.parse().unwrap();

        let peers = {
            let peers = await!(inner.peers().lock());

            peers
                .iter()
                .map(|(addr, peer)| Peer {
                    id: peer.id().to_string(),
                    address: addr.to_string(),
                })
                .collect()
        };

        trace!("Pushing these peers: {:?}", peers);

        await!(inner.peer_join(from_id, from_addr));

        let current_peer = Peer {
            id: await!(inner.id().lock()).to_string(),
            address: addr.to_string(),
        };

        Ok(Response::new(Pull {
            from: Some(current_peer),
            peers,
        }))
    }
}

impl server::Member for MemberServer {
    type JoinFuture =
        Box<dyn Future01<Item = Response<Pull>, Error = tower_grpc::Error> + Send + 'static>;

    fn join(&mut self, request: Request<Push>) -> Self::JoinFuture {
        let inner = self.inner.clone();
        let addr = self.addr;
        let fut = Self::join(request, addr, inner);

        let fut = Box::pinned(fut);
        let fut = TowerError::new(fut);
        Box::new(fut.compat())
    }
}

/// Future for the `unit_error` combinator, turning a `Future` into a `TryFuture`.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct TowerError<Fut> {
    future: Fut,
}

impl<Fut> TowerError<Fut> {
    unsafe_pinned!(future: Fut);

    /// Creates a new UnitError.
    pub(super) fn new(future: Fut) -> TowerError<Fut> {
        TowerError { future }
    }
}

impl<Fut: Unpin> Unpin for TowerError<Fut> {}
use std::result::Result as StdResult;
impl<Fut, T, E> Future for TowerError<Fut>
where
    E: std::fmt::Display,
    Fut: Future<Output = StdResult<T, E>>,
{
    type Output = StdResult<T, tower_grpc::Error>;

    fn poll(mut self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        match self.future().poll(lw) {
            Poll::Ready(Ok(i)) => Poll::Ready(Ok(i)),
            Poll::Ready(Err(e)) => {
                error!("RPC Error: {}", e);
                Poll::Ready(Err(tower_grpc::Error::Inner(())))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::MemberServer,
        crate::state::State,
        clique_proto::{server::Member, Peer, Push},
        futures::compat::Future01CompatExt,
        std::sync::Arc,
        tokio_async_await_test::async_current_thread_test,
        tower_grpc::Request,
        uuid::Uuid,
    };

    #[async_current_thread_test]
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
        //assert_eq!(response.into_inner().peers, vec![from]);
        assert!(response.into_inner().peers.is_empty());
    }
}
