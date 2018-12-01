//! # Clique
//!
//! A SWIM based gossip protocol library. Coming soon.

#![feature(await_macro, async_await, futures_api)]

#[macro_use]
extern crate prost_derive;

mod broadcasts;
mod codec;
mod node;
mod peer;
mod rpc;
mod state;

pub use crate::node::Node;
