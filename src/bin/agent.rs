#![feature(pin, await_macro, async_await, futures_api)]

#[macro_use]
extern crate tokio;

use {
    clique::Node,
    log::{error, info},
    std::{net::SocketAddr, sync::Arc},
};

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

        info!("Attempting to join cluster via: {}", peer_addr.to_string());
        // Join a remote cluster or _Clique_
        if let Err(e) = await!(node.join(vec![peer_addr])) {
            error!("Error joining cluster: {}", e);
        }
    }

    // Starts TCP, UDP and Gossip tasks
    if let Err(e) = await!(node.serve()) {
        error!("Error running node: {}", e);
    }
}
