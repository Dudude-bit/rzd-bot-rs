FROM rust:1.75-slim-buster as build

# create a new empty shell project
RUN USER=root cargo new --bin rzd_tg_bot
WORKDIR /rzd_tg_bot
RUN apt-get update
RUN apt-get install -y pkg-config libssl-dev
# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/rzd_tg_bot*
RUN cargo build --release

# our final base
FROM debian:buster-slim

# copy the build artifact from the build stage
COPY --from=build /rzd_tg_bot/target/release/rzd_tg_bot .

# set the startup command to run your binary
CMD ["./rzd_tg_bot"]