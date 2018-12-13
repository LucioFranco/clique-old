#![feature(pin, await_macro, async_await, futures_api)]

mod util;

use {
    crate::util::{async_current_thread_test, next_addr, sleep_ms, spawn},
    clique::Node,
    std::sync::Arc,
};

#[async_current_thread_test]
async fn join() {
    let node_a_addr = next_addr();

    let node_a = Arc::new(Node::new(node_a_addr));
    let node_a_clone = Arc::clone(&node_a);
    spawn(async move { await!(node_a_clone.serve()).unwrap() });

    // Sleep to let node_a start its rpc server
    await!(sleep_ms(500));

    let node_b = Node::new(next_addr());
    await!(node_b.join(vec![node_a_addr])).unwrap();

    let peers_a = await!(node_a.peers());
    assert_eq!(peers_a.len(), 1);

    let peers_b = await!(node_b.peers());
    assert_eq!(peers_b.len(), 1);
}

#[async_current_thread_test]
async fn join_3_node() {
    let node_a_addr = next_addr();
    let node_b_addr = next_addr();
    let node_c_addr = next_addr();

    let node_a = Node::new(node_a_addr);
    spawn(async move { await!(node_a.serve()).unwrap() });

    let node_b = Arc::new(Node::new(node_b_addr));

    await!(sleep_ms(500));

    await!(node_b.join(vec![node_a_addr])).unwrap();

    let node_b_clone = Arc::clone(&node_b);
    spawn(async move { await!(node_b_clone.serve()).unwrap() });

    let members = await!(node_b.peers());
    assert_eq!(members.len(), 1);

    await!(sleep_ms(500));

    let node_c = Node::new(node_c_addr);
    await!(node_c.join(vec![node_a_addr])).unwrap();
    spawn(async move { await!(node_c.serve()).unwrap() });

    await!(sleep_ms(1500));

    assert_eq!(await!(node_b.peers()).len(), 2);
    // let num_members = async move {
    //     let peers = await!(node_b.peers());

    //     peers.len()
    // };
    // assert_eventually_eq!(await!(num_members), 2);
}
