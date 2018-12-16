# FROM liuchong/rustup:nightly

# WORKDIR /clique
# COPY . .

# RUN cargo build --all

# CMD ["./target/debug/agent"]
# select image
FROM liuchong/rustup:nightly

# create a new empty shell project  
RUN USER=root cargo new /agent --lib
WORKDIR /agent

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

COPY components components

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/clique*
RUN cargo build --all --release

ENV PATH="/agent/target/release/:${PATH}"

# set the startup command to run your binary
CMD ["./target/release/agent"]
