# syntax=docker/dockerfile:1

FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY epub-builder ./epub-builder
COPY src ./src

RUN cargo build --release && strip target/release/kiyomi

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 kiyomi

COPY --from=builder /build/target/release/kiyomi /usr/local/bin/kiyomi

ENV KIYOMI_MANGA_DIR=/manga \
    KIYOMI_CACHE_PATH=/cache/kiyomi.cache \
    RUST_LOG=info

VOLUME ["/manga", "/cache"]

USER kiyomi

ENTRYPOINT ["/usr/local/bin/kiyomi"]
