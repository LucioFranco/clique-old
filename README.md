# Clique

[![Build Status](https://travis-ci.org/LucioFranco/clique.svg?branch=master)](https://travis-ci.org/LucioFranco/clique)

A SWIM based gossip agent and library. Inspired by serf.

## Roadmap

The roadmap can be found here [roadmap](docs/ROADMAP.md), this project is still very much a work in progress and requires nightly to compile.

## Building

To build clique you will need a nightly version of `rustc`, my current version is `rustc 1.32.0-nightly (d09466ceb 2018-11-30)`. This toolchain is needed to use `async/await`.

``` bash
cargo +nightly build --all
cargo +nightly test --all
cargo +nightly run --package clique-agent -- <bind addr> <peer addr>
```

## Project Structure

### Main Crate - clique

The main crate that contains the SWIM based gossip protocol.

### Agent - clique-agent

The CLI agent that can be used to run clique as a sidecar process. More to come here...

### Proto - clique-proto

The base RPC protobuf types for use with `tower-grpc`. It contains the logic to actually build the RPC types in Rust.
