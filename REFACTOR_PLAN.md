# Monorepo Refactoring Plan

## Current Structure (Before)

```
chronos-monorepo/
├── apps/
│   └── web/                    # Next.js web app
├── packages/
│   ├── mcp-server/             # MCP server
│   └── ui/                     # Shared UI components
├── services/
│   ├── core/                   # Rust event store
│   ├── control-plane/          # Go control plane
│   └── query_service_ex/       # Elixir query service
└── tooling/
    └── biome/                  # Linting config
```

## Proposed Structure (After)

```
chronos-monorepo/
├── apps/
│   ├── web/                    # Next.js web app (existing)
│   ├── mcp-server/             # MCP server (moved from packages)
│   ├── core/                   # Rust event store (moved from services)
│   ├── control-plane/          # Go control plane (moved from services)
│   └── query-service/          # Elixir query service (moved/renamed)
├── packages/
│   ├── ui/                     # Shared UI components (existing)
│   ├── typescript-config/      # Shared TS configs (new)
│   └── event-types/            # Shared event type definitions (new)
└── tooling/
    └── biome/                  # Linting config (existing)
```

## Rationale

### Why Move to Apps?

1. **MCP Server** → `apps/`
   - It's a standalone application, not a shared library
   - Has its own runtime and deployment

2. **Core (Rust)** → `apps/`
   - Independent service with its own binary
   - Not a shared library
   - Main application of the platform

3. **Control Plane (Go)** → `apps/`
   - Standalone service
   - Independent deployment
   - Application, not library

4. **Query Service (Elixir)** → `apps/`
   - Standalone Phoenix app
   - Independent deployment
   - Rename to `query-service` for consistency

### Naming Conventions

- **apps/**: Deployable applications (dashed-names)
  - `web`, `mcp-server`, `core`, `control-plane`, `query-service`

- **packages/**: Shared libraries (dashed-names)
  - `ui`, `typescript-config`, `event-types`

## Migration Steps

1. **Phase 1: Move MCP Server**
   ```bash
   mv packages/mcp-server apps/mcp-server
   ```

2. **Phase 2: Move Core**
   ```bash
   mv services/core apps/core
   ```

3. **Phase 3: Move Control Plane**
   ```bash
   mv services/control-plane apps/control-plane
   ```

4. **Phase 4: Move & Rename Query Service**
   ```bash
   mv services/query_service_ex apps/query-service
   ```

5. **Phase 5: Update Build Config**
   - Update `turbo.json`
   - Update `package.json` workspace config
   - Update documentation

6. **Phase 6: Cleanup**
   - Remove empty `services/` directory
   - Move documentation files to proper locations

## Port Assignments (Unchanged)

- **Port 3900**: Rust Core (`apps/core`)
- **Port 3901**: Go Control Plane (`apps/control-plane`)
- **Port 3902**: Elixir Query Service (`apps/query-service`)
- **Port 3000**: Next.js Web (`apps/web`)
- **Port varies**: MCP Server (`apps/mcp-server`)

## Benefits

1. **Clearer Organization**: Apps vs Packages distinction
2. **Consistent Naming**: All dash-case names
3. **Better DX**: Easier to navigate
4. **Standard Convention**: Follows monorepo best practices
5. **Scalability**: Easy to add new apps or packages

## Impact Assessment

### Low Risk
- No code changes required
- Only directory moves
- Update path references in configs

### Files to Update
- `turbo.json` - update app paths
- `package.json` - update workspace paths
- Root `README.md` - update directory references
- Each app's documentation
- CI/CD configs (if any)

## Rollback Plan

If issues arise:
```bash
git checkout -- .
git clean -fd
```

All changes are reversible with git.
