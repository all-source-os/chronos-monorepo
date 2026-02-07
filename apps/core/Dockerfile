# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Core - Production-Optimized Multi-Stage Dockerfile
# Optimized for: Fast cold starts, minimal image size, security
#
# Build targets:
#   docker build --target runtime .        # Distroless (~2MB base, smallest)
#   docker build --target runtime-alpine . # Alpine (~7MB base, has HEALTHCHECK)
# =============================================================================

ARG RUST_VERSION=1.92
ARG ALPINE_VERSION=3.23

# =============================================================================
# Stage 1: Chef - Prepare cargo-chef for dependency caching
# =============================================================================
FROM rust:${RUST_VERSION}-alpine AS chef

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf && \
    cargo install cargo-chef --locked

WORKDIR /app

# =============================================================================
# Stage 2: Planner - Analyze dependencies
# =============================================================================
FROM chef AS planner

# Copy only what's needed for dependency analysis
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Create minimal stubs for optional targets (referenced in Cargo.toml but not needed for main binary)
RUN mkdir -p benches tests/stress_tests && \
    echo 'fn main() {}' > benches/performance_benchmarks.rs && \
    echo 'fn main() {}' > tests/stress_tests/seven_day_stress.rs && \
    cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Builder - Build with cached dependencies
# =============================================================================
FROM chef AS builder

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# Copy dependency recipe and build dependencies (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source files needed for binary build
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Create minimal stubs and build in single layer for cache efficiency
RUN mkdir -p benches tests/stress_tests && \
    echo 'fn main() {}' > benches/performance_benchmarks.rs && \
    echo 'fn main() {}' > tests/stress_tests/seven_day_stress.rs && \
    CARGO_BUILD_JOBS=4 cargo build --release --bin allsource-core && \
    strip /app/target/release/allsource-core

# =============================================================================
# Stage 4a: Runtime (DEFAULT) - Minimal distroless production image
# Smallest image size, no shell/package manager = minimal attack surface
# Health checks: Use Kubernetes probes or orchestrator-level health checks
# =============================================================================
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

LABEL org.opencontainers.image.title="Chronos Core" \
      org.opencontainers.image.description="High-performance event store built in Rust" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo" \
      org.opencontainers.image.base.name="gcr.io/distroless/static-debian12:nonroot"

WORKDIR /app

# Copy binary (distroless:nonroot runs as uid 65532 by default)
COPY --from=builder /app/target/release/allsource-core ./allsource-core

EXPOSE 3900

ENV RUST_LOG=allsource_core=info,tower_http=info \
    ALLSOURCE_HOST=0.0.0.0 \
    ALLSOURCE_PORT=3900 \
    ALLSOURCE_DATA_DIR=/app/data \
    PORT=3900 \
    MALLOC_ARENA_MAX=2

# Note: Distroless has no shell/curl. Use orchestrator health probes:
# - Kubernetes: livenessProbe/readinessProbe httpGet to GET /health:3900
# - Returns: {"status":"healthy","service":"allsource-core","version":"..."}

ENTRYPOINT ["./allsource-core"]

# =============================================================================
# Stage 4b: Runtime Alpine - For environments requiring Docker HEALTHCHECK
# Slightly larger but includes curl for native Docker health checks
# =============================================================================
FROM alpine:${ALPINE_VERSION} AS runtime-alpine

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

LABEL org.opencontainers.image.title="Chronos Core" \
      org.opencontainers.image.description="High-performance event store built in Rust" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo" \
      org.opencontainers.image.base.name="alpine:${ALPINE_VERSION}"

# Install only curl for health checks and ca-certificates, create user in single layer
RUN apk add --no-cache curl ca-certificates && \
    addgroup -g 1000 -S allsource && \
    adduser -u 1000 -S allsource -G allsource -h /app -s /sbin/nologin && \
    mkdir -p /app/data && \
    chown -R allsource:allsource /app

WORKDIR /app

COPY --from=builder --chown=allsource:allsource /app/target/release/allsource-core ./allsource-core

USER allsource

EXPOSE 3900

ENV RUST_LOG=allsource_core=info,tower_http=info \
    ALLSOURCE_HOST=0.0.0.0 \
    ALLSOURCE_PORT=3900 \
    ALLSOURCE_DATA_DIR=/app/data \
    PORT=3900 \
    MALLOC_ARENA_MAX=2

# Docker-native health check for standalone deployments
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3900/health || exit 1

ENTRYPOINT ["./allsource-core"]
