# Go Control Plane Service Documentation

**Service**: control-plane
**Language**: Go
**Last Updated**: 2025-10-22

---

## 📚 Documentation Index

### Architecture
- [Architecture Overview](./architecture/OVERVIEW.md) - ⏳ PLANNED
- [Clean Architecture Migration](./architecture/CLEAN_ARCHITECTURE_MIGRATION.md) - ⏳ PLANNED

### Development Guides
- [Getting Started](./guides/GETTING_STARTED.md)
- [Testing Guide](./guides/TESTING.md)
- [Contributing](./guides/CONTRIBUTING.md)

### API Reference
- [REST API](./api/REST_API.md)
- [Authentication](./api/AUTH.md)
- [Tenancy](./api/TENANCY.md)

### Changelog
- [Service Changelog](./changelog/CHANGELOG.md)

---

## 🏗️ Current Architecture

```
[Current Structure - To be refactored in Phase 1.5]

Planned Structure (Weeks 6-8):
.
├── internal/
│   ├── domain/         ⏳ Layer 1: Business entities
│   ├── application/    ⏳ Layer 2: Use cases
│   ├── infrastructure/ ⏳ Layer 3: Repositories, adapters
│   └── interfaces/     ⏳ Layer 4: HTTP handlers, gRPC
```

---

## 🧪 Testing

```bash
# Run all tests
go test ./...

# Run with coverage
go test ./... -cover

# Run specific tests
go test -v -run TestPolicyEngine
```

**Current Status**: ✅ All tests passing, 23.2% coverage

---

## 🔧 Recent Fixes

### Policy Engine Fix (2025-10-22)
- **Issue**: Policy evaluated user's tenant instead of target tenant
- **Fix**: Updated `evaluateCondition()` to check `Attributes["tenant_id"]`
- **File**: `policy.go:290-310`
- **Impact**: All policy tests now passing

---

## 📅 Refactoring Roadmap

### Phase 1.5 (Weeks 6-8) - ⏳ PLANNED
- [ ] Create `internal/domain` structure
- [ ] Implement use case layer
- [ ] Apply SOLID principles
- [ ] Dependency injection with Wire
- [ ] Migrate handlers to use cases

See: [Phase 1.5 Progress](../../../docs/roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md)

---

## 📖 Related Documentation

- [Project Documentation](../../../docs/INDEX.md)
- [Rust Core Docs](../../core/docs/README.md)
- [SOLID Principles](../../../docs/current/SOLID_PRINCIPLES.md)

---

**Navigation**: [Home](../../../README.md) | [Architecture](./architecture/) | [Guides](./guides/) | [API](./api/)
