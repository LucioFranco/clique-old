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

    // Create a new Node, which contains all the state for
    // our Clique based service
    let node = Arc::new(Node::new(local_addr));

    // Bootstrap lazy future to allow us to call spawn
    let server = async move {
        if let Some(peer_addr) = peer_addr {
            let peer_addr: SocketAddr = peer_addr.parse().unwrap();
            let node = Arc::clone(&node);

            // Join a remote cluster or _Clique_
            tokio::spawn_async(async move { await!(node.join(vec![peer_addr])).unwrap() });
        }

        // Starts TCP, UDP and Gossip tasks
        tokio::spawn_async(async move { await!(node.serve()).unwrap() });
    };

    tokio::run_async(server);
}
