# syntax=docker/dockerfile:1.7
# =============================================================================
# AllSource Core - Production-Optimized Multi-Stage Dockerfile
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

RUN apk add --no-cache musl-dev && \
    cargo install cargo-chef --locked

WORKDIR /app

# =============================================================================
# Stage 2: Planner - Analyze dependencies
# =============================================================================
FROM chef AS planner

# Copy workspace root manifests first
COPY Cargo.toml Cargo.lock ./

# Copy the core app source
COPY apps/core/Cargo.toml apps/core/Cargo.lock* apps/core/
COPY apps/core/src apps/core/src

# Create stubs for workspace members and optional targets
# Only workspace.members need stubs — excluded crates (registry, better-auth-allsource)
# have their own Cargo.toml and are invisible to the workspace resolver.
COPY tooling/performance/Cargo.toml tooling/performance/Cargo.toml
COPY tooling/allsource-inspect/Cargo.toml tooling/allsource-inspect/Cargo.toml
COPY tooling/allsource-mcp/Cargo.toml tooling/allsource-mcp/Cargo.toml
COPY sdks/rust/Cargo.toml sdks/rust/Cargo.toml
# core_nif is a cdylib (Rustler NIF) which musl cannot produce, so we stub it
# as a plain lib to satisfy workspace resolution without triggering cdylib builds
RUN mkdir -p tooling/performance/src sdks/rust/src \
            tooling/allsource-inspect/src tooling/allsource-mcp/src \
            apps/mcp-server-elixir/native/core_nif/src \
            apps/core/benches apps/core/tests/stress_tests && \
    echo 'fn main() {}' > tooling/performance/src/main.rs && \
    echo 'fn main() {}' > tooling/allsource-inspect/src/main.rs && \
    echo 'fn main() {}' > tooling/allsource-mcp/src/main.rs && \
    echo '' > sdks/rust/src/lib.rs && \
    printf '[package]\nname = "core_nif"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\nname = "core_nif"\npath = "src/lib.rs"\n\n[dependencies]\n' \
        > apps/mcp-server-elixir/native/core_nif/Cargo.toml && \
    echo '' > apps/mcp-server-elixir/native/core_nif/src/lib.rs && \
    echo 'fn main() {}' > apps/core/benches/performance_benchmarks.rs && \
    echo 'fn main() {}' > apps/core/tests/stress_tests/seven_day_stress.rs && \
    cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3: Builder - Build with cached dependencies
# =============================================================================
FROM chef AS builder

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# Cook dependencies from recipe (cached layer — recipe is version-agnostic)
# cargo-chef replaces version fields with placeholders, so version bumps in
# Cargo.toml/Cargo.lock don't invalidate this expensive layer.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p allsource-core --features analytics

# Now copy real manifests (version-sensitive) and set up workspace structure
COPY Cargo.toml Cargo.lock ./
COPY apps/core/Cargo.toml apps/core/
COPY tooling/performance/Cargo.toml tooling/performance/
COPY tooling/allsource-inspect/Cargo.toml tooling/allsource-inspect/
COPY tooling/allsource-mcp/Cargo.toml tooling/allsource-mcp/
COPY sdks/rust/Cargo.toml sdks/rust/
RUN mkdir -p apps/core/src tooling/performance/src sdks/rust/src \
            tooling/allsource-inspect/src tooling/allsource-mcp/src \
            apps/mcp-server-elixir/native/core_nif/src && \
    echo 'fn main() {}' > apps/core/src/main.rs && \
    echo 'fn main() {}' > tooling/performance/src/main.rs && \
    echo 'fn main() {}' > tooling/allsource-inspect/src/main.rs && \
    echo 'fn main() {}' > tooling/allsource-mcp/src/main.rs && \
    echo '' > sdks/rust/src/lib.rs && \
    printf '[package]\nname = "core_nif"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\nname = "core_nif"\npath = "src/lib.rs"\n\n[dependencies]\n' \
        > apps/mcp-server-elixir/native/core_nif/Cargo.toml && \
    echo '' > apps/mcp-server-elixir/native/core_nif/src/lib.rs

# Copy actual source files for binary build
COPY apps/core/src apps/core/src
RUN mkdir -p apps/core/benches apps/core/tests/stress_tests && \
    echo 'fn main() {}' > apps/core/benches/performance_benchmarks.rs && \
    echo 'fn main() {}' > apps/core/tests/stress_tests/seven_day_stress.rs && \
    CARGO_BUILD_JOBS=4 cargo build --release -p allsource-core --bin allsource-core --features analytics && \
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

LABEL org.opencontainers.image.title="AllSource Core" \
      org.opencontainers.image.description="High-performance event store built in Rust" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/allsource-monorepo" \
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

LABEL org.opencontainers.image.title="AllSource Core" \
      org.opencontainers.image.description="High-performance event store built in Rust" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/allsource-monorepo" \
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
