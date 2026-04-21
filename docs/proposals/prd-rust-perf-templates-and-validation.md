# PRD: `rust-perf` skill — templates, CI workflow, and monorepo validation

## 1. Overview

The global `~/.claude/skills/rust-perf/SKILL.md` already delivers the staged profiling playbook (phases 0-6, tool selection, anti-patterns). This PRD covers only the **delta**: making the skill's inline code snippets available as runnable drop-in templates, adding a `justfile` target set and a reusable GitHub Actions workflow, and validating the full pipeline on the three Rust binaries in this monorepo (`apps/core/`, `apps/prime-mcp/`, `apps/chronis/`).

Templates and CI workflow extend the existing global skill. Validation work lives in this monorepo and records real measurements.

## 2. Goals

- `~/.claude/skills/rust-perf/templates/` contains drop-in files for allocator swap, divan bench, iai-callgrind bench, pprof endpoint, dhat harness, justfile, and GH Actions workflow
- A single reusable `.github/workflows/perf-regression.yml` gates all three monorepo apps via a matrix
- `apps/core/` end-to-end pipeline runs and produces ≥10% measured speedup on the ingest hot path
- `apps/prime-mcp/` pipeline runs through phases 0-4 and **correctly skips** PGO/BOLT (decision-tree negative test)
- `apps/chronis/` pipeline runs through phases 0-4 + PGO (CLI is CPU-bound), without the pprof endpoint
- Every template compiles standalone in a throwaway `cargo new` project

## 3. Quality Gates

### Epic-Level (run once on epic completion)

- `cargo test --workspace` — all unit tests pass
- `cargo clippy --workspace -- -D warnings` — zero clippy warnings
- `cargo fmt --check` — formatting clean

### Story-Level (checked per story)

- **Template stories:** Each template must compile standalone in an isolated `cargo new` tmpdir
- **Workflow story:** GH Actions YAML parses via `actionlint` (or `serde_yaml` fallback)
- **Validation stories:** Before/after benchmark numbers recorded directly in the story's acceptance criteria; PERF_NOTES.md committed alongside

## 4. Testing Trophy

Per 5A — unit-only, focused on template correctness.

### Unit Tests

- [ ] Test: `templates/allocator-mimalloc.rs` + `Cargo.toml` snippet compile in tmpdir
- [ ] Test: `templates/bench-divan.rs` builds and `cargo bench -- --help` exits 0
- [ ] Test: `templates/bench-iai-callgrind.rs` builds on Linux (skip on macOS)
- [ ] Test: `templates/pprof-endpoint-axum.rs` compiles behind `--features profiling`
- [ ] Test: `templates/dhat-heap.rs` compiles behind `--features dhat-heap`
- [ ] Test: `templates/justfile-perf.just` parses via `just --list --justfile`
- [ ] Test: `templates/ci-perf-regression.yml` parses via `actionlint` or `serde_yaml`

### Block-Merge Critical

- iai-callgrind gate logic in the GH Actions template — a broken gate silently lets regressions through

## 5. User Stories

### US-001: Augment skill with drop-in templates [Schema]

**Description:** As a user, I want the skill's code snippets available as standalone files in `~/.claude/skills/rust-perf/templates/` so I can `cp` them into a target repo instead of re-transcribing.

**Acceptance Criteria:**
- [ ] Directory `~/.claude/skills/rust-perf/templates/` created
- [ ] File `templates/allocator-mimalloc.rs` with 5-line header explaining where to paste (`src/main.rs` or `src/lib.rs`) and which Cargo.toml dep to add
- [ ] File `templates/allocator-jemalloc.rs` (equivalent)
- [ ] File `templates/bench-divan.rs` — standalone `benches/template_divan.rs` with one example benchmark
- [ ] File `templates/bench-iai-callgrind.rs` — standalone `benches/template_iai.rs` with a `LibraryBenchmarkConfig` that fails on >3% regression
- [ ] File `templates/pprof-endpoint-axum.rs` — feature-gated on `profiling`, exposes `/debug/pprof/profile?seconds=30` returning pprof protobuf
- [ ] File `templates/dhat-heap.rs` — feature-gated on `dhat-heap`, writes `dhat-heap.json`
- [ ] Test: each template copied into `cargo new perf-tmp` + appropriate `Cargo.toml` snippet → `cargo check` exits 0 (record in `templates/README.md` as the verification recipe)
- [ ] SKILL.md phases 2-4 updated to reference the template paths alongside the existing inline snippets

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: justfile target set [Schema]

**Description:** As a user, I want `just perf-baseline`, `just perf-pgo`, `just perf-bolt`, `just perf-regression` as copy-paste recipes for the full pipeline.

**Acceptance Criteria:**
- [ ] File `~/.claude/skills/rust-perf/templates/justfile-perf.just` created
- [ ] Targets: `perf-baseline`, `perf-alloc-check`, `perf-flamegraph` (samply), `perf-heap` (dhat), `perf-pgo`, `perf-bolt`, `perf-regression` (iai-callgrind)
- [ ] PGO target wraps `cargo pgo build && cargo pgo run -- {{workload}} && cargo pgo optimize` (workload templated as justfile variable)
- [ ] BOLT target wraps `cargo pgo bolt build --with-pgo && cargo pgo bolt run -- {{workload}} && cargo pgo bolt optimize --with-pgo`
- [ ] Each target has a one-line doc comment (shown in `just --list`)
- [ ] Test: `just --list --justfile templates/justfile-perf.just` exits 0 and lists all 7 targets
- [ ] SKILL.md phase 6a references this file instead of re-listing commands inline

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Reusable GH Actions regression-gate workflow [Schema]

**Description:** As a user, I want a single GitHub Actions workflow that runs iai-callgrind on every PR, comments before/after instruction counts, and fails on >3% regression.

**Acceptance Criteria:**
- [ ] File `~/.claude/skills/rust-perf/templates/ci-perf-regression.yml` created
- [ ] Workflow installs valgrind on `ubuntu-latest`, runs `cargo bench --bench {{bench_name}} -- --save-baseline main` on main pushes, compares on PRs
- [ ] Fails the job on >3% instruction-count regression (threshold templated as workflow input)
- [ ] Comments the before/after diff on the PR via `actions/github-script` or similar
- [ ] Uses a matrix input so the same workflow can gate multiple binaries in a monorepo
- [ ] Test: YAML parses via `actionlint` (if installed) or `serde_yaml` round-trip
- [ ] SKILL.md phase 3 references this file for "wire into CI"

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Apply pipeline to apps/core/ [Integration]

**Description:** As a user, I want end-to-end validation that the skill+templates produce real speedups on Core, the one CPU-bound binary in the monorepo.

**Acceptance Criteria:**
- [ ] Phase 0 detection run and findings recorded in `apps/core/PERF_NOTES.md` (existing hotpath? allocator? criterion benches? CI gates?)
- [ ] Workload defined: the existing 100k-event ingest benchmark or equivalent, command recorded in PERF_NOTES.md
- [ ] Baseline recorded: iai-callgrind and divan results for ingest + query hot paths, committed as `apps/core/perf-baselines/main.json`
- [ ] Allocator swapped to mimalloc (Phase 4a), delta measured vs. baseline, recorded in PERF_NOTES.md
- [ ] samply flamegraph captured, top 3 hot functions documented in PERF_NOTES.md
- [ ] dhat heap report captured, top 3 allocation sites documented in PERF_NOTES.md
- [ ] `cargo pgo` optimize run against the workload, post-PGO iai-callgrind numbers recorded
- [ ] `cargo pgo bolt` run on top of PGO, post-BOLT numbers recorded
- [ ] Measured speedup ≥10% on the ingest hot path vs. baseline (or rationale documented if below threshold)
- [ ] `.github/workflows/perf-regression.yml` created from the template, gating `allsource-core` benches
- [ ] CI run on main green; synthetic-regression test PR demonstrates the gate fires

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Apply pipeline to apps/prime-mcp/ [Integration]

**Description:** As a user, I want to confirm the decision tree correctly identifies prime-mcp as stdio-dominated and skips PGO/BOLT (negative test for the tree).

**Acceptance Criteria:**
- [ ] Phase 0 detection run and findings recorded in `apps/prime-mcp/PERF_NOTES.md`
- [ ] Decision-tree output in PERF_NOTES.md explicitly skips PGO + BOLT with written rationale ("stdio-dominated, Claude request cadence is the bound")
- [ ] Workload defined: HTTP-mode synthetic load script (not stdio), command recorded
- [ ] Baseline: iai-callgrind + divan for Prime graph operations
- [ ] Allocator swapped to jemallocator (long-running server heuristic), delta measured and recorded
- [ ] samply flamegraph captured under synthetic HTTP load, hot paths documented
- [ ] dhat heap report captured
- [ ] pprof-rs endpoint added to the HTTP server behind `--features profiling`, wire confirmed (curl returns a valid pprof protobuf)
- [ ] `perf-regression.yml` matrix extended to include prime-mcp benches
- [ ] PGO/BOLT explicitly NOT applied — verified by a negative test in PERF_NOTES.md

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Apply pipeline to apps/chronis/ [Integration]

**Description:** As a user, I want to confirm the pipeline works on a short-lived CLI (different shape from a server) and that PGO helps CLIs.

**Acceptance Criteria:**
- [ ] Phase 0 detection run and findings recorded in `apps/chronis/PERF_NOTES.md`
- [ ] Decision-tree output includes PGO (CPU-bound single-shot execution) but excludes pprof-rs endpoint (not a server)
- [ ] Workload defined: representative 10k-event ingest script, command recorded
- [ ] Baseline: iai-callgrind + divan for the hottest CLI command paths
- [ ] Allocator swapped to mimalloc (short-lived, CPU-bound)
- [ ] samply flamegraph captured under the workload, hot paths documented
- [ ] `cargo pgo` applied, post-PGO speedup measured and recorded
- [ ] BOLT optional: applied if PGO shows >5% improvement, skipped otherwise (rationale recorded either way)
- [ ] `perf-regression.yml` matrix extended to include chronis benches
- [ ] Post-PGO speedup ≥5% on the benchmarked path, or rationale documented

Mark each item [x] as you complete it. Only close when all are checked.

## 6. Functional Requirements

- FR-1: Template files live under `~/.claude/skills/rust-perf/templates/` (augment existing global skill, don't replace)
- FR-2: No template references chronos-specific paths or names (grep for "chronos", "apps/core", "prime-mcp", "allsource" in the templates → zero matches)
- FR-3: One GH Actions workflow file gates all three monorepo apps via a matrix — no per-app copies
- FR-4: Each monorepo validation story produces a committed `PERF_NOTES.md` with before/after numbers and the exact workload command
- FR-5: The `perf-regression.yml` regression threshold is templated as a workflow input (default 3%), not hardcoded
- FR-6: All pprof/dhat instrumentation is feature-gated — production builds incur zero overhead

## 7. Non-Goals

- Replacing or rewriting any existing content in `SKILL.md` beyond cross-references to the new template files
- Adding new phases to the skill's existing 0-6 playbook
- Benchmarking non-Rust code (Query Service Elixir, frontend TS)
- Multi-host perf testing (single-node Core only, per CLAUDE.md)
- Auto-bisection or auto-tuning infrastructure
- Publishing the skill or templates to any public registry

## 8. Technical Considerations

- Templates must be standalone — no cross-references between template files
- `cargo-pgo` requires the `llvm-tools-preview` rustup component; SKILL.md already mentions this, templates shouldn't duplicate the advice
- iai-callgrind requires Linux; the GH Actions workflow must run on `ubuntu-latest`, macOS CI runners skip iai jobs
- prime-mcp has both stdio and HTTP modes — benchmark the HTTP path only (stdio is bounded by Claude's request cadence)
- Core's in-flight replication work (`docs/proposals/CORE_REPLICATION_DESIGN.md`) — US-004 benchmarks single-node Core to avoid confounding variables
- `PERF_NOTES.md` files live under each app (`apps/core/PERF_NOTES.md` etc.), not in `docs/`, so the numbers travel with the code that produced them

## 9. Success Metrics

- All 7 templates compile in throwaway `cargo new` projects
- `.github/workflows/perf-regression.yml` green on main, red on a synthetic 5%-regression PR
- `apps/core/` post-PGO speedup ≥10% on the ingest hot path
- Decision tree correctly routes prime-mcp away from PGO/BOLT (negative test recorded in PERF_NOTES.md)
- chronis post-PGO speedup ≥5%, or rationale documented

## 10. Open Questions

- Does `apps/chronis/` have a stable enough workload to make PGO worthwhile, given it's still evolving? (Leaning: attempt it; if variance swamps the signal, document as such and skip BOLT.)
- Should PERF_NOTES.md live in each app or be consolidated under `docs/perf/`? (Current: per-app, travels with code.)
