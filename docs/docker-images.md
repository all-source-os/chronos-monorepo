---
title: "Chronos Docker Images"
status: CURRENT
last_updated: 2026-02-02
category: operations
---

# Chronos Docker Images

This document describes the official Docker images for the Chronos event store platform.

## Authentication

GitHub Container Registry requires authentication to pull images:

```bash
# Create a Personal Access Token (PAT) with read:packages scope at:
# https://github.com/settings/tokens/new?scopes=read:packages

# Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin

# Or use GitHub CLI (recommended)
gh auth token | docker login ghcr.io -u $(gh api user -q .login) --password-stdin
```

> **Note**: If the packages are made public, authentication is not required for pulling.

## Available Images

All images are published to GitHub Container Registry (GHCR):

| Image | Description | Port |
|-------|-------------|------|
| `ghcr.io/all-source-os/chronos-core` | High-performance Rust event store | 3900 |
| `ghcr.io/all-source-os/chronos-query-service` | Elixir query service with SQL interface | 4000 |
| `ghcr.io/all-source-os/chronos-control-plane` | Go enterprise orchestration | 8080 |
| `ghcr.io/all-source-os/chronos-web` | Next.js web dashboard | 3000 |
| `ghcr.io/all-source-os/chronos-mcp-server` | Model Context Protocol server for AI integration | 4001 |

## Image Tags

Images are tagged with multiple identifiers:

- `latest` - Latest build from main branch
- `v0.7.0` - Specific version (semver)
- `0.7` - Major.minor version
- `sha-abc1234` - Git commit SHA

## Quick Start

### Using Docker Compose (Recommended)

Create a `docker-compose.yml`:

```yaml
services:
  chronos-core:
    image: ghcr.io/all-source-os/chronos-core:latest
    ports:
      - "3900:3900"
    volumes:
      - chronos-data:/data
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3900/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  chronos-query:
    image: ghcr.io/all-source-os/chronos-query-service:latest
    ports:
      - "4000:4000"
    environment:
      - CHRONOS_CORE_URL=http://chronos-core:3900
      - DATABASE_URL=postgres://postgres:postgres@postgres:5432/chronos
      - REDIS_URL=redis://redis:6379
    depends_on:
      - chronos-core
      - postgres
      - redis

  chronos-web:
    image: ghcr.io/all-source-os/chronos-web:latest
    ports:
      - "3000:3000"
    environment:
      - NEXT_PUBLIC_API_URL=http://localhost:4000
    depends_on:
      - chronos-query

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_DB=chronos
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
    volumes:
      - postgres-data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

volumes:
  chronos-data:
  postgres-data:
  redis-data:
```

Start the stack:

```bash
docker compose up -d
```

### Single Container

Run just the core event store:

```bash
docker run -d \
  --name chronos-core \
  -p 3900:3900 \
  -v chronos-data:/data \
  ghcr.io/all-source-os/chronos-core:latest
```

## Environment Variables

### chronos-core

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | `info` |
| `DATA_DIR` | Data directory path | `/data` |
| `HTTP_PORT` | HTTP server port | `3900` |

### chronos-query-service

| Variable | Description | Default |
|----------|-------------|---------|
| `CHRONOS_CORE_URL` | URL of chronos-core | Required |
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `REDIS_URL` | Redis connection string | Required |
| `PORT` | HTTP server port | `4000` |
| `SECRET_KEY_BASE` | Phoenix secret key | Required for prod |

### chronos-control-plane

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | `8080` |
| `JWT_SECRET` | JWT signing secret | Required |
| `JAEGER_ENDPOINT` | Jaeger tracing endpoint | Optional |

### chronos-web

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | `3000` |
| `NEXT_PUBLIC_API_URL` | Public API URL | Required |

### chronos-mcp-server

| Variable | Description | Default |
|----------|-------------|---------|
| `CHRONOS_CORE_URL` | URL of chronos-core | Required |
| `MIX_ENV` | Elixir environment | `prod` |

## Multi-Architecture Support

All images are built for:
- `linux/amd64` (x86_64)
- `linux/arm64` (Apple Silicon, ARM servers)

## Health Checks

All images include health checks:

```bash
# Check if container is healthy
docker inspect --format='{{.State.Health.Status}}' chronos-core
```

## Security

- All images run as non-root users
- Base images are Alpine Linux (minimal attack surface)
- Images are scanned for vulnerabilities during CI/CD
- No secrets are baked into images

## License

Chronos is licensed under the **MIT License**.

```
MIT License

Copyright (c) 2024-2025 AllSource Team

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
```

### Attribution Requirements

When using Chronos in your projects:

1. **Source code**: Include the LICENSE file if redistributing source
2. **Binary distribution**: Include license notice in documentation
3. **SaaS/hosted**: No attribution required in UI, but welcome

### Commercial Use

MIT license permits:
- Commercial use without royalties
- Modification and private use
- Distribution and sublicensing
- Patent use (implicit grant)

No restrictions on:
- Proprietary derivatives
- Closed-source modifications
- Charging for services built on Chronos

## Support

- GitHub Issues: https://github.com/all-source-os/chronos-monorepo/issues
- Documentation: https://github.com/all-source-os/chronos-monorepo/tree/main/docs

## Changelog

See [CHANGELOG.md](../CHANGELOG.md) for version history.
