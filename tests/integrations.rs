#![feature(pin, await_macro, async_await, futures_api)]

extern crate tokio_async_await_test;

use clique::Node;
use tokio_async_await_test::async_test;

#[async_test]
async fn join() {
    let node_a_addr = "127.0.0.1:7878".to_string();

    let node_a = Node::new(node_a_addr.parse().unwrap());
    tokio::spawn_async(async move { await!(node_a.serve()).unwrap() });

    // Sleep to let node_a start its rpc server
    await!(sleep_ms(500));

    let node_b = Node::new("127.0.0.1:7879".parse().unwrap());
    await!(node_b.join(vec![node_a_addr.parse().unwrap()])).unwrap();
    tokio::spawn_async(async move { await!(node_b.serve()).unwrap() });
}

async fn sleep_ms(ms: u64) {
    use tokio::await;
    use std::time::{Duration, Instant};
    use tokio::timer::Delay;

    let when = Instant::now() + Duration::from_millis(ms);

    await!(Delay::new(when)).expect("Error running sleep");
}
