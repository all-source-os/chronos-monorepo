# syntax=docker/dockerfile:1.7
# =============================================================================
# Chronos Web - Production-Optimized Multi-Stage Dockerfile (Bun)
# 
# Build from monorepo root: docker build -f apps/web/Dockerfile -t chronos-web .
# =============================================================================

ARG BUN_VERSION=1.3
ARG ALPINE_VERSION=3.23

# =============================================================================
# Stage 1: Build with Bun
# =============================================================================
FROM oven/bun:${BUN_VERSION}-alpine AS builder

ARG VERSION=dev
ARG REVISION=unknown

RUN apk add --no-cache libc6-compat

WORKDIR /app

# Copy root package files first for better layer caching
COPY package.json bun.lock ./

# Copy ALL workspace package.jsons to satisfy lockfile (monorepo pattern)
# Tooling packages
COPY tooling/typescript/package.json tooling/typescript/

# Package packages
COPY packages/ui/package.json packages/ui/

# Install dependencies (cached layer if package.jsons unchanged)
RUN bun install

# Copy tooling configs (shared across packages)
COPY tooling/typescript tooling/typescript

# Copy packages/ui source and config
COPY packages/ui/src packages/ui/src
COPY packages/ui/tsconfig.json packages/ui/
COPY packages/ui/bunfig.toml packages/ui/
COPY packages/ui/postcss.config.mjs packages/ui/

# Copy web app source and configs
COPY apps/web/src apps/web/src
COPY apps/web/public apps/web/public
COPY apps/web/next.config.mjs apps/web/
COPY apps/web/postcss.config.mjs apps/web/
COPY apps/web/tsconfig.json apps/web/
COPY apps/web/next-env.d.ts apps/web/
COPY apps/web/components.json apps/web/
COPY apps/web/content apps/web/content

# Environment for build
ENV NEXT_TELEMETRY_DISABLED=1 \
    NODE_ENV=production \
    NEXT_PUBLIC_VERSION=${VERSION}

# Build the web application
WORKDIR /app/apps/web
RUN bun run build

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM node:20-alpine AS runner

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

# Install tini and create non-root user
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
