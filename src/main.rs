extern crate clique;
extern crate pretty_env_logger;
extern crate tokio;

use clique::Node;
use std::net::SocketAddr;

fn main() {
    let remote_addr: SocketAddr = std::env::args()
        .nth(1)
        .unwrap_or("127.0.0.1:8080".into())
        .parse()
        .unwrap();

    let peer_addr: SocketAddr = std::env::args()
        .nth(2)
        .unwrap_or("127.0.0.1:8081".into())
        .parse()
        .unwrap();

    std::env::set_var("RUST_LOG", "clique=debug");
    pretty_env_logger::init();

    let mut node = Node::new(remote_addr);

    tokio::run(node.serve(vec![peer_addr]));
}
