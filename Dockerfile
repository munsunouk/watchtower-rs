from rustlang/rust:nightly-slim as builder

workdir /usr/src/watchtower
copy . .

# Install OpenSSL and pkg-config
run apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

run cargo build --release

from debian:bookworm-slim

# Install OpenSSL runtime dependencies with proper key handling
run apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    update-ca-certificates && \
    apt-get install -y --no-install-recommends libssl3 && \
    rm -rf /var/lib/apt/lists/*

workdir /app

# Create necessary directories first
run mkdir -p /app/worker
run mkdir -p /app/lib/src/cli/abi
run mkdir -p /app/service

# Copy the built binary
copy --from=builder /usr/src/watchtower/target/release/watch_tower_worker .

# Copy configuration files
copy worker/config.yaml /app/worker/
copy worker/param.yaml /app/worker/

# Copy ABI files
copy --from=builder /usr/src/watchtower/lib/src/cli/abi /app/lib/src/cli/abi

cmd ["./watch_tower_worker"] 