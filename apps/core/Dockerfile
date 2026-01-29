# =============================================================================
# Chronos Core - Serverless-Ready Dockerfile
# Optimized for fast cold starts and minimal image size
# =============================================================================

# Build stage - use cargo-chef for optimal caching
FROM rust:1.75-alpine AS chef
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconf
RUN cargo install cargo-chef --locked
WORKDIR /app

# Planner stage - analyze dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - build with cached dependencies
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin allsource-core && \
    strip /app/target/release/allsource-core

# =============================================================================
# Runtime stage - minimal distroless-like image
# =============================================================================
FROM alpine:3.20 AS runtime

# Install minimal runtime dependencies
RUN apk add --no-cache \
    libgcc \
    ca-certificates \
    tini && \
    rm -rf /var/cache/apk/*

# Create non-root user
RUN addgroup -g 1000 -S allsource && \
    adduser -u 1000 -S allsource -G allsource

WORKDIR /app

# Copy binary
COPY --from=builder --chown=allsource:allsource /app/target/release/allsource-core ./allsource-core

# Create data directory for persistent storage (optional in serverless)
RUN mkdir -p /app/data && chown -R allsource:allsource /app

USER allsource

# Expose port (serverless platforms typically use PORT env var)
EXPOSE 3900

# =============================================================================
# Serverless-optimized environment
# =============================================================================
ENV RUST_LOG=allsource_core=info,tower_http=info \
    ALLSOURCE_HOST=0.0.0.0 \
    ALLSOURCE_PORT=3900 \
    ALLSOURCE_DATA_DIR=/app/data \
    # Serverless: respect PORT env var (Cloud Run, Fly.io, etc.)
    PORT=3900 \
    # Reduce memory footprint
    MALLOC_ARENA_MAX=2

# Health check (disabled for serverless - platforms have their own)
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
#   CMD wget --no-verbose --tries=1 --spider http://localhost:${PORT}/health || exit 1

# Use tini for proper signal handling (SIGTERM for graceful shutdown)
ENTRYPOINT ["/sbin/tini", "-g", "--"]

# Start the application
CMD ["./allsource-core"]

# =============================================================================
# Build targets for different platforms
# =============================================================================
# Standard: docker build -t chronos-core .
# Cloud Run: docker build -t gcr.io/PROJECT/chronos-core .
# Fly.io: flyctl deploy (uses fly.toml)
# AWS: docker build -t ECR_URI/chronos-core .
