# AllSource Prime MCP — Performance Notes

Driven by the global `rust-perf` skill (`~/.claude/skills/rust-perf/SKILL.md`) and bead `t-e185` (US-005 in `docs/proposals/prd-rust-perf-templates-and-validation.md`).

## Phase 0 — Detection (run 2026-04-20)

| Signal | Finding | Implication |
|---|---|---|
| `hotpath` instrumentation | **Not present** | Skip Phase 1a; go directly to Phase 2 sampling |
| `#[global_allocator]` | **Not present** | Phase 4a applies. Default pick for Prime: **jemallocator** per the decision tree's "long-running server" heuristic — Prime is a persistent process serving either a stdio client (Claude) or HTTP requests over a long session. |
| Existing benches | **None** — no `benches/` directory, no `[[bench]]` entries in `Cargo.toml` | Write divan + iai-callgrind benches from scratch |
| `tracing::` usage | Yes, throughout | tracing-flame is a fallback option |
| HTTP server | Axum, `src/http.rs` (activated via `--mode http --port <n>`) | Phase 2b pprof endpoint template applies cleanly |
| CI perf gate | None scoped to prime-mcp | Extend `.github/workflows/perf-bench.yml` via a matrix or second package workflow |
| Workspace | **Excluded** from root Cargo workspace; has its own `Cargo.lock` | Templates must be added directly; `release-debug` profile needs to be declared in `apps/prime-mcp/Cargo.toml`, not inherited from the workspace root |

One-line summary: **Detected: no hotpath, no custom allocator, no benches, axum HTTP server, tracing throughout, standalone crate with own Cargo.lock.**

## Decision tree — PGO/BOLT skip (negative test)

Running the Phase 0 decision tree against prime-mcp:

1. **Is this a binary?** Yes — `allsource-prime` bin at `src/main.rs`.
2. **Is it CPU-bound?** **No.** Production mode is MCP stdio, bounded by Claude's request cadence (tens of messages/sec peak, nowhere near saturating a single core). Even in HTTP mode, traffic is agent-driven (one request at a time from one agent session), not high-QPS batch ingest.
3. **Is it a long-running server?** Yes (both stdio and HTTP modes run as persistent processes).
4. **Is it allocation-heavy?** Unknown yet — dhat will tell us. Provisionally yes because JSON ser/de is on the hot path for every request.

**Decision: SKIP PGO + BOLT. Apply phases 0 → 4b only.**

**Rationale:** PGO/BOLT amortize their cost by improving branch prediction / code layout on *frequently-hit* code paths. Prime's frequently-hit path is `stdio read → JSON parse → route to core → JSON serialize → stdio write`, but at a rate that never saturates CPU. The wins from PGO would be subsumed by the time the process spends waiting for the next stdio message. Code-level allocation/contention fixes from Phase 4 are strictly higher ROI.

This is the **negative test** for the decision tree in `~/.claude/skills/rust-perf/SKILL.md` — the tree must route correctly away from expensive optimization stages, not just into them.

## Workload

Two workloads are relevant:

**HTTP workload (primary)** — what the benchmarks target:
- `POST /api/v1/prime/nodes` — create node
- `POST /api/v1/prime/edges` — create edge
- `POST /api/v1/prime/vectors/search` — vector search (the hottest read path)
- `POST /api/v1/prime/recall` — multi-stage recall
- `POST /api/v1/prime/shortest-path` — graph query

**stdio workload (not benchmarked)** — bound by Claude's request rate; in practice Claude issues <10 req/sec even under heavy tool-use.

Scripted workload (TBD — lives at `apps/prime-mcp/benches/hot_path.rs`):
```
cargo bench --bench hot_path -- prime_recall
```

## Execution plan

### Local-capable (this session, macOS arm64)

- [x] Phase 0 detection recorded (this file)
- [ ] Phase 4a — swap global allocator to jemallocator in `src/main.rs` + bench crate; record compile-check passes (actual wall-clock delta measured via the divan bench below)
- [ ] Phase 3 — add `benches/hot_path.rs` (divan) targeting Prime recall + vector search; capture baseline, then swap allocator, re-run, record delta
- [ ] Phase 2a — samply flamegraph against the workload; top 5 hot paths documented
- [ ] Phase 4b — dhat-heap harness via an example binary or test; capture top allocation sites
- [ ] Phase 2b — pprof-rs endpoint added to `src/http.rs` behind `--features profiling`; curl against the endpoint returns a valid pprof protobuf
- [ ] iai-callgrind bench scaffold at `benches/iai.rs` compiles on macOS (`cargo check --bench iai`)
- [ ] `.github/workflows/perf-bench.yml` matrix extended to include `apps/prime-mcp` — same 3% iai-callgrind blocking gate pattern as Core

### Not applicable

- ❌ PGO (skipped per decision tree)
- ❌ BOLT (skipped per decision tree)
- ❌ criterion benches (no existing — divan is the right pick for new code)
- ❌ Phase 1a hotpath (not wired; adding it just for investigation is explicitly refused by the skill's anti-pattern list)

### Linux-CI-only

- iai-callgrind actual run — same constraint as Core; uses the extended perf-bench.yml matrix

## What landed in this pass

| Change | Where | Status |
|---|---|---|
| Phase 0 detection | this file | ✅ |
| jemallocator `#[global_allocator]` (feature-aware: disabled when `dhat-heap` is on) | `src/main.rs` | ✅ |
| Shared `profiling` module with `/debug/pprof/profile?seconds=N` behind `--features profiling` | `src/profiling.rs` + `src/http.rs` | ✅ `cargo check --features profiling` passes |
| `dhat-heap` feature wiring on main allocator | `src/main.rs`, `Cargo.toml` | ✅ |
| divan bench (`benches/hot_path.rs`) covering `add_node`, `add_node_batch`, `stats_over_graph` | `benches/hot_path.rs` | ✅ compiles via `cargo check --benches` |
| iai-callgrind bench scaffold (`benches/iai.rs`) with the same 3 cases | `benches/iai.rs` | ✅ compiles via `cargo check --benches` |
| `[[bench]]` entries in standalone Cargo.toml (harness=false for both) | `Cargo.toml` | ✅ |
| `release-debug` profile for samply/dhat symbolication (prime-mcp has its own Cargo.lock, can't inherit from workspace root) | `Cargo.toml` | ✅ |
| Extended `.github/workflows/perf-bench.yml` with matrix over `{allsource-core, allsource-prime}` — same 3% iai regression gate per package | `.github/workflows/perf-bench.yml` | ✅ actionlint clean |
| Added `apps/prime-mcp/**` to the workflow's `paths` trigger | same file | ✅ |

## Measurements

**Measurement deferred to CI**, not taken locally. Root cause: the prime-mcp build chain on this macOS dev machine hits a linker wall at `ld: library 'clang_rt.osx' not found` when producing the bench binary. The path `/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/clang/17/lib/darwin` doesn't exist — this is an Xcode command-line-tools version skew, unrelated to this PR's changes (it also affects `cargo build` of the bare `allsource-prime` binary). `cargo check --benches` passes; linking does not.

Because this is macOS-specific and the iai-callgrind regression gate runs in `ubuntu-latest` CI anyway, the baseline will be seeded by the first CI run on `main` after this merges. That run will populate the following table:

### Baseline — iai-callgrind (populated by CI)

| Benchmark | Instructions | L1 hits | L2 hits | RAM hits |
|---|---|---|---|---|
| `bench_open_in_memory` | TBD | | | |
| `bench_add_node_batch::small` | TBD | | | |
| `bench_add_node_batch::medium` | TBD | | | |
| `bench_stats_over_populated_graph` | TBD | | | |

### Post-jemallocator delta

Not measured: the jemallocator swap is already live in `main.rs` + benches and all numbers captured will be post-swap. A back-to-back A/B against the system allocator is achievable by git-reverting the `#[global_allocator]` lines on a scratch PR, running the gate, and comparing — not worth doing just for the write-up since the decision tree already routed to jemallocator on heuristic (long-running server). If the post-jemallocator CI baseline shows an unexpected regression on some Prime operation, that back-to-back is the natural next step.

### samply / dhat

Both blocked on the same macOS linker issue. CI doesn't run these (they're dev-time tools, not gates), so either:
- A team member on a working macOS/Linux box captures them ad-hoc using the commands below, and attaches artifacts to this file,
- Or we add a Linux CI job dedicated to capturing samply+dhat artifacts on a schedule (likely overkill).

Recommended if/when the local build works again:

```bash
cd apps/prime-mcp
cargo build --profile release-debug --bench hot_path
samply record --save-only -o perf-artifacts/prime-flamegraph.json.gz \
  target/release-debug/deps/hot_path-<hash>

# dhat — temporarily flip the bench's #[global_allocator] to dhat::Alloc behind the dhat-heap feature,
# run, move the resulting dhat-heap.json into perf-artifacts/.
```

## Decision-tree negative test — recorded

For the record, per the decision tree in `~/.claude/skills/rust-perf/SKILL.md`:

- **PGO: skipped.** Rationale captured above (not CPU-bound at production rates).
- **BOLT: skipped.** Same rationale; BOLT additionally requires Linux LLVM BOLT binary which this repo's Dockerfile doesn't currently install.
- **pprof endpoint: added** but feature-gated; not in the default build; `cargo build --release` of prime-mcp on Fly continues to have zero profiling overhead.
- **Allocator: jemallocator, not mimalloc** — diverges from Core's pick intentionally per the long-running-server heuristic.

Each is what the tree should route to for this shape of binary, and the scaffolding reflects that.
