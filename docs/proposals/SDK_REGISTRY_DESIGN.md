# SDK Registry Design — Self-Hosted Rust Registry on Fly.io

> **Status**: Implemented
> **Author**: Design session 2026-02-18
> **Scope**: SDK distribution for Rust, Go, TypeScript, Python

---

## 1. Problem

All four SDKs live in `sdks/` with no distribution mechanism. Users cannot `cargo add`, `go get`, `npm install`, or `pip install` our SDKs. The previous plan proposed publishing to four public registries (crates.io, npm, PyPI, separate Go repo), but that creates four account credentials to manage, four attack surfaces, and ongoing maintenance across four different ecosystems.

## 2. Solution

A single Rust binary (`apps/registry`) deployed on **Fly.io** that serves all four package manager protocols. CI uploads artifacts via a protected endpoint on release. Artifacts are stored on a Fly Volume. Token-gated access is supported via bearer tokens.

```
                         +--------------------------+
  cargo/npm/pip/go  ---> |  registry.all-source.xyz   |
                         |  (Fly.io, port 3901)     |
                         +------------+-------------+
                                      |
                         +------------v-------------+
                         |  AllFrame + Hyper server  |
                         |  /cargo/  Sparse registry |
                         |  /go/     GOPROXY         |
                         |  /npm/    Packument + tgz |
                         |  /pypi/   PEP 503 Simple  |
                         |  /upload  CI artifact push|
                         |  /health  Health check    |
                         +------------+-------------+
                                      |
                         +------------v-------------+
                         |  Fly Volume               |
                         |  /app/data/registry/      |
                         +--------------------------+
                                      ^
                                      |
                         +------------+-------------+
                         |  GitHub Actions CI        |
                         |  (on tag push)            |
                         |  1. Build artifacts       |
                         |  2. POST to /upload       |
                         +--------------------------+
```

### Why a single binary works

All four registry protocols are simple file-serving protocols:

| Protocol | Spec basis |
|----------|-----------|
| Go GOPROXY | [Explicitly documented](https://go.dev/ref/mod#goproxy-protocol) as compatible with static file servers |
| Cargo sparse | Static JSON files + tarball downloads |
| PyPI Simple | [PEP 503](https://peps.python.org/pep-0503/) — HTML index pages + file links |
| npm | JSON packument + tarball downloads |

The registry is essentially a file server with an upload endpoint and optional auth. The upload handler generates metadata (checksums, index files, packuments) automatically.

## 3. Data Layout (Fly Volume)

```
/app/data/registry/
├── cargo/
│   ├── config.json                          # auto-generated from Host header
│   ├── al/ls/allsource                      # index (JSON Lines)
│   └── crates/allsource/0.10.5/download     # .crate tarball
│
├── go/
│   └── github.com/all-source-os/allsource-go/
│       ├── @latest                           # latest version info
│       └── @v/
│           ├── list                          # text: version list
│           ├── v0.10.5.info                  # json: {"Version":"v0.10.5","Time":"..."}
│           ├── v0.10.5.mod                   # text: go.mod contents
│           └── v0.10.5.zip                   # zip: module source
│
├── npm/
│   ├── @allsource/client                    # json: packument (all versions)
│   └── @allsource/client/-/
│       └── client-0.10.5.tgz                # tarball
│
└── pypi/
    ├── simple/
    │   └── allsource-client/
    │       └── index.html                   # file links with sha256 fragments
    └── files/
        └── allsource-client-0.10.5.tar.gz   # sdist/wheel
```

## 4. Protocol Details

### 4a. Cargo Sparse Registry

Consumer config (`~/.cargo/config.toml` or project `.cargo/config.toml`):
```toml
[registries.allsource]
index = "sparse+https://registry.all-source.xyz/cargo/"
```

Usage:
```bash
cargo add allsource --registry allsource
```

**`/cargo/config.json`** (auto-generated from Host header):
```json
{
  "dl": "https://registry.all-source.xyz/cargo/crates/{crate}/{version}/download",
  "api": "https://registry.all-source.xyz/cargo"
}
```

**Index path formula** (by crate name length):
- 1 char: `1/{name}`
- 2 chars: `2/{name}`
- 3 chars: `3/{first-char}/{name}`
- 4+ chars: `{first-two}/{second-two}/{name}` — `allsource` → `al/ls/allsource`

### 4b. Go GOPROXY

Consumer config:
```bash
export GOPROXY=https://registry.all-source.xyz/go,https://proxy.golang.org,direct
```

Usage:
```bash
go get github.com/all-source-os/allsource-go@v0.10.5
```

Four static files per version under `/go/{module}/@v/`:

| File | Content-Type | Content |
|------|-------------|---------|
| `list` | `text/plain` | One version per line (`v0.10.5\n`) |
| `v0.10.5.info` | `application/json` | `{"Version":"v0.10.5","Time":"2026-02-18T00:00:00Z"}` |
| `v0.10.5.mod` | `text/plain` | Raw `go.mod` contents |
| `v0.10.5.zip` | `application/zip` | Source zip (entries prefixed `github.com/all-source-os/allsource-go@v0.10.5/`) |

### 4c. npm Registry

Consumer config (`~/.npmrc` or project `.npmrc`):
```ini
@allsource:registry=https://registry.all-source.xyz/npm/
```

Usage:
```bash
npm install @allsource/client
# or
bun add @allsource/client
```

### 4d. PyPI Simple API

Consumer config:
```bash
pip install allsource-client --index-url https://registry.all-source.xyz/pypi/simple/
```

The `#sha256=` fragment in index.html links is mandatory — pip uses it for integrity verification.

## 5. CI Pipeline — `upload-registry` Job

The release workflow builds SDK artifacts and uploads each to the running registry:

```bash
# Rust → .crate
cd sdks/rust && cargo package --allow-dirty

# TypeScript → .tgz
cd sdks/typescript && bun run build && npm pack

# Python → .tar.gz
cd sdks/python-client && hatch build

# Go → .zip (with correct internal paths)
# zip contents of sdks/go/ with module@version/ prefix
```

Upload via protected endpoint:
```bash
curl -X POST -H "Authorization: Bearer $DEPLOY_TOKEN" \
  --data-binary @artifact \
  https://registry.all-source.xyz/upload/{protocol}/{name}/{version}
```

The registry handles metadata generation (index files, packuments, checksums) automatically on upload. No separate metadata generation script needed.

## 6. Security Model

### Attack surface comparison

| Threat | Public registries | Self-hosted registry |
|--------|------------------|---------------------|
| Account takeover | 4 accounts to protect | 0 accounts (no registry accounts) |
| Typosquatting | Possible on all 4 | Impossible (you own the namespace) |
| Registry compromise | Out of your control | You control the server |
| Dependency confusion | Possible | Impossible (dedicated registry URL) |
| CI secret compromise | 4 tokens | 1 token (DEPLOY_TOKEN) |

### Authentication

- **Upload routes** (`POST /upload/*`): Protected by `DEPLOY_TOKEN` env var. CI sends `Authorization: Bearer <token>`.
- **Download routes**: If `REGISTRY_TOKENS` env var is set (comma-separated), all download requests require `Authorization: Bearer <token>` matching one of the listed tokens. If empty, downloads are public.
- **Health check** (`GET /health`): Always public.

### Package manager token configuration

All four package managers natively support bearer token auth:

**Cargo** (`~/.cargo/config.toml`):
```toml
[registries.allsource]
index = "sparse+https://registry.all-source.xyz/cargo/"
token = "Bearer as_sdk_xxxxxxxxxxxx"
```

**npm** (`.npmrc`):
```ini
@allsource:registry=https://registry.all-source.xyz/npm/
//registry.all-source.xyz/npm/:_authToken=as_sdk_xxxxxxxxxxxx
```

**pip** (`pip.conf` or `--index-url`):
```bash
pip install allsource-client \
  --index-url https://as_sdk_xxxxxxxxxxxx@registry.all-source.xyz/pypi/simple/
```

**Go** (`~/.netrc`):
```bash
machine registry.all-source.xyz login token password as_sdk_xxxxxxxxxxxx
export GOPROXY=https://registry.all-source.xyz/go,direct
export GONOSUMCHECK=github.com/all-source-os/allsource-go
```

## 7. Cost

| Resource | Fly.io free/hobby | Our usage | Monthly cost |
|----------|-------------------|-----------|-------------|
| Shared CPU VM | 3 shared VMs free | 1 VM | $0 |
| Fly Volume (1 GB) | Included | ~100 MB | $0 |
| Bandwidth | 100 GB/month free | ~1 GB/month | $0 |
| **Total** | | | **$0** |

At scale, a dedicated VM ($3.19/month for shared-cpu-1x 256MB) with larger volume handles significant traffic.

## 8. Consumer Experience

### First-time setup (one line per ecosystem)

```bash
# Rust — add to .cargo/config.toml (project or global)
[registries.allsource]
index = "sparse+https://registry.all-source.xyz/cargo/"

# Go — set env (or add to shell profile)
export GOPROXY=https://registry.all-source.xyz/go,https://proxy.golang.org,direct

# TypeScript — add to .npmrc (project or global)
@allsource:registry=https://registry.all-source.xyz/npm/

# Python — pass flag or set in pip.conf
pip install allsource-client --index-url https://registry.all-source.xyz/pypi/simple/
```

### After setup

```bash
cargo add allsource --registry allsource
go get github.com/all-source-os/allsource-go@v0.10.5
npm install @allsource/client
pip install allsource-client
```

## 9. Implementation

### Source: `apps/registry/`

Single Rust binary built with **AllFrame** framework (uses AllFrame's re-exported hyper, tokio, serde, tracing). Routes:

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Health check (always public) |
| `GET /cargo/config.json` | Cargo registry config |
| `GET /cargo/{prefix}/{name}` | Cargo index (JSON Lines) |
| `GET /cargo/crates/{crate}/{version}/download` | Cargo .crate download |
| `GET /go/{module}/@v/list` | Go version list |
| `GET /go/{module}/@v/{version}.info` | Go version info |
| `GET /go/{module}/@v/{version}.mod` | Go go.mod |
| `GET /go/{module}/@v/{version}.zip` | Go source zip |
| `GET /go/{module}/@latest` | Go latest version |
| `GET /npm/{scope}/{name}` | npm packument JSON |
| `GET /npm/{scope}/{name}/-/{tarball}` | npm tarball |
| `GET /pypi/simple/` | PyPI project index |
| `GET /pypi/simple/{project}/` | PyPI project files |
| `GET /pypi/files/{filename}` | PyPI file download |
| `POST /upload/{protocol}/{name}/{version}` | CI artifact upload |

### Deployment: Fly.io

- `fly.toml` in `apps/registry/`
- Dockerfile builds a minimal Alpine image
- Fly Volume at `/app/data` for artifact persistence
- `DEPLOY_TOKEN` and optional `REGISTRY_TOKENS` set as Fly secrets

## 10. What We No Longer Need

- `CRATES_IO_TOKEN`, `NPM_TOKEN`, `PYPI_TOKEN` secrets
- `GO_REPO_DEPLOY_KEY` secret and `all-source-os/allsource-go` mirror repo
- Accounts on crates.io, npm, PyPI
- Cloudflare R2 bucket, Workers, KV store

Replaced by: one Fly.io app, one deploy token, one domain.
