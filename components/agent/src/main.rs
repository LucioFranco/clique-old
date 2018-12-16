#![feature(pin, await_macro, async_await, futures_api)]

use {
    clique::{Net, Node as RawNode},
    futures::future::{FutureExt, TryFutureExt},
    log::{error, info},
    std::{net::{SocketAddr, ToSocketAddrs}, sync::Arc},
};

type Node = RawNode<Net>;

fn main() {
    let local_addr: SocketAddr = resolve(std::env::args()
        .nth(1)
        .unwrap_or("[::]:8080".into()));

    let peer_addr = std::env::args().nth(2);

    //std::env::set_var("RUST_LOG", "clique=debug");
    pretty_env_logger::init();

    // Bootstrap lazy future to allow us to call spawn
    let server = run(local_addr, peer_addr).unit_error().boxed().compat();

    tokio::run(server);
}

async fn run(local_addr: SocketAddr, peer_addr: Option<String>) {
    // Create a new Node, which contains all the state for
    // our Clique based service
    let node = Arc::new(Node::new(local_addr));

    if let Some(peer_addr) = peer_addr {
        let peer_addr: SocketAddr = resolve(peer_addr);
        info!("connecting to: {}", peer_addr);
        let node = Arc::clone(&node);

        info!("Attempting to join cluster via: {}", peer_addr.to_string());
        // Join a remote cluster or _Clique_
        if let Err(e) = await!(node.join(vec![peer_addr])) {
            error!("Error joining cluster: {}", e);
        }
    }

    // Starts TCP, UDP and Gossip tasks
    if let Err(e) = await!(node.run()) {
        error!("Error running node: {}", e);
    }
}

fn resolve<A: ToSocketAddrs>(target: A) -> SocketAddr {
    let mut addrs = target.to_socket_addrs().unwrap();
    match addrs.next() {
        Some(addr) => addr,
        None => panic!("Could not resolve!"),
    }
}