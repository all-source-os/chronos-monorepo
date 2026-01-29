# syntax=docker/dockerfile:1

# ============================================================================
# Stage 1: Build
# ============================================================================
FROM golang:1.22-alpine AS builder

# Install build dependencies
RUN apk add --no-cache git ca-certificates tzdata

WORKDIR /app

# Copy go mod files first for better caching
COPY go.mod go.sum ./
RUN go mod download

# Copy source code
COPY . .

# Build the binary with optimizations
RUN CGO_ENABLED=0 GOOS=linux GOARCH=${TARGETARCH:-amd64} \
    go build -ldflags="-s -w" -o /app/control-plane .

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM alpine:3.20

# Labels for OCI compliance
LABEL org.opencontainers.image.title="Chronos Control Plane" \
      org.opencontainers.image.description="Enterprise orchestration and management for Chronos" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo"

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata \
    && addgroup -g 1000 chronos \
    && adduser -u 1000 -G chronos -s /bin/sh -D chronos

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/control-plane /app/control-plane

# Use non-root user
USER chronos:chronos

# Expose metrics and API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/control-plane"]
