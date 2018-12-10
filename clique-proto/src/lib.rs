//! The RPC protocol for Clique
//! 
//! This crate provides the bare protobuf and prost types for interacting
//! with the Clique RPC service. This crate will also use `tower-grpc-build`
//! to build the grpc types via `prost`.

#[macro_use]
extern crate prost_derive;

pub use self::proto::{client, server, Peer, Pull, Push};

#[allow(dead_code)]
mod proto {
    include!(concat!(env!("OUT_DIR"), "/clique.proto.rs"));
}
