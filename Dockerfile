FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY epub-builder ./epub-builder

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    rm -rf src

COPY src ./src

RUN cargo build --release --target x86_64-unknown-linux-musl


FROM alpine:latest

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/kiyomi /kiyomi

ENTRYPOINT ["/kiyomi"]
