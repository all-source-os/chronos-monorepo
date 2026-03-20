# PRD: Core and Query-Service Production Readiness Review

## Overview
A comprehensive review of the AllSource monorepo's core (Rust) and query-service (Elixir) components to ensure production readiness. This review covers test quality, coverage improvements, PostgreSQL integration verification using testcontainers, and embedded mode functionality for the core component.

## Goals
- Achieve 90%+ test coverage for both core and query-service components
- Ensure all tests pass consistently with no flaky tests
- Verify PostgreSQL integration works correctly using testcontainers
- Confirm core component works properly in embedded mode (library API, in-memory storage, dependency usage)
- Remove unnecessary tests (duplicates, deprecated features, trivial tests)
- Run and verify existing performance benchmarks pass
- Fix all discovered issues directly in the codebase

## Quality Gates

These commands must pass for every user story:

**For Rust (core) stories:**
- `make quality-rust` - Runs tests, clippy, and formatting checks

**For Elixir (query-service) stories:**
- `make quality-elixir` - Runs tests, credo, and formatting checks

## User Stories

### US-001: Audit core component test coverage
As a developer, I want to analyze the current test coverage of the core Rust component so that I can identify gaps requiring additional tests.

**Acceptance Criteria:**
- [ ] Run `cargo tarpaulin` or equivalent coverage tool on core component
- [ ] Generate coverage report showing line and branch coverage
- [ ] Identify modules/functions below 90% coverage threshold
- [ ] Document coverage gaps in a checklist for subsequent stories

### US-002: Add missing tests for core component coverage gaps
As a developer, I want to add tests for uncovered critical paths so that the core component reaches 90%+ coverage.

**Acceptance Criteria:**
- [ ] Add unit tests for each identified coverage gap from US-001
- [ ] Ensure new tests cover edge cases and error paths
- [ ] Coverage report shows 90%+ line coverage after additions
- [ ] All new tests pass with `make quality-rust`

### US-003: Verify core PostgreSQL integration with testcontainers
As a developer, I want to verify that the core component's PostgreSQL integration works correctly so that database operations are reliable in production.

**Acceptance Criteria:**
- [ ] Review existing testcontainers setup in core component
- [ ] Ensure integration tests spin up real Postgres container
- [ ] Tests cover CRUD operations for all entity types (events, sessions, etc.)
- [ ] Tests verify connection pooling behavior
- [ ] Tests verify transaction handling and rollback scenarios
- [ ] All integration tests pass with `make quality-rust`

### US-004: Verify core embedded mode - Library API
As a developer, I want to verify the core library API works without starting the HTTP server so that it can be used as an embedded library.

**Acceptance Criteria:**
- [ ] Tests exist that use core as a library without HTTP server initialization
- [ ] API functions (create session, record event, query) work in library mode
- [ ] No HTTP-related code is required for library usage
- [ ] All embedded mode tests pass with `make quality-rust`

### US-005: Verify core embedded mode - In-memory storage
As a developer, I want to verify the in-memory storage mode functions correctly so that the core can run without external dependencies.

**Acceptance Criteria:**
- [ ] Tests exist for in-memory storage backend
- [ ] All CRUD operations work with in-memory storage
- [ ] Data persists correctly within a session
- [ ] Memory is properly cleaned up when storage is dropped
- [ ] All in-memory tests pass with `make quality-rust`

### US-006: Verify core embedded mode - Dependency usage
As a developer, I want to verify core can be used as a Cargo dependency so that other Rust projects can integrate it.

**Acceptance Criteria:**
- [ ] Verify `Cargo.toml` exposes appropriate public API
- [ ] Feature flags work correctly for optional functionality
- [ ] No unnecessary dependencies are required for basic usage
- [ ] Documentation exists for embedding core in other projects
- [ ] Example usage compiles and runs correctly

### US-007: Run and verify core performance benchmarks
As a developer, I want to run existing performance benchmarks so that I can verify the core meets performance expectations.

**Acceptance Criteria:**
- [ ] Identify existing benchmark files in core component
- [ ] Run all benchmarks using `cargo bench` or equivalent
- [ ] Document benchmark results (ops/sec, latency percentiles)
- [ ] Verify no benchmark shows regression from expected baselines
- [ ] All benchmarks complete without errors

### US-008: Audit query-service test quality
As a developer, I want to review the 242 existing tests in query-service so that I can identify tests to remove or improve.

**Acceptance Criteria:**
- [ ] Review all test files in query-service
- [ ] Identify duplicate tests covering same functionality
- [ ] Identify tests for deprecated or removed features
- [ ] Identify trivially obvious tests that add no value
- [ ] Create list of tests to remove with justification for each

### US-009: Remove unnecessary tests from query-service
As a developer, I want to remove unnecessary tests so that the test suite is focused and maintainable.

**Acceptance Criteria:**
- [ ] Remove duplicate tests identified in US-008
- [ ] Remove tests for deprecated features identified in US-008
- [ ] Remove trivial tests identified in US-008
- [ ] Verify remaining tests still provide adequate coverage
- [ ] All remaining tests pass with `make quality-elixir`

### US-010: Analyze query-service test coverage
As a developer, I want to analyze test coverage for query-service so that I can identify areas needing additional tests.

**Acceptance Criteria:**
- [ ] Run `mix test --cover` or excoveralls for coverage analysis
- [ ] Generate coverage report showing line coverage by module
- [ ] Identify modules/functions below 90% coverage threshold
- [ ] Document coverage gaps for subsequent stories

### US-011: Add missing tests for query-service coverage gaps
As a developer, I want to add tests for uncovered paths so that query-service reaches 90%+ coverage.

**Acceptance Criteria:**
- [ ] Add tests for each identified coverage gap from US-010
- [ ] Ensure new tests cover edge cases and error paths
- [ ] Coverage report shows 90%+ line coverage after additions
- [ ] All new tests pass with `make quality-elixir`

### US-012: Verify query-service PostgreSQL integration with testcontainers
As a developer, I want to verify that query-service's PostgreSQL integration works correctly so that database operations are reliable.

**Acceptance Criteria:**
- [ ] Review existing testcontainers setup in query-service
- [ ] Ensure integration tests spin up real Postgres container
- [ ] Tests cover all Ecto schema operations
- [ ] Tests verify connection pooling with Ecto
- [ ] Tests verify transaction handling and rollback
- [ ] Tests cover migration scenarios
- [ ] All integration tests pass with `make quality-elixir`

### US-013: Run and verify query-service performance benchmarks
As a developer, I want to run existing performance benchmarks for query-service so that I can verify it meets performance expectations.

**Acceptance Criteria:**
- [ ] Identify existing benchmark files in query-service
- [ ] Run all benchmarks using Benchee or equivalent
- [ ] Document benchmark results
- [ ] Verify no benchmark shows regression from expected baselines
- [ ] All benchmarks complete without errors

### US-014: Fix any discovered flaky tests
As a developer, I want to identify and fix any flaky tests discovered during the review so that the test suite is reliable.

**Acceptance Criteria:**
- [ ] Document any tests that fail intermittently during review
- [ ] Analyze root cause of each flaky test
- [ ] Fix flaky tests (timing issues, race conditions, test isolation)
- [ ] Verify fixed tests pass consistently (run 10+ times)
- [ ] All tests pass with respective quality gate commands

### US-015: Final production readiness verification
As a developer, I want to run a complete verification of both components so that I can confirm production readiness.

**Acceptance Criteria:**
- [ ] `make quality-rust` passes completely
- [ ] `make quality-elixir` passes completely
- [ ] Core coverage is 90%+
- [ ] Query-service coverage is 90%+
- [ ] All PostgreSQL integration tests pass for both components
- [ ] All embedded mode tests pass for core
- [ ] All benchmarks pass for both components
- [ ] No known flaky tests remain

## Functional Requirements

- FR-1: Core component must pass all unit and integration tests via `make quality-rust`
- FR-2: Query-service must pass all unit and integration tests via `make quality-elixir`
- FR-3: Both components must achieve 90%+ line coverage
- FR-4: PostgreSQL integration tests must use testcontainers (real Postgres in Docker)
- FR-5: Core must function correctly without HTTP server (embedded library mode)
- FR-6: Core must function correctly with in-memory storage backend
- FR-7: Core must be usable as a Cargo dependency in other Rust projects
- FR-8: All existing benchmarks must run and complete without errors
- FR-9: No flaky tests may remain in either component's test suite

## Non-Goals

- Creating new benchmark suites (only running existing ones)
- Reviewing the control-plane (Go) component (deferred to future PRD)
- Adding new features to core or query-service
- Refactoring code beyond what's needed to fix test issues
- Performance optimization (only verification of current performance)
- CI/CD pipeline changes

## Technical Considerations

- **Testcontainers:** Both Rust and Elixir use testcontainers for PostgreSQL integration tests. Ensure Docker is available in the test environment.
- **Coverage Tools:** Rust uses `cargo tarpaulin` for coverage; Elixir uses `mix test --cover` or `excoveralls`
- **Embedded Mode:** Core's embedded mode requires careful review of public API surface and feature flags in `Cargo.toml`
- **Test Isolation:** When removing/modifying tests, ensure remaining tests maintain proper isolation (no shared state bleeding between tests)

## Success Metrics

- 100% of tests pass in both components
- 90%+ line coverage achieved in both components
- All PostgreSQL integration tests pass using real containers
- All embedded mode scenarios verified working in core
- All existing benchmarks complete successfully
- Zero flaky tests remaining
- Test suite execution time does not significantly increase

## Open Questions

- Are there specific coverage exclusions needed (e.g., generated code, test utilities)?
- What are the expected baseline numbers for existing benchmarks?
- Should benchmark results be committed to the repo for future comparison?