# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Query Service - Production-Optimized Multi-Stage Dockerfile
# =============================================================================

ARG ELIXIR_VERSION=1.17
ARG OTP_VERSION=27
ARG ALPINE_VERSION=3.20

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM elixir:${ELIXIR_VERSION}-otp-${OTP_VERSION}-alpine AS build

ARG VERSION=dev
ARG REVISION=unknown

# Install build dependencies
RUN apk add --no-cache build-base git

WORKDIR /app

# Install hex and rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Set build environment
ENV MIX_ENV=prod \
    SECRET_KEY_BASE=build_time_dummy_key_will_be_replaced_at_runtime_minimum_64_chars

# Copy mix files first for dependency caching
COPY mix.exs mix.lock ./

# Install and compile dependencies (cached layer)
RUN mix deps.get --only prod && \
    mix deps.compile

# Copy source code
COPY lib ./lib
COPY config ./config

# Create priv directory if it doesn't exist (optional for Phoenix)
RUN mkdir -p priv

# Compile and build release
RUN mix compile && \
    mix release

# =============================================================================
# Stage 2: Runtime (use same base as build for library compatibility)
# =============================================================================
FROM elixir:${ELIXIR_VERSION}-otp-${OTP_VERSION}-alpine AS runtime

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

# Install minimal runtime dependencies
RUN apk add --no-cache \
    tini \
    wget && \
    rm -rf /var/cache/apk/* /tmp/*

# Create non-root user
RUN addgroup -g 1000 -S app && \
    adduser -u 1000 -S app -G app -h /app -s /sbin/nologin

WORKDIR /app

# Copy release from build stage
COPY --from=build --chown=app:app /app/_build/prod/rel/query_service_ex ./

USER app

# Expose port
EXPOSE 4000

# Environment configuration
ENV HOME=/app \
    MIX_ENV=prod \
    PORT=4000 \
    ERL_FLAGS="+sbwt very_short +sbwtdcpu very_short +sbwtdio very_short" \
    MALLOC_ARENA_MAX=2

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:4000/api/health || exit 1

# Use tini for proper signal handling
ENTRYPOINT ["/sbin/tini", "-g", "--"]

CMD ["bin/query_service_ex", "start"]
