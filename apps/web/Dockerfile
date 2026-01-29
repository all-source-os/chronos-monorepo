# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Web - Production-Optimized Multi-Stage Dockerfile
# 
# Build from monorepo root: docker build -f apps/web/Dockerfile -t chronos-web .
# =============================================================================

ARG BUN_VERSION=1.1
ARG NODE_VERSION=20
ARG ALPINE_VERSION=3.20

# =============================================================================
# Stage 1: Dependencies
# =============================================================================
FROM oven/bun:${BUN_VERSION}-alpine AS deps

RUN apk add --no-cache libc6-compat

WORKDIR /app

# Copy workspace configuration
COPY package.json bun.lock turbo.json ./

# Copy workspace packages that web depends on
COPY packages/ui/package.json ./packages/ui/
COPY tooling/typescript/package.json ./tooling/typescript/

# Copy web app package
COPY apps/web/package.json ./apps/web/

# Install all dependencies (workspace-aware)
RUN bun install --frozen-lockfile

# =============================================================================
# Stage 2: Build
# =============================================================================
FROM oven/bun:${BUN_VERSION}-alpine AS builder

ARG VERSION=dev
ARG REVISION=unknown

WORKDIR /app

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/ui/node_modules ./packages/ui/node_modules
COPY --from=deps /app/apps/web/node_modules ./apps/web/node_modules

# Copy workspace configuration
COPY package.json bun.lock turbo.json biome.json ./

# Copy UI package source (workspace dependency)
COPY packages/ui ./packages/ui

# Copy typescript config
COPY tooling/typescript ./tooling/typescript

# Copy web app source
COPY apps/web ./apps/web

# Environment for build
ENV NEXT_TELEMETRY_DISABLED=1 \
    NODE_ENV=production \
    NEXT_PUBLIC_VERSION=${VERSION}

# Build the web application
WORKDIR /app/apps/web
RUN bun run build

# =============================================================================
# Stage 3: Runtime
# =============================================================================
FROM node:${NODE_VERSION}-alpine3.20 AS runner

ARG VERSION=dev
ARG REVISION=unknown
ARG BUILDTIME=unknown

# OCI Image Labels
LABEL org.opencontainers.image.title="Chronos Web" \
      org.opencontainers.image.description="Interactive web dashboard for Chronos event store" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.created="${BUILDTIME}" \
      org.opencontainers.image.vendor="AllSource Team" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.source="https://github.com/all-source-os/chronos-monorepo"

WORKDIR /app

# Environment
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME="0.0.0.0"

# Install tini, wget for healthcheck, and create non-root user
RUN apk add --no-cache tini wget && \
    rm -rf /var/cache/apk/* && \
    addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs

# Copy built application with correct permissions
COPY --from=builder /app/apps/web/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/apps/web/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/apps/web/.next/static ./.next/static

USER nextjs

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/ || exit 1

# Use tini for signal handling
ENTRYPOINT ["/sbin/tini", "-g", "--"]

CMD ["node", "server.js"]
