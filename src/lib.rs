//! # Clique
//!
//! A SWIM based gossip protocol agent and library. Coming soon.

#![feature(arbitrary_self_types, pin, async_await, await_macro, futures_api)]

mod acks;
mod broadcasts;
mod codec;
mod error;
// mod failure;
mod handlers;
mod node;
mod peer;
mod rpc;
mod state;
mod task;
mod transport;

#[cfg(test)]
mod support;

#[cfg(test)]
#[macro_use]
extern crate tokio_async_await_test;

pub use crate::error::{Error, Result};
pub use crate::node::Node;
pub use crate::transport::Net;
