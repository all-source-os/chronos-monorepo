# =============================================================================
# Chronos Query Service - Serverless-Ready Dockerfile
# Optimized for fast cold starts and minimal image size
# =============================================================================

# Build stage
FROM hexpm/elixir:1.19.1-erlang-27.3.1-alpine-3.20.3 AS build

# Install build dependencies
RUN apk add --no-cache build-base git

WORKDIR /app

# Install hex and rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Copy mix files first for dependency caching
COPY mix.exs mix.lock ./

ENV MIX_ENV=prod

# Install and compile dependencies (cached layer)
RUN mix deps.get --only prod && \
    mix deps.compile

# Copy source code
COPY lib ./lib
COPY config ./config
COPY priv ./priv

# Compile and build release
RUN mix compile && mix release

# =============================================================================
# Runtime stage - minimal image for serverless
# =============================================================================
FROM alpine:3.20.3 AS runtime

# Install minimal runtime dependencies
RUN apk add --no-cache \
    openssl \
    ncurses-libs \
    libstdc++ \
    libgcc \
    tini && \
    rm -rf /var/cache/apk/*

# Create non-root user
RUN addgroup -g 1000 -S app && \
    adduser -u 1000 -S app -G app

WORKDIR /app

# Copy release from build stage
COPY --from=build --chown=app:app /app/_build/prod/rel/query_service_ex ./

USER app

# Expose port (serverless platforms typically use PORT env var)
EXPOSE 4000

# =============================================================================
# Serverless-optimized environment
# =============================================================================
ENV HOME=/app \
    MIX_ENV=prod \
    # Default port - serverless platforms override via PORT
    PORT=4000 \
    # Erlang VM tuning for serverless
    ERL_FLAGS="+sbwt very_short +sbwtdcpu very_short +sbwtdio very_short" \
    # Reduce memory footprint
    MALLOC_ARENA_MAX=2

# Health check (disabled for serverless - platforms have their own)
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
#   CMD wget --no-verbose --tries=1 --spider http://localhost:${PORT}/api/health || exit 1

# Use tini for proper signal handling (SIGTERM for graceful shutdown)
ENTRYPOINT ["/sbin/tini", "-g", "--"]

# Start the application
CMD ["bin/query_service_ex", "start"]

# =============================================================================
# Build targets for different platforms
# =============================================================================
# Standard: docker build -t chronos-query-service .
# Cloud Run: docker build -t gcr.io/PROJECT/chronos-query-service .
# Fly.io: flyctl deploy (uses fly.toml)
# AWS: docker build -t ECR_URI/chronos-query-service .
