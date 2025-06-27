FROM rustlang/rust:nightly-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml .

COPY lib /app/lib
COPY worker /app/worker
COPY service /app/service

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/watch_tower_worker /app/
COPY --from=builder /app/lib /app/lib

CMD ["./watch_tower_worker"]