# Clique

[![Build Status](https://travis-ci.org/LucioFranco/clique.svg?branch=master)](https://travis-ci.org/LucioFranco/clique)

A SWIM based gossip agent and library. Inspired by serf.

## Building

To build clique you will need a nightly version of `rustc`, my current version is `rustc 1.32.0-nightly (d09466ceb 2018-11-30)`. This toolchain is needed to use `async/await`.

``` bash
cargo +nightly build
cargo +nightly run -- <bind addr> <peer addr>
```
