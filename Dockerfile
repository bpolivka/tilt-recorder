# Build Stage
FROM rust:1.62.0 AS builder
WORKDIR /usr/src/

RUN apt-get update && \
  apt-get install -y libdbus-1-dev pkg-config

RUN USER=root cargo new tilt-recorder
WORKDIR /usr/src/tilt-recorder
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src ./src
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && \
  apt-get install -y libssl1.1 dbus

COPY --from=builder /usr/local/cargo/bin/tilt-recorder /usr/local/bin/tilt-recorder
CMD ["tilt-recorder"]