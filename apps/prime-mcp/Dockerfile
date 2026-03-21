# syntax=docker/dockerfile:1.7
# =============================================================================
# AllSource Prime — Multi-Stage Dockerfile
#
# Build from repo root (required for path dependencies):
#   docker build -t allsource-prime -f apps/prime-mcp/Dockerfile .
#
# Run:
#   docker run -v prime-data:/data -p 3905:3905 allsource-prime
# =============================================================================

ARG RUST_VERSION=1.92

# =============================================================================
# Stage 1: Builder
# =============================================================================
FROM rust:${RUST_VERSION}-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev cmake g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy only what prime-mcp needs: itself + allsource-core (path dep)
COPY apps/prime-mcp/ apps/prime-mcp/
COPY apps/core/ apps/core/

# Build release binary
WORKDIR /build/apps/prime-mcp
RUN cargo build --release

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r prime && useradd -r -g prime -d /data prime

COPY --from=builder /build/apps/prime-mcp/target/release/allsource-prime /usr/local/bin/

RUN mkdir -p /data && chown prime:prime /data
VOLUME /data

USER prime

ENV PRIME_DATA_DIR=/data \
    PRIME_MODE=http \
    PRIME_PORT=3905 \
    PRIME_LOG_LEVEL=info

EXPOSE 3905

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3905/health || exit 1

ENTRYPOINT ["allsource-prime"]
CMD ["--data-dir", "/data", "--mode", "http", "--port", "3905"]
