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
│   ├── mcp-server-elixir/ # Elixir MCP server
│   └── chronis/           # Event-sourced task CLI (standalone workspace)
│
├── sdks/                  # Client SDKs — ALL languages go here
│   ├── rust/              # Rust SDK (allsource crate)
│   ├── go/                # Go SDK
│   └── python-client/     # Python SDK
│
├── crates/                # Shared Rust crates (libraries, NOT binaries)
│   └── (future)           # e.g., allsource-core library split
│
├── packages/              # Shared non-Rust packages (NOT SDKs, NOT crates)
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

### 3. Apps are fully isolated

**This is a hard rule.** Each app is self-contained — no app may depend on, import, or copy another app's source code.

#### What isolation means in practice

| Principle | Correct | Wrong |
|-----------|---------|-------|
| **Build** | Each app's Dockerfile only references its own source and shared crates (`sdks/`, `packages/`) | `COPY apps/chronis/ apps/chronis/` inside Core's Dockerfile |
| **Cargo workspace** | Apps that need independence use `workspace.exclude` and manage their own `[dependencies]` | Every Rust app in one flat workspace, forcing all Dockerfiles to stub every other app |
| **Runtime** | Apps communicate over the network (HTTP, gRPC, WebSocket) | Direct function calls or shared-memory coupling between apps |
| **Shared code** | Extract to `sdks/` or `packages/` | One app importing from another app's `src/` |

#### Rust workspace isolation

Apps that would create cross-contamination in Dockerfiles (e.g., chronis depends on core as a library) should be **excluded** from the root Cargo workspace:

```toml
# Root Cargo.toml
[workspace]
exclude = ["apps/chronis"]
```

The excluded app manages its own dependency versions and can still use `path = "../core"` for local development. This means:
- Core's Dockerfile does not need to know chronis exists
- Registry's Dockerfile does not need to stub chronis
- Adding a new app never requires modifying another app's build

#### Why this matters

Without isolation, adding app N forces changes to apps 1 through N-1. Dockerfiles accumulate stubs, build times grow, and a change to one app's Cargo.toml can break another app's Docker build. Isolation keeps each app's blast radius to itself.

### 4. `crates/` is for shared Rust libraries

Shared Rust crates consumed by multiple `apps/`. If a Rust crate is a library (not a deployable binary) and is used by more than one app, it belongs in `crates/`.

```
crates/
└── (future)     # e.g., allsource-core library if split from the binary
```

**Note**: `allsource-core` currently lives in `apps/core/` because it is both a library and a deployable binary. If/when the library is split from the binary, the library portion moves to `crates/` and the binary stays in `apps/`.

**DO NOT** put crates in `packages/`, `sdks/`, or `apps/` unless they are deployable binaries.

### 5. `packages/` is for shared non-Rust packages (NOT SDKs, NOT crates)

Shared non-Rust libraries consumed by `apps/` — UI component libraries, shared types, internal utilities. Not for external-facing client SDKs, not for Rust crates.

### 6. SDKs and crates are fully isolated (same as apps)

The same isolation principles from Rule 3 apply to `sdks/` and `crates/`:

- **Each SDK is self-contained.** No SDK may import from another SDK, from `apps/`, or from `crates/` source code directly.
- **Each shared crate is self-contained.** Crates may depend on other crates via `path = "../other-crate"` within `crates/`, but never on `apps/` source.
- **Dockerfiles never cross boundaries.** An app's Dockerfile references its own source, plus `crates/` and `sdks/` Cargo.toml stubs if needed — never another app.
- **No circular dependencies.** The dependency graph is strictly: `apps/` → `crates/` → (external). SDKs are leaf nodes — they depend on nothing internal.

```
Allowed dependency direction:

  apps/       →  crates/     →  (external crates.io deps)
  apps/       →  sdks/       (only if embedding an SDK)
  sdks/       →  (external)  (SDKs are standalone HTTP clients)
  crates/     →  crates/     (shared crate → shared crate OK)
  packages/   →  (external)  (UI components are standalone)
```

**DO NOT** create dependencies from `sdks/` → `apps/`, `sdks/` → `crates/`, `crates/` → `apps/`, or `packages/` → `apps/`.

### 7. `docs/` subdirectories have purpose

| Directory | Contents |
|-----------|----------|
| `proposals/` | Design proposals (RFC-style, named `UPPER_SNAKE_CASE.md`) |
| `use-cases/` | Use case documents tied to proposals |
| `current/` | Living architecture documentation |
| `sales/` | Customer-facing documents |
| `checklists/` | Operational runbooks and checklists |

### 8. No stray top-level directories

Do not create new top-level directories without updating this document. The current set is intentional:

- `apps/` — deployable services and binaries
- `crates/` — shared Rust library crates
- `sdks/` — client SDKs
- `packages/` — shared non-Rust packages
- `deploy/` — deployment configs
- `docs/` — documentation
- `scripts/` — automation
- `tooling/` — dev tools
- `test-results/` — CI artifacts

### 9. Version consistency

All services and SDKs share the same version number. Use `make set-version VERSION=X.Y.Z` to bump everywhere. After bumping, verify with `make check-versions`.

SDK versions must match the release tag. If Core is v0.10.5, every SDK is v0.10.5.

---

## Cleanup History

Legacy SDK copies in `packages/` (`go-client/`, `python-client/`, `client/`) were removed in v0.10.5. All SDK code now lives exclusively in `sdks/`.
