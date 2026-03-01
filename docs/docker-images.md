---
title: "AllSource Docker Images"
status: CURRENT
last_updated: 2026-03-01
category: operations
---

# AllSource Docker Images

This document describes the official Docker images for the AllSource event store platform.

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
| `ghcr.io/all-source-os/chronos-query-service` | Elixir API gateway (stateless, no database) | 3902 |
| `ghcr.io/all-source-os/chronos-control-plane` | Go auth, billing, operations | 3901 |
| `ghcr.io/all-source-os/chronos-web` | Next.js web dashboard | 3000 |
| `ghcr.io/all-source-os/chronos-mcp-server` | Elixir MCP server for AI integration | 4000 |

## Image Tags

Images are tagged with multiple identifiers:

- `latest` - Latest build from main branch
- `v0.10.7` - Specific version (semver)
- `0.10` - Major.minor version
- `sha-abc1234` - Git commit SHA

## Quick Start

### Using Docker Compose (Recommended)

Create a `docker-compose.yml`:

```yaml
services:
  allsource-core:
    image: ghcr.io/all-source-os/chronos-core:latest
    ports:
      - "3900:3900"
    volumes:
      - allsource-data:/data
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:3900/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  allsource-control-plane:
    image: ghcr.io/all-source-os/chronos-control-plane:latest
    ports:
      - "3901:3901"
    environment:
      - CORE_URL=http://allsource-core:3900
      - JWT_SECRET=your-jwt-secret
    depends_on:
      - allsource-core

  allsource-query:
    image: ghcr.io/all-source-os/chronos-query-service:latest
    ports:
      - "3902:3902"
    environment:
      - CORE_URL=http://allsource-core:3900
      - CONTROL_PLANE_URL=http://allsource-control-plane:3901
      - SECRET_KEY_BASE=generate-with-mix-phx-gen-secret
    depends_on:
      - allsource-core
      - allsource-control-plane

  allsource-web:
    image: ghcr.io/all-source-os/chronos-web:latest
    ports:
      - "3000:3000"
    environment:
      - NEXT_PUBLIC_API_URL=http://localhost:3902
    depends_on:
      - allsource-query

volumes:
  allsource-data:
```

Start the stack:

```bash
docker compose up -d
```

### Single Container

Run just the core event store:

```bash
docker run -d \
  --name allsource-core \
  -p 3900:3900 \
  -v allsource-data:/data \
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
| `CORE_URL` | URL of allsource-core | Required |
| `CONTROL_PLANE_URL` | URL of control plane | `http://localhost:3901` |
| `PORT` | HTTP server port | `3902` |
| `SECRET_KEY_BASE` | Phoenix secret key | Required for prod |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | Optional |
| `GITHUB_CLIENT_ID` | GitHub OAuth client ID | Optional |

### chronos-control-plane

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | `3901` |
| `CORE_URL` | URL of allsource-core | Required |
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
| `CORE_URL` | URL of chronos-core | Required |
| `MIX_ENV` | Elixir environment | `prod` |
| `PORT` | HTTP server port | `4000` |

## Multi-Architecture Support

All images are built for:
- `linux/amd64` (x86_64)
- `linux/arm64` (Apple Silicon, ARM servers)

## Health Checks

All images include health checks:

```bash
# Check if container is healthy
docker inspect --format='{{.State.Health.Status}}' allsource-core
```

## Security

- All images run as non-root users
- Base images are Alpine Linux (minimal attack surface)
- Images are scanned for vulnerabilities during CI/CD
- No secrets are baked into images

## License

AllSource is licensed under the **MIT License**.

```
MIT License

Copyright (c) 2024-2026 AllSource Team

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

When using AllSource in your projects:

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
- Charging for services built on AllSource

## Support

- GitHub Issues: https://github.com/all-source-os/allsource-monorepo/issues
- Documentation: https://github.com/all-source-os/allsource-monorepo/tree/main/docs

## Changelog

See [CHANGELOG.md](../CHANGELOG.md) for version history.
