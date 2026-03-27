# =============================================================================
# Dina Network — Validator Node Dockerfile
# Multi-stage build: Rust builder -> minimal Debian runtime
# =============================================================================

# ---------- Stage 1: Builder ----------
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    make \
    gcc \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first to cache dependency builds
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY node/ node/
COPY cli/ cli/
COPY contracts/ contracts/

# Build only the node binary in release mode
RUN cargo build --release --bin dina-node

# ---------- Stage 2: Runtime ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user to run the node
RUN useradd --create-home --shell /bin/bash dina

COPY --from=builder /build/target/release/dina-node /usr/local/bin/dina-node

RUN chmod +x /usr/local/bin/dina-node

# Data directory for chain storage and keys
RUN mkdir -p /data && chown dina:dina /data
VOLUME ["/data"]

USER dina

# P2P networking
EXPOSE 9944
# JSON-RPC
EXPOSE 8545
# REST API
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8080/health || exit 1

ENTRYPOINT ["dina-node"]
CMD [ \
    "--data-dir", "/data", \
    "--listen", "/ip4/0.0.0.0/tcp/9944", \
    "--rpc-port", "8545", \
    "--rest-port", "8080" \
]
