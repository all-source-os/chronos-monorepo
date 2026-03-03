# Monorepo Structure — Best Practices

> **This is authoritative.** All contributors (human and AI) must follow this layout. Do not create new top-level directories or place code in the wrong location.

---

## Directory Layout

```
chronos-monorepo/
├── apps/                  # Deployable services (backend, frontend)
│   ├── core/              # Rust event store (AllSource Core)
│   ├── query-service/     # Elixir/Phoenix API gateway
│   ├── control-plane/     # Go control plane
│   ├── web/               # Next.js frontend dashboard
│   └── mcp-server-elixir/ # Elixir MCP server
│
├── sdks/                  # Client SDKs — ALL languages go here
│   ├── rust/              # Rust SDK (allsource crate)
│   ├── go/                # Go SDK
│   └── python-client/     # Python SDK
│
├── packages/              # Shared internal packages (NOT SDKs)
│   └── ui/                # Shared UI component library
│
├── deploy/                # Deployment manifests
│   └── k8s/               # Kubernetes manifests
│
├── docs/                  # Documentation
│   ├── proposals/         # Design proposals
│   ├── use-cases/         # Use case documents
│   ├── current/           # Current architecture docs
│   ├── sales/             # Sales-facing docs
│   └── checklists/        # Operational checklists
│
├── .claude/
│   └── skills/            # Claude Code skills (auto-discovered)
│       ├── chronos-data-flow/          # Docker stack data flow test
│       ├── chronos-data-flow-embedded/ # Embedded backend data flow test
│       ├── chronos-durability/         # Container restart durability test
│       ├── chronos-embedded-durability/# Rust crate crash recovery test
│       └── chronos-release/            # Versioned release automation
│
├── scripts/               # Build and automation scripts
├── tooling/               # Developer tooling
│   ├── data-flow-test/             # Docker stack E2E test
│   ├── durability-test/            # Container restart durability test
│   ├── embedded-data-flow-test/    # Embedded backend E2E test
│   └── embedded-durability-test/   # Rust crate crash recovery test
└── test-results/          # CI test output artifacts
```

---

## Rules

### 1. SDKs go in `sdks/`, nowhere else

Every client library — regardless of language — lives under `sdks/`. Not in `packages/`, not in `apps/`, not at the root.

```
sdks/
├── rust/           # name by language, not "rust-client"
├── go/
├── python-client/  # (legacy naming, would be python/ if starting fresh)
├── typescript/     # future
└── swift/          # future
```

Naming convention: prefer the bare language name (`rust/`, `go/`, `python/`). Avoid `-client` suffix for new SDKs.

### 2. `apps/` is for deployable services only

Each subdirectory in `apps/` produces a deployable artifact (Docker image, binary, or web bundle). If it doesn't get deployed independently, it doesn't belong here.

### 3. `packages/` is for shared internal code

Shared libraries consumed by `apps/` — UI component libraries, shared types, internal utilities. Not for external-facing client SDKs.

### 4. `docs/` subdirectories have purpose

| Directory | Contents |
|-----------|----------|
| `proposals/` | Design proposals (RFC-style, named `UPPER_SNAKE_CASE.md`) |
| `use-cases/` | Use case documents tied to proposals |
| `current/` | Living architecture documentation |
| `sales/` | Customer-facing documents |
| `checklists/` | Operational runbooks and checklists |

### 5. No stray top-level directories

Do not create new top-level directories without updating this document. The current set is intentional:

- `apps/` — services
- `sdks/` — client SDKs
- `packages/` — shared internal packages
- `deploy/` — deployment configs
- `docs/` — documentation
- `scripts/` — automation
- `tooling/` — dev tools
- `test-results/` — CI artifacts

### 6. Version consistency

All services and SDKs share the same version number. Use `make set-version VERSION=X.Y.Z` to bump everywhere. After bumping, verify with `make check-versions`.

SDK versions must match the release tag. If Core is v0.10.5, every SDK is v0.10.5.

---

## Cleanup History

Legacy SDK copies in `packages/` (`go-client/`, `python-client/`, `client/`) were removed in v0.10.5. All SDK code now lives exclusively in `sdks/`.
