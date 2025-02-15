# Build Stage
FROM rust:1.84.1 AS builder
WORKDIR /usr/src/

RUN apt-get update && \
  apt-get install -y libdbus-1-dev pkg-config

RUN USER=root cargo new tilt-recorder
WORKDIR /usr/src/tilt-recorder
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && \
  apt-get install -y libssl3 dbus

COPY --from=builder /usr/local/cargo/bin/tilt-recorder /usr/local/bin/tilt-recorder
CMD ["tilt-recorder"]