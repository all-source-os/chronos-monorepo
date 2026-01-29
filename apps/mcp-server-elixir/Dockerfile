# syntax=docker/dockerfile:1

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM elixir:1.17-otp-27-alpine AS builder

# Install build dependencies
RUN apk add --no-cache git build-base

# Set build environment
ENV MIX_ENV=prod

WORKDIR /app

# Install hex + rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Copy config and mix files
COPY mix.exs mix.lock ./
COPY config config

# Install dependencies
RUN mix deps.get --only $MIX_ENV
RUN mix deps.compile

# Copy source code
COPY lib lib

# Compile and create release
RUN mix compile
RUN mix release

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM alpine:3.20

# Labels for OCI compliance
LABEL org.opencontainers.image.title="Chronos MCP Server" \
      org.opencontainers.image.description="Model Context Protocol server for AI-native Chronos integration" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo"

# Install runtime dependencies
RUN apk add --no-cache libstdc++ libgcc ncurses-libs openssl \
    && addgroup -g 1000 chronos \
    && adduser -u 1000 -G chronos -s /bin/sh -D chronos

WORKDIR /app

# Copy release from builder
COPY --from=builder --chown=chronos:chronos /app/_build/prod/rel/mcp_server_elixir ./

# Use non-root user
USER chronos:chronos

# Expose port (MCP typically uses stdio, but HTTP endpoint may be needed)
EXPOSE 4001

ENV PHX_SERVER=true
ENV MIX_ENV=prod

# Health check via /health endpoint if available
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD bin/mcp_server_elixir rpc "System.otp_release()" || exit 1

ENTRYPOINT ["bin/mcp_server_elixir"]
CMD ["start"]
