FROM rust:1.85-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates

RUN cargo build --release -p rusty-castle

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/rusty-castle /usr/local/bin/rusty-castle

VOLUME ["/media"]
EXPOSE 49152/tcp
EXPOSE 1900/udp

ENTRYPOINT ["rusty-castle"]
CMD ["/media"]
