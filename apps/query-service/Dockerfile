# Build stage
FROM hexpm/elixir:1.19.1-erlang-27.3.1-alpine-3.20.3 AS build

# Install build dependencies
RUN apk add --no-cache build-base git

# Set working directory
WORKDIR /app

# Install hex and rebar
RUN mix local.hex --force && \
    mix local.rebar --force

# Copy mix files
COPY mix.exs mix.lock ./

# Set environment to prod
ENV MIX_ENV=prod

# Install dependencies
RUN mix deps.get --only prod && \
    mix deps.compile

# Copy source code
COPY lib ./lib
COPY config ./config
COPY priv ./priv

# Compile the application
RUN mix compile

# Build release
RUN mix release

# Runtime stage
FROM alpine:3.20.3

# Install runtime dependencies
RUN apk add --no-cache \
    openssl \
    ncurses-libs \
    libstdc++ \
    libgcc

# Create app user
RUN addgroup -S app && adduser -S app -G app

# Set working directory
WORKDIR /app

# Copy release from build stage
COPY --from=build --chown=app:app /app/_build/prod/rel/query_service_ex ./

# Set user
USER app

# Expose port
EXPOSE 3902

# Set environment variables
ENV HOME=/app
ENV MIX_ENV=prod
ENV PORT=3902

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:3902/api/health || exit 1

# Start the application
CMD ["bin/query_service_ex", "start"]
