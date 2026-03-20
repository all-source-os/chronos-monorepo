# PRD: GitHub Actions Pipeline Optimization

## Overview
Optimize the existing GitHub Actions CI/CD pipeline to reduce costs, improve feedback speed, and establish better job sequencing. The focus is on eliminating redundant steps, ensuring fast checks run before slow operations, and implementing a two-stage container build process (build-only → build+push).

## Goals
- Reduce CI costs by eliminating duplicate/redundant checks
- Fail fast by running quick checks before long-running operations
- Consolidate change detection using `workflow_run` from CI to container-ci
- Fix Elixir workflow caching (deps and PLT)
- Remove redundant Go linting tools (keep only golangci-lint)
- Implement two-stage container builds (build-only, then build+push)
- Move security checks to main branch + scheduled only
- Add workflow syntax validation using `act`
- Unify container push conditions across registries

## Quality Gates

These commands must pass for every user story:
- `act --list` - Validate workflow syntax
- `act -n <workflow>` - Dry-run workflow validation
- Manual review of workflow changes

## User Stories

### US-001: Remove redundant Go linting tools
As a maintainer, I want to remove duplicate linting steps so that CI runs faster without losing coverage.

**Acceptance Criteria:**
- [ ] Remove `go vet` step from `.github/workflows/ci.yml` (covered by golangci-lint)
- [ ] Remove `staticcheck` step from `.github/workflows/ci.yml` (covered by golangci-lint)
- [ ] Verify golangci-lint config includes equivalent checks
- [ ] Go lint job runs only golangci-lint

### US-002: Fix Elixir dependency caching order
As a maintainer, I want deps cached correctly so that builds don't re-download dependencies unnecessarily.

**Acceptance Criteria:**
- [ ] Move cache restore step before `mix deps.get` in `.github/workflows/ci.yml`
- [ ] Cache key includes `mix.lock` hash
- [ ] `mix deps.get` only downloads missing deps (cache hit scenario)

### US-003: Add Elixir PLT caching for Dialyzer
As a maintainer, I want Dialyzer PLT cached so that type checking doesn't rebuild PLT on every run.

**Acceptance Criteria:**
- [ ] Add cache step for `_build/*/*.plt` files
- [ ] Cache key based on Elixir version + OTP version
- [ ] PLT cache restores correctly on cache hit
- [ ] Dialyzer step skips PLT rebuild when cache hit

### US-004: Move security workflow to main branch and schedule only
As a maintainer, I want security scans to run only on main and scheduled so that PRs get faster feedback.

**Acceptance Criteria:**
- [ ] Update `.github/workflows/security.yml` triggers
- [ ] Remove `pull_request` trigger
- [ ] Keep `push` trigger for `main` branch only
- [ ] Keep `schedule` trigger for regular scanning
- [ ] PRs no longer trigger security workflow

### US-005: Convert container-ci to workflow_run trigger
As a maintainer, I want container builds to trigger after CI passes so that we don't waste compute on failed CI.

**Acceptance Criteria:**
- [ ] Update `.github/workflows/container-ci.yml` to use `workflow_run` trigger
- [ ] Trigger on `ci.yml` workflow completion
- [ ] Add condition to check if triggering workflow succeeded
- [ ] Skip all jobs if CI workflow failed
- [ ] Remove duplicate `dorny/paths-filter` (use CI outputs or re-detect)

### US-006: Implement two-stage container build (build-only first)
As a maintainer, I want a build-only stage before push so that all images build successfully before any are pushed.

**Acceptance Criteria:**
- [ ] Add `build-only` job that builds all changed containers without push
- [ ] `build-only` job runs in parallel for all changed services
- [ ] Add `build-and-push` job that depends on `build-only` success
- [ ] `build-and-push` only runs after all `build-only` jobs pass
- [ ] `build-and-push` only runs on main branch

### US-007: Unify container push conditions across registries
As a maintainer, I want consistent push logic for GHCR and DockerHub so that the workflow is simpler to maintain.

**Acceptance Criteria:**
- [ ] Both GHCR and DockerHub use identical push conditions
- [ ] Push only occurs on main branch
- [ ] Push only occurs after all builds succeed
- [ ] Remove conditional logic differences between registries

### US-008: Add workflow syntax validation with act
As a maintainer, I want workflow changes validated before merge so that broken workflows don't reach main.

**Acceptance Criteria:**
- [ ] Add new job or workflow to validate workflow file syntax
- [ ] Use `act --list` or `act -n` for dry-run validation
- [ ] Triggered when `.github/workflows/*.yml` files change
- [ ] Validation runs on PRs affecting workflow files
- [ ] Clear error messages on syntax failures

### US-009: Document required branch protection checks
As a maintainer, I want documentation of recommended branch protection settings so that the team can configure them correctly.

**Acceptance Criteria:**
- [ ] Add section in README or CONTRIBUTING.md
- [ ] List required status checks for branch protection
- [ ] Document which checks must pass before merge
- [ ] Include CI workflow as required check
- [ ] Include workflow validation as required check (when applicable)

### US-010: Define required status checks for branch protection
As a maintainer, I want required checks defined so that PRs cannot merge without CI passing.

**Acceptance Criteria:**
- [ ] Identify jobs that should be required checks
- [ ] Document check names in exact format GitHub expects
- [ ] CI workflow primary jobs are required
- [ ] Workflow validation is required when workflow files change

## Functional Requirements
- FR-1: Go CI must run golangci-lint only (no separate vet/staticcheck)
- FR-2: Elixir CI must restore cache before running `mix deps.get`
- FR-3: Elixir CI must cache Dialyzer PLT with Elixir+OTP version key
- FR-4: Security workflow must only trigger on main branch push and schedule
- FR-5: Container-ci must trigger via `workflow_run` after CI completes
- FR-6: Container-ci must skip all jobs if CI workflow failed
- FR-7: Container builds must complete successfully before any push occurs
- FR-8: Container push must only occur on main branch
- FR-9: GHCR and DockerHub must use identical push conditions
- FR-10: Workflow file changes must be validated with `act` before merge

## Non-Goals
- Adding new CI tools beyond `act` for validation
- Migrating to a different CI platform
- Adding new linting or security tools
- Changing the container build technology (Docker)
- Implementing parallel test execution within jobs
- Adding workflow visualization or dashboards
- Caching Docker layers (separate optimization)

## Technical Considerations
- `workflow_run` provides `conclusion` field to check success/failure
- `act` must be installed in the runner or use a container action
- PLT files can be ~100MB; cache storage should accommodate this
- Branch protection settings are configured in GitHub repo settings, not workflow files
- Changes should be made directly to main in small increments for safety

## Success Metrics
- CI runs complete faster (measure before/after)
- No duplicate linting steps execute
- Container builds don't start until CI passes
- No container pushes occur if any build fails
- Elixir deps cache shows hits on subsequent runs
- Dialyzer PLT cache shows hits when deps unchanged
- Security scans only appear on main branch runs
- All workflow syntax validated before merge

## Open Questions
- Should we add retry logic for flaky `act` validation?
- Is there a preferred `act` version or container image to standardize on?
- Should workflow validation be a separate workflow or a job in CI?