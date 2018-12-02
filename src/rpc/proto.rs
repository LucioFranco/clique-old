pub use self::proto::{client, server, Peer, Pull, Push};

#[allow(dead_code)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/clique.proto.rs"));
}
