FROM rustlang/rust:nightly-slim as builder

WORKDIR /usr/src/watchtower
COPY . .

# Install OpenSSL and pkg-config
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

# Install OpenSSL runtime dependencies
RUN apt-get update && \
    apt-get install -y libssl3 && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin
COPY --from=builder /usr/src/watchtower/target/release/watch_tower_worker .

WORKDIR /app
COPY service /app/service

# Create log directory and set permissions
RUN mkdir -p /app/service/log && \
    chmod 777 /app/service/log

CMD ["./watch_tower_worker"] 