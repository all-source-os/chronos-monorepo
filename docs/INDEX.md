# AllSource Documentation Index

**Last Updated**: 2025-10-22
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
├── architecture/     # Architecture Decision Records (ADRs)
├── operations/       # Operational guides (deployment, monitoring)
└── roadmaps/         # Product roadmaps and planning
```

### Service Documentation
Each service has its own `/docs` directory:
```
services/[service]/docs/
├── architecture/     # Service-specific architecture
├── guides/          # Service-specific guides
├── api/             # API documentation
└── changelog/       # Service changelog
```

---

## 📋 Current Documentation

### Architecture & Design
- [Clean Architecture Guide](./current/CLEAN_ARCHITECTURE.md) - ⏰ 2025-10-22
- [SOLID Principles Guide](./current/SOLID_PRINCIPLES.md) - ⏰ 2025-10-22
- [Performance Optimization Guide](./current/PERFORMANCE.md) - ⏰ 2025-10-22

### Roadmaps & Planning
- [Comprehensive Roadmap](./roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md) - v1.0 → v2.0
- [Phase 1.5 Progress](./roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md) - Clean Architecture refactoring
- [Phase 1.5 TDD Results](./roadmaps/2025-10-22_PHASE_1.5_TDD_RESULTS.md) - Implementation results

### Operational Guides
- [README](../README.md) - Project overview
- [Getting Started](./guides/GETTING_STARTED.md) - Quick start guide

---

## 📦 Service-Specific Documentation

### Rust Core (`services/core`)
- [Architecture Overview](../services/core/docs/architecture/README.md)
- [API Reference](../services/core/docs/api/README.md)
- [Development Guide](../services/core/docs/guides/DEVELOPMENT.md)

### Go Control Plane (`services/control-plane`)
- [Architecture Overview](../services/control-plane/docs/architecture/README.md)
- [API Reference](../services/control-plane/docs/api/README.md)
- [Development Guide](../services/control-plane/docs/guides/DEVELOPMENT.md)

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
