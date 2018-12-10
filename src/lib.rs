//! # Clique
//!
//! A SWIM based gossip protocol agent and library. Coming soon.

#![feature(pin, async_await, await_macro, futures_api)]

mod broadcasts;
mod codec;
mod error;
mod failure;
mod node;
mod peer;
mod rpc;
mod state;

pub use crate::error::{Error, Result};
pub use crate::node::Node;
