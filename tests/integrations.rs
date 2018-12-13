#![feature(pin, await_macro, async_await, futures_api)]

mod util;

use {
    crate::util::{async_current_thread_test, async_test, next_addr, sleep_ms, spawn},
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
    let nodes = await!(create_cluster(3));
    await!(assert_eventually_eq(nodes, 2, 30));
}

#[async_current_thread_test]
async fn join_5_node() {
    let nodes = await!(create_cluster(5));
    await!(assert_eventually_eq(nodes, 4, 30));
}

#[async_test]
async fn join_12_node() {
    let nodes = await!(create_cluster(12));
    await!(assert_eventually_eq(nodes, 11, 100));
}

async fn create_cluster(size: usize) -> Vec<Arc<Node>> {
    let join_addr = next_addr();
    let node_origin = Arc::new(Node::new(join_addr));
    let node_origin_clone = node_origin.clone();
    spawn(async move { await!(node_origin_clone.serve()).unwrap() });

    await!(sleep_ms(100));

    let mut nodes = vec![node_origin.clone()];
    for _ in 0usize..(size - 1) {
        let node = Arc::new(Node::new(next_addr()));
        let node_clone = node.clone();
        spawn(async move { await!(node_clone.serve()).unwrap() });

        await!(node.join(vec![join_addr])).unwrap();
        nodes.push(node);
    }

    nodes
}

async fn assert_eventually_eq(nodes: Vec<Arc<Node>>, num: usize, limit: usize) {
    for node in nodes {
        for i in 0..limit {
            let node = node.clone();
            let peers = await!(node.peers()).len();

            if i == (limit - 1) {
                assert_eq!(peers, num);
            }

            if peers != num {
                await!(sleep_ms(100));
            } else {
                break;
            }
        }
    }
}
