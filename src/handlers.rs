use {
    crate::{acks::Handler, state::State},
    futures::future::{ready, Ready},
    std::sync::Arc,
};

pub struct PingHandler;

impl Handler<()> for PingHandler {
    type Future = Ready<()>;

    fn call(&mut self, _state: Arc<State>) -> Self::Future {
        ready(())
    }
}
