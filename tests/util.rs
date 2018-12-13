#![allow(unused_attributes)]
#![feature(await_macro, async_await, futures_api)]

use std::{
    future::Future,
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

use futures::{
    compat::{Future01CompatExt, TokioDefaultSpawner},
    future::FutureExt,
    task::SpawnExt,
};

pub use tokio_async_await_test::{async_current_thread_test, async_test};

#[macro_export]
macro_rules! assert_eventually_eq {
    ($a: expr, $b: expr) => {{
        await!(sleep_ms(200));

        if $a != $b {
            assert_eq!($a, $b);
        }
    }};
}

pub async fn sleep_ms(ms: u64) {
    use std::time::{Duration, Instant};
    use tokio::timer::Delay;

    let when = Instant::now() + Duration::from_millis(ms);

    await!(Delay::new(when).compat()).expect("Error running sleep");
}

pub fn spawn<F>(f: F)
where
    F: Future + Send + 'static,
{
    TokioDefaultSpawner.spawn(f.map(|_| ())).unwrap();
}

static NEXT_PORT: AtomicUsize = AtomicUsize::new(1234);
pub fn next_addr() -> SocketAddr {
    use std::net::{IpAddr, Ipv4Addr};
    let port = NEXT_PORT.fetch_add(1, Ordering::AcqRel) as u16;
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}
