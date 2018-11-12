extern crate clique;
extern crate futures;
extern crate pretty_env_logger;
extern crate tokio;

use clique::Node;
use futures::future;
use std::net::SocketAddr;

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
    let server = future::lazy(move || {
        // Create a new Node, which contains all the state for
        // our Clique based service
        let mut node = Node::new(local_addr);

        if let Some(peer_addr) = peer_addr {
            let peer_addr: SocketAddr = peer_addr.parse().unwrap();

            // Join a remote cluster or _Clique_
            tokio::spawn(node.join(peer_addr));
        }

        // Starts TCP, UDP and Gossip tasks
        tokio::spawn(node.serve());

        future::ok(())
    });

    tokio::run(server);
}
