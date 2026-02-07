# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Query Service - Production-Optimized Multi-Stage Dockerfile
# =============================================================================
#
# Optimizations applied:
# - Runtime uses plain alpine instead of elixir image (~180MB -> ~8MB base)
# - Elixir release includes ERTS, no full Elixir installation needed at runtime
# - Migrations included for Ecto release tasks
# - Healthcheck uses release RPC instead of wget (removes wget dependency)
# - Combined RUN commands to reduce layers
# =============================================================================

ARG ELIXIR_VERSION=1.17
ARG OTP_VERSION=27
# Must match Alpine version in elixir:${ELIXIR_VERSION}-otp-${OTP_VERSION}-alpine for OpenSSL compatibility
ARG ALPINE_VERSION=3.23

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM elixir:${ELIXIR_VERSION}-otp-${OTP_VERSION}-alpine AS build

ARG VERSION=dev
ARG REVISION=unknown

# Install build dependencies and hex/rebar in single layer
RUN apk add --no-cache build-base git && \
    mix local.hex --force && \
    mix local.rebar --force

WORKDIR /app

# Set build environment
ENV MIX_ENV=prod \
    SECRET_KEY_BASE=build_time_dummy_key_will_be_replaced_at_runtime_minimum_64_chars

# Copy mix files first for dependency caching
COPY mix.exs mix.lock ./

# Install and compile dependencies (cached layer)
RUN mix deps.get --only prod && \
    mix deps.compile

# Copy source code and priv directory (includes migrations)
COPY lib ./lib
COPY config ./config
COPY priv ./priv

# Compile and build release
RUN mix compile && \
    mix release

# =============================================================================
# Stage 2: Runtime
# =============================================================================
# Use plain alpine - Elixir releases are self-contained with ERTS included.
# This reduces image size from ~180MB (elixir-alpine) to ~8MB (alpine base).
# The release at _build/prod/rel/query_service_ex includes everything needed.
# =============================================================================
FROM alpine:${ALPINE_VERSION} AS runtime

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# OCI Image Labels
LABEL org.opencontainers.image.title="Chronos Query Service" \
      org.opencontainers.image.description="High-performance query service for Chronos event store" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo"

# Install minimal runtime dependencies and create non-root user in single layer
# - tini: proper signal handling and zombie reaping
# - libstdc++: required by ERTS for C++ standard library
# - libgcc: required by ERTS for GCC runtime
# - ncurses-libs: required by ERTS for terminal handling
# - libcrypto3: required by ERTS crypto module (OpenSSL)
RUN apk add --no-cache tini libstdc++ libgcc ncurses-libs libcrypto3 && \
    rm -rf /var/cache/apk/* /tmp/* && \
    addgroup -g 1000 -S app && \
    adduser -u 1000 -S app -G app -h /app -s /sbin/nologin

WORKDIR /app

# Copy release from build stage
COPY --from=build --chown=app:app /app/_build/prod/rel/query_service_ex ./

USER app

# Expose port - matches prod.exs and runtime.exs default of 3902
EXPOSE 3902

# Environment configuration
# Required environment variables (must be provided at runtime):
#   DATABASE_URL        - PostgreSQL connection URL (ecto://USER:PASS@HOST/DATABASE)
#   SECRET_KEY_BASE     - Secret key for signing (generate with: mix phx.gen.secret)
#   GOOGLE_CLIENT_ID    - Google OAuth client ID
#   GOOGLE_CLIENT_SECRET- Google OAuth client secret
#   GITHUB_CLIENT_ID    - GitHub OAuth client ID
#   GITHUB_CLIENT_SECRET- GitHub OAuth client secret
#
# Optional environment variables:
#   PORT                - HTTP port (default: 3902)
#   POOL_SIZE           - Database pool size (default: 10)
#   PHX_HOST            - Hostname for URLs (default: localhost)
#   RUST_CORE_URL       - Rust Core service URL (default: http://localhost:3900)
#   CORE_WS_URL         - Rust Core WebSocket URL (default: ws://localhost:3900)
#   LEMON_SQUEEZY_API_KEY       - LemonSqueezy API key for billing
#   LEMON_SQUEEZY_STORE_ID      - LemonSqueezy store ID
#   LEMON_SQUEEZY_WEBHOOK_SECRET- LemonSqueezy webhook secret
ENV HOME=/app \
    MIX_ENV=prod \
    PORT=3902 \
    ERL_FLAGS="+sbwt very_short +sbwtdcpu very_short +sbwtdio very_short" \
    MALLOC_ARENA_MAX=2

# Health check using release RPC - no wget needed
# This calls the release's built-in RPC mechanism to verify the VM is responsive
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD bin/query_service_ex rpc "IO.puts(:ok)" | grep -q "ok" || exit 1

# Use tini for proper signal handling
ENTRYPOINT ["/sbin/tini", "-g", "--"]

CMD ["bin/query_service_ex", "start"]
