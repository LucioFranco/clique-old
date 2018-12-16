use {
    crate::{state::State, task::spawn},
    futures::channel::oneshot::{self, Receiver, Sender},
    std::{
        collections::HashMap,
        future::Future,
        net::SocketAddr,
        sync::Arc,
        time::{Duration, Instant},
    },
};

pub type SeqNum = u32;

#[derive(Debug)]
pub struct Acks {
    acks: HashMap<SeqNum, AckHandler>,
    timeout: Duration,
    seq_num: SeqNum,
    expired: Vec<SeqNum>,
}

impl Acks {
    pub fn new(timeout: Duration) -> Acks {
        Acks {
            timeout,
            seq_num: 0,
            acks: HashMap::new(),
            expired: Vec::new(),
        }
    }

    pub fn add<F, T: 'static>(&mut self, target: SocketAddr, f: F) -> SeqNum
    where
        F: Handler<T> + Send + Sync + 'static,
    {
        let seq_num = self.seq_num;
        self.next_seq();
        self.acks.insert(seq_num, AckHandler::new(target, f));
        seq_num
    }

    pub fn handle_ack(&mut self, seq: SeqNum, state: Arc<State>) {
        // TODO: check that returning socketaddr matches
        if let Some(handler) = self.acks.remove(&seq) {
            handler.handle_ack(state.clone());
        }
    }

    pub fn next_seq(&mut self) -> SeqNum {
        let new_seq_num = self.seq_num.wrapping_add(1);
        self.seq_num = new_seq_num;
        new_seq_num
    }
}

#[derive(Debug)]
pub struct AckHandler {
    sent_at: Instant,
    addr: SocketAddr,
    // The signal that we got this ack
    signal: Sender<Arc<State>>,
}

impl AckHandler {
    pub fn new<F, T: 'static>(target: SocketAddr, f: F) -> AckHandler
    where
        F: Handler<T> + Send + Sync + 'static,
    {
        let (signal, rx) = oneshot::channel();

        spawn(Self::handle_signal(rx, f));

        AckHandler {
            addr: target,
            sent_at: Instant::now(),
            signal,
        }
    }

    async fn handle_signal<F, T: 'static>(rx: Receiver<Arc<State>>, mut fut: F)
    where
        F: Handler<T> + Send + Sync + 'static,
    {
        let state = await!(rx).unwrap();
        await!(fut.call(state));
    }

    pub fn handle_ack(self, state: Arc<State>) {
        self.signal.send(state).unwrap();
    }
}

pub trait Handler<T> {
    type Future: Future<Output = T> + Send + 'static;

    fn call(&mut self, state: Arc<State>) -> Self::Future;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        state::State,
        support::{next_addr, sleep_ms},
    };
    use futures::compat::Future01CompatExt;
    use futures::{
        channel::oneshot::{self, Sender},
        future::{self, FutureObj},
    };
    use std::{sync::Arc, time::Duration};
    use tokio::timer::Timeout;

    #[async_current_thread_test]
    async fn ack() {
        let mut acks = Acks::new(Duration::from_secs(1));

        let target = next_addr();

        let (tx, rx) = oneshot::channel();

        acks.add(target, Mock(Some(tx)));

        let state = Arc::new(State::new());

        await!(sleep_ms(200));

        acks.handle_ack(0, state);

        await!(sleep_ms(200));

        let timeout = Timeout::new(rx.unit_error().boxed().compat(), Duration::from_secs(100));

        let result = await!(timeout.compat()).unwrap();
        assert!(result.is_ok());
    }

    #[async_current_thread_test]
    async fn no_ack() {
        let mut acks = Acks::new(Duration::from_secs(1));

        let target = next_addr();

        let (tx, rx) = oneshot::channel();

        acks.add(target, Mock(Some(tx)));

        await!(sleep_ms(200));

        let timeout = Timeout::new(rx.unit_error().boxed().compat(), Duration::from_secs(2));

        let result = await!(timeout.compat());
        assert!(result.is_err());
    }

    struct Mock(Option<Sender<()>>);
    impl Handler<()> for Mock {
        type Future = FutureObj<'static, ()>;

        fn call(&mut self, _state: Arc<State>) -> Self::Future {
            self.0.take().unwrap().send(()).unwrap();
            Box::new(future::ready(())).into()
        }
    }
}
