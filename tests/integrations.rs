#![feature(await_macro, async_await, futures_api)]

use clique::Node;
use futures::future::{FutureExt, TryFutureExt};

#[test]
fn join() {
    let fut = async {
        let node_a_addr = "127.0.0.1:7878".to_string();

        let node_a = Node::new(node_a_addr.parse().unwrap());
        tokio::spawn_async(async move { await!(node_a.serve()).unwrap() });

        // Sleep to let node_a start its rpc server
        await!(sleep_ms(500));

        let node_b = Node::new("127.0.0.1:7879".parse().unwrap());
        await!(node_b.join(vec![node_a_addr.parse().unwrap()])).unwrap();
        tokio::spawn_async(async move { await!(node_b.serve()).unwrap() });
    };

    run_test(fut)
}

fn run_test<F>(fut: F)
where
    F: futures::Future<Output = ()> + Send + 'static,
{
    use tokio::runtime::Runtime;

    let mut rt = Runtime::new().unwrap();

    rt.block_on(fut.unit_error().boxed().compat()).unwrap();
}

async fn sleep_ms(ms: u64) {
    use futures::compat::Future01CompatExt;
    use std::time::{Duration, Instant};
    use tokio::timer::Delay;

    let when = Instant::now() + Duration::from_millis(ms);

    await!(Delay::new(when).compat()).expect("Error running sleep");
}
