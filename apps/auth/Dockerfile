# syntax=docker/dockerfile:1.7
# =============================================================================
# AllSource Auth Service — Multi-Stage Dockerfile
#
# Build from repo root (required for crate path dependencies):
#   docker build -t allsource-auth -f apps/auth/Dockerfile .
#
# Run:
#   docker run -p 3903:3903 allsource-auth
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

# Copy workspace root manifest and lockfile
COPY Cargo.toml Cargo.lock ./

# Copy better-auth-allsource crate (auth adapter — path dependency)
COPY crates/better-auth-allsource/ crates/better-auth-allsource/

# Copy apps/core/ (workspace member referenced by root Cargo.toml)
COPY apps/core/ apps/core/

# Copy auth service itself
COPY apps/auth/ apps/auth/

# Stub workspace members that auth doesn't need but Cargo.toml references
RUN mkdir -p apps/mcp-server-elixir/native/core_nif/src \
    && echo '' > apps/mcp-server-elixir/native/core_nif/src/lib.rs \
    && printf '[package]\nname = "core_nif"\nversion = "0.1.0"\nedition = "2024"\n' \
       > apps/mcp-server-elixir/native/core_nif/Cargo.toml \
    && mkdir -p sdks/rust/src \
    && echo '' > sdks/rust/src/lib.rs \
    && printf '[package]\nname = "allsource"\nversion = "0.16.0"\nedition = "2024"\n' \
       > sdks/rust/Cargo.toml \
    && mkdir -p tooling/performance/src \
    && echo 'fn main() {}' > tooling/performance/src/main.rs \
    && printf '[package]\nname = "allsource-performance"\nversion = "0.1.0"\nedition = "2024"\n[[bin]]\nname = "allsource-performance"\npath = "src/main.rs"\n' \
       > tooling/performance/Cargo.toml

# Build release binary
WORKDIR /build/apps/auth
RUN cargo build --release

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM rust:${RUST_VERSION}-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r auth && useradd -r -g auth -d /data auth

COPY --from=builder /build/apps/auth/target/release/allsource-auth /usr/local/bin/

RUN mkdir -p /data && chown auth:auth /data
VOLUME /data

USER auth

ENV AUTH_SECRET=change-me-in-production \
    AUTH_BACKEND=allsource \
    AUTH_PORT=3903 \
    RUST_LOG=allsource_auth=info

EXPOSE 3903

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:3903/health || exit 1

ENTRYPOINT ["allsource-auth"]
