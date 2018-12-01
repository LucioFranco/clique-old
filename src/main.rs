#![feature(await_macro, async_await, futures_api)]

use clique::Node;
use std::{net::SocketAddr, sync::Arc};

#[macro_use]
extern crate tokio;

fn main() {
    let local_addr: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:8080".into())
        .parse()
        .unwrap();

    let peer_addr = std::env::args().nth(2);

    std::env::set_var("RUST_LOG", "clique=debug");
    pretty_env_logger::init();

    // Bootstrap lazy future to allow us to call spawn
    let server = run(local_addr, peer_addr);

    tokio::run_async(server);
}

async fn run(local_addr: SocketAddr, peer_addr: Option<String>) {
    // Create a new Node, which contains all the state for
    // our Clique based service
    let node = Arc::new(Node::new(local_addr));

    if let Some(peer_addr) = peer_addr {
        let peer_addr: SocketAddr = peer_addr.parse().unwrap();
        let node = Arc::clone(&node);

        // Join a remote cluster or _Clique_
        await!(node.join(vec![peer_addr])).unwrap();
    }

    // Starts TCP, UDP and Gossip tasks
    await!(node.serve()).expect("Error running node.serve");
}
