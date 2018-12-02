//! # Clique
//!
//! A SWIM based gossip protocol library. Coming soon.

#![feature(pin, async_await, await_macro, futures_api)]

#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate tokio;

mod broadcasts;
mod client;
mod codec;
mod node;
mod peer;
mod rpc;
mod state;

pub use crate::node::Node;
