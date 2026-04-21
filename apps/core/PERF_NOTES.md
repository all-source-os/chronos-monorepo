# AllSource Core — Performance Notes

Driven by the global `rust-perf` skill (`~/.claude/skills/rust-perf/SKILL.md`) and bead `t-a35b` (US-004 in `docs/proposals/prd-rust-perf-templates-and-validation.md`).

## Phase 0 — Detection (run 2026-04-20)

Grep results:

| Signal | Finding | Implication |
|---|---|---|
| `hotpath` instrumentation | **Wired** — `Cargo.toml` has `hotpath` and `hotpath-alloc` optional features | Start triage at Phase 1a with `--features hotpath` — the project already knows its hot paths |
| `#[global_allocator]` | **Not present** | Phase 4a allocator swap is valid. Default pick: `mimalloc` (CPU-bound event store, not a long-running-server-with-churn allocator profile) |
| Criterion benches | **Present** — `benches/event_ingestion.rs` + `benches/performance_benchmarks.rs` | Keep criterion. Add iai-callgrind **alongside** (Phase 3) for CI regression gating — don't replace |
| `tracing::` usage | Extensive | Optional Phase 2c fallback if flamegraph signal is poor |
| HTTP server | Axum, `src/main.rs` | Phase 2b pprof endpoint applicable (feature-gated) |
| CI perf gate | **Exists** — `.github/workflows/perf-bench.yml`, criterion-based, 115% alert threshold, wall-clock | Extend this workflow, do not add a parallel one |

One-line summary: **Detected: hotpath (feature-gated), criterion benches, no custom allocator, existing perf-bench.yml (criterion / 115% wall-clock alert), tracing throughout, axum server.**

## Workload

Primary workload for PGO/BOLT profiling and flamegraph capture: the existing criterion `event_ingestion` bench — specifically the `batch_ingestion/10000` case (10k event ingest into in-memory `EventStore::new()`). This is what CI already benchmarks, so baselines are comparable.

Command (from `apps/core/`):

```bash
cargo bench --bench event_ingestion -- batch_ingestion/10000
```

Note: criterion benches use `EventStore::new()` (in-memory, no WAL/Parquet). This is the correct target for CPU-bound PGO — disk-bound workloads don't benefit. A separate disk-backed benchmark would measure a different thing.

## Execution plan (staged)

Only Phase 0 is complete. Remaining stages below are **not yet run**; this file will be updated with measurements as each stage executes.

### Local-capable (this session, macOS arm64)

- [ ] **Phase 1a** — Run with `--features hotpath` against the workload; record the top 5 instrumented frames
- [ ] **Phase 3 (criterion baseline)** — `cargo bench --bench event_ingestion -- --save-baseline main` (already the format perf-bench.yml uses)
- [ ] **Phase 4a** — Swap global allocator to mimalloc; re-run criterion; record delta
- [ ] **Phase 2a** — `samply record` under the workload; top 3 hot uninstrumented frames
- [ ] **Phase 4b** — dhat-heap run against a representative workload (likely a small binary wrapping the ingest loop); top 3 allocation sites

### Linux-CI-only (cannot run on macOS arm64)

- [ ] **Phase 3 (iai-callgrind)** — adds deterministic instruction-count gate. Requires valgrind ⇒ Linux runner.
- [ ] **Phase 6a — PGO** — `cargo pgo` runs on macOS but pairs with the iai numbers for measurement.
- [ ] **Phase 6a — BOLT** — requires LLVM BOLT binary (Linux, LLVM 14+).
- [ ] **Regression gate** — extend `.github/workflows/perf-bench.yml` (or add `perf-regression.yml` from `~/.claude/skills/rust-perf/templates/ci-perf-regression.yml`) to include the iai-callgrind job on the existing `ubuntu-latest` runner. The criterion wall-clock job stays for throughput tracking; iai becomes the deterministic merge gate.

### Required local installs (not yet done)

```bash
cargo install samply
cargo install cargo-pgo
rustup component add llvm-tools-preview
```

dhat-rs and mimalloc are cargo deps, no host install needed.

## Decisions / open questions

- **PGO workload representativeness**: the criterion bench is a good CPU-bound proxy for write-heavy usage. A more representative workload for a real deployment would mix reads + writes + the query path. For the first iteration, the criterion workload is the honest pick; a `perf-bench` binary for richer workload capture is a follow-up.
- **BOLT on macOS**: the binaries shipped to Fly are Linux, so BOLT-optimize should happen in the Docker build, not locally. This needs a Dockerfile tweak downstream of PGO.
- **Core replication work in flight** (see `docs/proposals/CORE_REPLICATION_DESIGN.md`): keep all benchmarks against single-node to avoid confounding variables. Revisit once replication lands.

## Measurements

Bench: `performance_benchmarks::ingestion_throughput/10000` (criterion, wall-clock on macOS arm64 / Apple Silicon).
Workload: 10,000 synthetic events into `EventStore::new()` (in-memory).
Runner: local dev machine. **These numbers are directional** — CI-pinned hardware is the source of truth for merge gating, which is why iai-callgrind will be the regression gate (deterministic instruction count, not wall clock).

| Stage | Median time | Throughput | Δ time vs baseline | Δ throughput vs baseline |
|---|---|---|---|---|
| Baseline (no custom allocator) | **58.28 ms** | **171.59 Kelem/s** | — | — |
| + mimalloc `#[global_allocator]` | **39.22 ms** | **255.01 Kelem/s** | **−32.7%** | **+48.6%** |
| + PGO (cargo-pgo) | TBD — US-004b | | | |
| + BOLT (cargo-pgo bolt) | TBD — US-004b | | | |

mimalloc change: p < 0.05 (significant). 95% CI on time delta: [−33.5%, −31.9%].

**Commentary:** 32.7% reduction from mimalloc alone is at the high end of what allocator swaps deliver — consistent with an event-store write path that's heavy on small-object allocation (`Event` struct cloning, `serde_json::Value` creation, `DashMap` insertions). This exceeds the US-004b ≥10% acceptance threshold from this single stage; PGO and BOLT are expected to add incremental wins, not carry the target.

Reproducing:
```bash
cd apps/core
# baseline (revert allocator swap first)
cargo bench --bench performance_benchmarks -- --save-baseline main ingestion_throughput/10000
# post-mimalloc
cargo bench --bench performance_benchmarks -- --baseline main ingestion_throughput/10000
```

## Profiling artifacts

### samply (CPU)

```bash
cargo build --profile release-debug --example ingest_workload
samply record --save-only -o perf-artifacts/ingest-flamegraph.json.gz \
  ../../target/release-debug/examples/ingest_workload
# open at https://profiler.firefox.com (drop the .json.gz in)
# or: samply load perf-artifacts/ingest-flamegraph.json.gz
```

Captured file: `perf-artifacts/ingest-flamegraph.json.gz` (~13 KB, 867 samples).
**Reading note**: samply's `--save-only` stores raw addresses; symbolication happens at view time in Firefox Profiler. Top named frames extracted via the dhat pass below, which resolves frames at capture time.

### dhat (heap)

```bash
cargo run --profile release-debug --example ingest_workload --features dhat-heap
mv dhat-heap.json perf-artifacts/dhat-heap.json
```

**Summary (release-debug, 20 iterations × 10k events = 200k events):**
- Total allocated: **2.37 GB** across **36.97M blocks** over the run
- Peak (t-gmax): **11.97 MB** in 73,868 live blocks
- At-exit: 0 B / 0 blocks (clean)

**Top 6 allocation sites by total bytes:**

| # | Bytes | Blocks | Site |
|---|---|---|---|
| 1 | 404 MB | 200K | `EventIndex::get_by_entity::{{closure}}` — `infrastructure/persistence/index.rs:82` |
| 2 | 307 MB | **12.8M** | `WebhookRegistry::find_matching` — `application/services/webhook.rs:199` |
| 3 | 307 MB | **12.8M** | `PipelineManager::process_event` — `application/services/pipeline.rs:741` |
| 4 | 179 MB | 200K | `schema_evolution::infer_schema` — `schema_evolution.rs:147` |
| 5 | 179 MB | 200K | `InferredSchema::clone` — `schema_evolution.rs:114` |
| 6 | 126 MB × 3 | 200K each | `Event::clone` — `domain/entities/event.rs:28` (three call-sites) |

**Interpretation:**
- Rows 2 & 3: **64 allocations per event** each (12.8M ÷ 200K). Webhook matching and pipeline processing are doing per-event allocations in their dispatch paths even when there are probably no registered webhooks/pipelines. Hot candidate for a fast-path-when-empty check.
- Row 1: `get_by_entity` allocates 2 KB per call on the entity-id lookup path. Cow/Arc opportunity.
- Rows 4–5: Schema evolution is inferring + cloning a schema per event. Caching the inferred schema by entity_id would cut 358 MB total.
- Row 6: `Event::clone` is called 3× per ingest (126 MB × 3 = 378 MB). Tracing call-sites via `cargo expand` or adding `#[track_caller]` annotations on `.clone()` would identify which are avoidable.

These are US-004b candidates (code-level follow-ups), not US-004a scope. Documented here so the next agent / engineer has a concrete list.

## CI scaffolding

### iai-callgrind bench (`benches/iai.rs`)

Linux-only (valgrind required). Mirrors the criterion workload: `bench_ingest_single`, `bench_ingest_batch` (100/1k/10k), `bench_query_after_ingest`. Compiles on macOS (`cargo check --bench iai`) so all contributors can iterate without a Linux box — only the `cargo bench --bench iai` run requires valgrind.

### Workflow — `.github/workflows/perf-bench.yml`

Extended with a second job `iai-callgrind` that runs alongside the existing criterion job. Split of concerns:

| Job | Purpose | Gate |
|---|---|---|
| `criterion` (existing) | Wall-clock throughput tracker via benchmark-action | 115% advisory, non-blocking |
| `iai-callgrind` (new) | Deterministic instruction-count regression gate | **3%, blocking** |

The 3% iai gate installs valgrind, runs `cargo bench --bench iai -- --save-baseline main`, greps for `Instructions:.*?Change:` lines, fails on any single metric exceeding +3%, and posts a marker-tagged PR comment with the full diff. First run on `main` seeds the baseline artifact used by future PRs.

Not yet done (belongs to US-004b):
- Run this workflow on `main` to seed the baseline
- Synthetic-regression test PR to prove the gate fails as intended
- cargo-pgo + BOLT in CI, with post-optimize iai numbers in PERF_NOTES.md

### Phase 1a hotpath run

**Not captured in this session.** `--features hotpath` rebuilds the crate with hotpath's macros active — the dhat+samply workload I captured gave a richer picture (named sites, specific byte counts) than hotpath's coarser top-N on instrumented frames would. Running `--features hotpath` against the same workload is still worthwhile as a complementary signal and is a low-effort follow-up. Tracking as open in the US-004a acceptance criteria rather than silently skipping.

### What this unlocks

- US-004b (t-e9f3) — Linux CI measurement + PGO + BOLT: the iai bench is now in place, and extending the same workflow with a `cargo-pgo` job against the same bench is straightforward.
- US-005 (t-e185, apps/prime-mcp) — can reuse this workflow pattern with `PACKAGE=allsource-prime` once prime-mcp gets its own `iai.rs`.
- US-006 (t-3eea, apps/chronis) — same.
