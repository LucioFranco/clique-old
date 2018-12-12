#![feature(pin, await_macro, async_await, futures_api)]

extern crate tokio_async_await_test;

use {
    clique::Node,
    futures::{
        compat::{Future01CompatExt, TokioDefaultSpawner},
        future::FutureExt,
        task::SpawnExt,
    },
    std::{future::Future, sync::Arc},
    tokio_async_await_test::async_current_thread_test,
};

#[async_current_thread_test]
async fn join() {
    let node_a_addr = "127.0.0.1:7878".to_string();

    let node_a = Arc::new(Node::new(node_a_addr.parse().unwrap()));
    let node_a_clone = Arc::clone(&node_a);
    spawn(async move { await!(node_a_clone.serve()).unwrap() });

    // Sleep to let node_a start its rpc server
    await!(sleep_ms(500));

    let node_b = Node::new("127.0.0.1:7879".parse().unwrap());
    await!(node_b.join(vec![node_a_addr.parse().unwrap()])).unwrap();

    let peers_a = await!(node_a.peers());
    assert_eq!(peers_a.len(), 1);

    let peers_b = await!(node_b.peers());
    assert_eq!(peers_b.len(), 1);
}

#[async_current_thread_test]
async fn join_3_node() {
    let node_a_addr = "127.0.0.1:7890".to_string();
    let node_b_addr = "127.0.0.1:7891".to_string();
    let node_c_addr = "127.0.0.1:7892".to_string();

    let node_a = Node::new(node_a_addr.parse().unwrap());
    spawn(async move { await!(node_a.serve()).unwrap() });

    let node_b = Arc::new(Node::new(node_b_addr.parse().unwrap()));

    await!(sleep_ms(500));

    await!(node_b.join(vec![node_a_addr.parse().unwrap()])).unwrap();

    let node_b_clone = Arc::clone(&node_b);
    spawn(async move { await!(node_b_clone.serve()).unwrap() });

    let members = await!(node_b.peers());
    assert_eq!(members.len(), 1);

    await!(sleep_ms(500));

    let node_c = Node::new(node_c_addr.parse().unwrap());
    await!(node_c.join(vec![node_a_addr.parse().unwrap()])).unwrap();
    spawn(async move { await!(node_c.serve()).unwrap() });

    await!(sleep_ms(1000));

    let members = await!(node_b.peers());
    assert_eq!(members.len(), 2);
}

async fn sleep_ms(ms: u64) {
    use std::time::{Duration, Instant};
    use tokio::await;
    use tokio::timer::Delay;

    let when = Instant::now() + Duration::from_millis(ms);

    await!(Delay::new(when).compat()).expect("Error running sleep");
}

fn spawn<F>(f: F)
where
    F: Future + Send + 'static,
{
    TokioDefaultSpawner.spawn(f.map(|_| ())).unwrap();
}
