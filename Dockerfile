FROM rust:1.85-bookworm AS builder

WORKDIR /app

ARG RUSTY_CASTLE_VERSION
ARG RUSTY_CASTLE_REVISION
ENV RUSTY_CASTLE_VERSION=${RUSTY_CASTLE_VERSION}
ENV RUSTY_CASTLE_REVISION=${RUSTY_CASTLE_REVISION}

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates

RUN cargo build --release -p rusty-castle

FROM debian:bookworm-slim

ARG RUSTY_CASTLE_VERSION
ARG RUSTY_CASTLE_REVISION
LABEL org.opencontainers.image.title="rusty-castle"
LABEL org.opencontainers.image.description="UPnP AV / DLNA MediaServer"
LABEL org.opencontainers.image.version="${RUSTY_CASTLE_VERSION}"
LABEL org.opencontainers.image.revision="${RUSTY_CASTLE_REVISION}"

COPY --from=builder /app/target/release/rusty-castle /usr/local/bin/rusty-castle

VOLUME ["/media"]
EXPOSE 49152/tcp
EXPOSE 1900/udp

ENTRYPOINT ["rusty-castle"]
CMD ["/media"]
