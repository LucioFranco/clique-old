use std::{
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_PORT: AtomicUsize = AtomicUsize::new(1234);

pub fn next_addr() -> SocketAddr {
    use std::net::{IpAddr, Ipv4Addr};

    let port = NEXT_PORT.fetch_add(1, Ordering::AcqRel) as u16;
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}
