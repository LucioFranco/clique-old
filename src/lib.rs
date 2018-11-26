//! # Clique
//!
//! A SWIM based gossip protocol library. Coming soon.

#![warn(rust_2018_compatibility)]
#![warn(rust_2018_idioms)]

extern crate bytes;
extern crate futures;
extern crate indexmap;
extern crate log;
extern crate serde_derive;
extern crate tokio;
extern crate uuid;

#[macro_use]
extern crate prost_derive;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_http;

mod broadcasts;
mod codec;
mod node;
mod peer;
mod rpc;
mod state;

pub use crate::node::Node;
