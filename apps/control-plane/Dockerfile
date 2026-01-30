# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Control Plane - Production-Optimized Multi-Stage Dockerfile
# =============================================================================

ARG GO_VERSION=1.22
ARG ALPINE_VERSION=3.20

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM golang:${GO_VERSION}-alpine AS builder

ARG VERSION=dev
ARG REVISION=unknown
ARG TARGETOS=linux
ARG TARGETARCH=amd64

# Install build dependencies
RUN apk add --no-cache git ca-certificates tzdata

WORKDIR /app

# Copy go mod files first for better caching
COPY go.mod go.sum ./
RUN go mod download && go mod verify

# Copy source code
COPY . .

# Build with optimizations and version info
RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} \
    go build \
    -ldflags="-s -w -X main.version=${VERSION} -X main.revision=${REVISION}" \
    -trimpath \
    -o /app/control-plane .

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM alpine:${ALPINE_VERSION}

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# OCI Image Labels
LABEL org.opencontainers.image.title="Chronos Control Plane" \
      org.opencontainers.image.description="Enterprise orchestration and management for Chronos" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo" \
      org.opencontainers.image.base.name="alpine:${ALPINE_VERSION}"

# Install runtime dependencies and create user
RUN apk add --no-cache ca-certificates tzdata tini && \
    rm -rf /var/cache/apk/* /tmp/* && \
    addgroup -g 1000 chronos && \
    adduser -u 1000 -G chronos -s /sbin/nologin -D chronos

WORKDIR /app

# Copy binary from builder
COPY --from=builder --chown=chronos:chronos /app/control-plane /app/control-plane

# Use non-root user
USER chronos:chronos

# Expose metrics and API port
EXPOSE 8080

# Environment
ENV GIN_MODE=release
ENV PORT=8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Use tini for proper signal handling
ENTRYPOINT ["/sbin/tini", "-g", "--"]

CMD ["/app/control-plane"]
