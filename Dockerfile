from rustlang/rust:nightly-slim as builder

workdir /usr/src/watchtower
copy . .

# Install OpenSSL and pkg-config
run apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

run cargo build --release

from debian:bookworm-slim

# Install OpenSSL runtime dependencies
run apt-get update && \
    apt-get install -y --no-install-recommends libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

workdir /app
copy --from=builder /usr/src/watchtower/target/release/watch_tower_worker .
copy --from=builder /usr/src/watchtower/worker/config.yaml ./worker/
copy --from=builder /usr/src/watchtower/worker/param.yaml ./worker/
copy lib/src/cli/abi /app/lib/src/cli/abi

# Create directories for volume mounts
run mkdir -p /app/service

cmd ["./watch_tower_worker"] 