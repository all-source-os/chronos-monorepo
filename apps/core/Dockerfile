# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Core - Production-Optimized Multi-Stage Dockerfile
# Optimized for: Fast cold starts, minimal image size, security
# =============================================================================

ARG RUST_VERSION=1.92
ARG ALPINE_VERSION=3.20

# =============================================================================
# Stage 1: Chef - Prepare cargo-chef for dependency caching
# =============================================================================
FROM rust:${RUST_VERSION}-alpine AS chef

RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconf

RUN cargo install cargo-chef --locked

WORKDIR /app

# =============================================================================
# Stage 2: Planner - Analyze dependencies
# =============================================================================
FROM chef AS planner

# Copy all source files needed by Cargo.toml
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
COPY examples ./examples
COPY tests ./tests

RUN cargo chef prepare --recipe-path recipe.json

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

# Copy all source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
COPY examples ./examples
COPY tests ./tests

# Build with version info embedded
ENV CARGO_BUILD_JOBS=4
RUN cargo build --release --bin allsource-core && \
    strip /app/target/release/allsource-core

# =============================================================================
# Stage 4: Runtime - Minimal production image
# =============================================================================
FROM alpine:${ALPINE_VERSION} AS runtime

# Build arguments for labels
ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# OCI Image Labels
LABEL org.opencontainers.image.title="Chronos Core" \
      org.opencontainers.image.description="High-performance event store built in Rust" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo" \
      org.opencontainers.image.base.name="alpine:${ALPINE_VERSION}"

# Install minimal runtime dependencies
RUN apk add --no-cache \
    libgcc \
    ca-certificates \
    tini && \
    rm -rf /var/cache/apk/* /tmp/*

# Security: Create non-root user
RUN addgroup -g 1000 -S allsource && \
    adduser -u 1000 -S allsource -G allsource -h /app -s /sbin/nologin

WORKDIR /app

# Copy binary with proper ownership
COPY --from=builder --chown=allsource:allsource /app/target/release/allsource-core ./allsource-core

# Create data directory
RUN mkdir -p /app/data && chown -R allsource:allsource /app

# Switch to non-root user
USER allsource

# Expose port
EXPOSE 3900

# Environment configuration
ENV RUST_LOG=allsource_core=info,tower_http=info \
    ALLSOURCE_HOST=0.0.0.0 \
    ALLSOURCE_PORT=3900 \
    ALLSOURCE_DATA_DIR=/app/data \
    PORT=3900 \
    MALLOC_ARENA_MAX=2

# Use tini for proper signal handling
ENTRYPOINT ["/sbin/tini", "-g", "--"]

# Start application
CMD ["./allsource-core"]
