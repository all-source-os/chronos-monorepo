# AllSource Documentation Index

**Last Updated**: 2025-11-04
**Repository**: chronos-monorepo

---

## 📖 Documentation Structure

This repository uses a structured documentation approach with:
- **Timestamps** for versioning
- **Clear deprecation** markers
- **Organized by type** (architecture, guides, operations, etc.)
- **Service-specific docs** in service directories

---

## 📂 Directory Organization

### Root Documentation (`/docs`)
```
docs/
├── current/          # Active, current documentation
├── archive/          # Historical/deprecated docs (timestamped)
├── guides/           # How-to guides and tutorials
│   └── mcp-server/  # MCP server specific guides
├── architecture/     # Architecture Decision Records (ADRs)
├── operations/       # Operational guides (deployment, monitoring)
├── roadmaps/         # Product roadmaps and planning
└── testing/          # Test plans and coverage reports
```

### App-Specific Documentation (Minimal)
Each app maintains minimal docs in its root:
```
apps/[app]/
├── README.md         # App overview and quick start (REQUIRED)
└── docs/            # Optional: app-specific detailed docs
    ├── CHANGELOG.md  # Version history
    ├── FEATURES.md   # Feature showcase
    ├── SECURITY.md   # Security documentation
    └── api/         # API documentation
```

**Rule**: Keep app-level docs minimal. Most documentation belongs in central `docs/`.

---

## 📋 Current Documentation

### Architecture & Design
- [Clean Architecture Guide](./current/CLEAN_ARCHITECTURE.md) - ⏰ 2025-10-22
- [SOLID Principles Guide](./current/SOLID_PRINCIPLES.md) - ⏰ 2025-10-22
- [Performance Optimization Guide](./current/PERFORMANCE.md) - ⏰ 2025-10-22
- [Architecture Optimization](./current/ARCHITECTURE_OPTIMIZATION.md) - ⏰ 2025-11-04
- [Event Store Features](./current/EVENT_STORE_FEATURES.md) - ⏰ 2025-11-04

### Roadmaps & Planning
- [Comprehensive Roadmap](./roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md) - v1.0 → v2.0
- [Query Service Roadmap](./roadmaps/query-service-roadmap.md) - ⏰ 2025-11-04
- [MCP v2 Enhancements](./roadmaps/mcp-v2-enhancements.md) - ⏰ 2025-10-24
- [Phase 1.5 Progress](./roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md) - Clean Architecture refactoring
- [Phase 1.5 TDD Results](./roadmaps/2025-10-22_PHASE_1.5_TDD_RESULTS.md) - Implementation results

### Guides
- [Quick Start](./guides/QUICK_START.md) - Quick start guide
- [Demo](./guides/DEMO.md) - Demo walkthrough
- [MCP Server Setup](./guides/mcp-server/CLAUDE_DESKTOP_SETUP.md) - Claude Desktop integration
- [MCP Demo Script](./guides/mcp-server/DEMO_SCRIPT.md) - MCP demo walkthrough
- [MCP Quick Reference](./guides/mcp-server/QUICK_REFERENCE.md) - MCP tool reference

### Testing
- [Control Plane Coverage Plan](./testing/control-plane-coverage-plan.md) - ⏰ 2025-11-04

---

## 📦 App-Specific Documentation

### Rust Core (`apps/core`)
- [README](../apps/core/README.md) - Core overview and quick start
- [Changelog](../apps/core/docs/CHANGELOG.md) - Version history
- [Features](../apps/core/docs/FEATURES.md) - Comprehensive feature showcase
- [Security](../apps/core/docs/SECURITY.md) - Security documentation

### Go Control Plane (`apps/control-plane`)
- [README](../apps/control-plane/README.md) - Control plane overview

### Elixir Query Service (`apps/query-service`)
- [README](../apps/query-service/README.md) - Query service overview
- [Roadmap](./roadmaps/query-service-roadmap.md) - Development roadmap (centralized)

### MCP Server (`apps/mcp-server`)
- [README](../apps/mcp-server/README.md) - MCP server overview
- [Setup Guide](./guides/mcp-server/CLAUDE_DESKTOP_SETUP.md) - Integration guide
- [Demo Script](./guides/mcp-server/DEMO_SCRIPT.md) - Demo walkthrough
- [Quick Reference](./guides/mcp-server/QUICK_REFERENCE.md) - Tool reference

### Web App (`apps/web`)
- [README](../apps/web/README.md) - Web app overview

---

## 🗄️ Archived Documentation

Historical documentation is kept in `/docs/archive/` with timestamps:

### v1.0 Documentation (October 2025)
- [2025-10-21_V1_COMPLETE.md](./archive/2025-10-21_V1_COMPLETE.md) - ⚠️ DEPRECATED
- [2025-10-21_FINAL_ASSESSMENT.md](./archive/2025-10-21_FINAL_ASSESSMENT.md) - ⚠️ DEPRECATED
- [2025-10-21_HONEST_V1_STATUS.md](./archive/2025-10-21_HONEST_V1_STATUS.md) - ⚠️ DEPRECATED
- [2025-10-21_V1_STATUS.md](./archive/2025-10-21_V1_STATUS.md) - ⚠️ DEPRECATED
- [2025-10-20_V1_ROADMAP.md](./archive/2025-10-20_V1_ROADMAP.md) - ⚠️ SUPERSEDED by Comprehensive Roadmap

### Pre-v1.0 Documentation
- [2025-10-20_METRICS_IMPLEMENTATION.md](./archive/2025-10-20_METRICS_IMPLEMENTATION.md) - ⚠️ DEPRECATED
- [2025-10-20_PROMETHEUS_METRICS.md](./archive/2025-10-20_PROMETHEUS_METRICS.md) - ⚠️ DEPRECATED
- [2025-10-21_PERFORMANCE_REPORT.md](./archive/2025-10-21_PERFORMANCE_REPORT.md) - ⚠️ SUPERSEDED by PERFORMANCE.md

---

## 🏷️ Documentation Conventions

### Timestamps
All timestamped documentation uses format: `YYYY-MM-DD_FILENAME.md`

Example: `2025-10-22_PHASE_1.5_RESULTS.md`

### Status Markers
- ✅ **CURRENT** - Active, up-to-date documentation
- ⚠️ **DEPRECATED** - No longer accurate, kept for history
- 🔄 **SUPERSEDED** - Replaced by newer document (link provided)
- 📝 **DRAFT** - Work in progress

### Linking
Always use relative paths:
```markdown
[Architecture Guide](./current/CLEAN_ARCHITECTURE.md)
[Service Docs](../services/core/docs/README.md)
```

---

## 🔍 Finding Documentation

### By Topic
- **Architecture**: `/docs/architecture/` or `/docs/current/`
- **How-To**: `/docs/guides/`
- **Roadmaps**: `/docs/roadmaps/`
- **Historical**: `/docs/archive/`

### By Service
- **Rust Core**: `/services/core/docs/`
- **Go Control Plane**: `/services/control-plane/docs/`

### By Date
All timestamped files are prefixed with `YYYY-MM-DD_`

---

## 📝 Contributing Documentation

### Creating New Documentation
1. Determine type (guide, architecture, operations)
2. Place in appropriate directory
3. Add timestamp if appropriate
4. Update this INDEX.md
5. Add status marker (CURRENT, DRAFT, etc.)

### Deprecating Documentation
1. Move to `/docs/archive/` with timestamp prefix
2. Add deprecation marker to title
3. Update INDEX.md
4. Add link to replacement document if applicable

### Updating Documentation
1. Update the document
2. Update "Last Updated" timestamp in document
3. If major changes, consider creating new timestamped version

---

## 🔗 External Resources

- [GitHub Repository](https://github.com/allsource/chronos-monorepo)
- [Issue Tracker](https://github.com/allsource/chronos-monorepo/issues)
- [Wiki](https://github.com/allsource/chronos-monorepo/wiki)

---

## 📧 Documentation Maintainers

For questions or suggestions about documentation:
- Create an issue with `[docs]` prefix
- Tag @allsource-team

---

**Navigation**: [Home](../README.md) | [Architecture](./current/) | [Guides](./guides/) | [Roadmaps](./roadmaps/) | [Archive](./archive/)
