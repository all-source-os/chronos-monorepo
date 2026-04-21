# chronis — Performance Notes

Driven by the global `rust-perf` skill (`~/.claude/skills/rust-perf/SKILL.md`) and bead `t-3eea` (US-006 in `docs/proposals/prd-rust-perf-templates-and-validation.md`).

## Phase 0 — Detection (run 2026-04-20)

| Signal | Finding | Implication |
|---|---|---|
| `hotpath` instrumentation | **Wired** — `Cargo.toml` features `hotpath` + `hotpath-alloc`, `#[cfg_attr(feature = "hotpath", hotpath::main)]` on `main()` | Phase 1a applies — hotpath rankings are the first stop, but given chronis's short-lived nature, one-shot CLI runs under hotpath are the right pattern (not long-running) |
| `#[global_allocator]` | **Not present** | Phase 4a. Pick: **mimalloc** (CLI, short-lived, CPU-bound single-shot execution — different from prime-mcp where long-running server pushed us to jemallocator) |
| Existing benches | **None** — no `benches/` dir, no `[[bench]]` in `Cargo.toml` | Write divan + iai-callgrind from scratch, same as prime-mcp |
| HTTP server | **No** — `cn` is a CLI with optional `cn serve` TUI but production workload is one-shot commands | Phase 2b pprof endpoint **does not apply** |
| Workspace | **Excluded** from root workspace; own `Cargo.lock` | Same pattern as prime-mcp — add deps + `release-debug` profile directly |
| `tokio::main` | Yes | Bench harness needs `Runtime::new()` blocks (same pattern as prime-mcp benches) |

One-line summary: **Detected: hotpath wired (one-shot runner pattern), no custom allocator, no benches, not an HTTP server, standalone crate with own Cargo.lock.**

## Decision tree — PGO applies, BOLT conditional

Running the tree:

1. **Is this a binary?** Yes — `cn` at `src/main.rs`.
2. **Is it CPU-bound?** **Yes.** A single `cn` invocation parses arguments, opens the embedded AllSource store, runs a projection, and exits. No network wait, no idle time.
3. **Is it a long-running server?** **No** — short-lived, one-shot. Allocator choice: **mimalloc** (CPU-bound general workload).
4. **Is PGO workload representative?** **Yes.** A canonical training run is `cn init && for i in $(seq 100); do cn create "task $i"; done && cn list && cn show <id> && cn done <id>` — exercises init, event ingest, projection query, event close. Short-lived CLIs are actually an ideal PGO target because the training workload can exactly match production runs.

**Decision: Apply phases 0 → 4b + Phase 6a (PGO). BOLT conditional on PGO showing ≥5% improvement.**

**Decision on pprof-rs endpoint: SKIP.** Not a server; no long-running process to attach to.

**Decision on BOLT in Dockerfile: DEFER.** chronis ships as a `cargo install` / published binary. BOLT would require its own build pipeline; apply only if the PGO gain justifies it and a Linux binary-distribution step is added separately.

## Workload

PGO + iai-callgrind target: the canonical CLI command sequence above.

Scripted form (for the `cargo pgo run --` step and for future ad-hoc use):

```bash
# representative workload — kept deterministic so PGO training matches production
CHRONIS_WORKLOAD_DIR=$(mktemp -d)
cn --workspace "$CHRONIS_WORKLOAD_DIR" init
for i in $(seq 1 100); do
  cn --workspace "$CHRONIS_WORKLOAD_DIR" create "task $i"
done
cn --workspace "$CHRONIS_WORKLOAD_DIR" list
# pick a few and roundtrip
for id in $(cn --workspace "$CHRONIS_WORKLOAD_DIR" list --toon | tail -5 | cut -d'|' -f1); do
  cn --workspace "$CHRONIS_WORKLOAD_DIR" show "$id"
  cn --workspace "$CHRONIS_WORKLOAD_DIR" done "$id"
done
rm -rf "$CHRONIS_WORKLOAD_DIR"
```

For iai-callgrind the bench drives the same operations directly via chronis's library API (not the binary CLI) to avoid including argument-parsing variance.

## Execution plan

### Local-capable (this session, macOS arm64)

- [x] Phase 0 detection recorded (this file)
- [ ] Phase 4a — mimalloc `#[global_allocator]` in `src/main.rs` + bench crates; `cargo check` passes
- [ ] Phase 3 — `benches/hot_path.rs` (divan) targeting init + create + list via the library API
- [ ] iai-callgrind bench at `benches/iai.rs` — `cargo check --benches` passes
- [ ] dhat-heap feature wiring (same pattern as core + prime-mcp)
- [ ] `release-debug` profile in `apps/chronis/Cargo.toml` for future samply/dhat symbolication
- [ ] `.github/workflows/perf-bench.yml` matrix extended to include `allsource-prime` and `chronis` (both alongside `allsource-core`)

### Linux-CI-only (same constraint as core/prime-mcp)

- iai-callgrind actual run
- cargo-pgo training run + optimized-binary re-bench
- BOLT conditional follow-up

### Skipped

- ❌ pprof-rs endpoint (not a server)
- ❌ BOLT in Dockerfile (deferred)

## Measurements

Local bench runs on macOS arm64 — chronis (unlike prime-mcp) links and runs cleanly, so real numbers are available.

Bench: `cargo bench --bench hot_path -- --sample-count 50`.

| Benchmark | Baseline median | Post-mimalloc median | Δ time |
|---|---|---|---|
| `bench_workspace_init` | 99.39 μs | **69.83 μs** | **−29.7%** |
| `bench_create_single_task` | 34.22 μs | **25.10 μs** | **−26.7%** |
| `bench_create_then_list/10` | 444.4 μs | **248.8 μs** | **−44.0%** |
| `bench_create_then_list/100` | 2.828 ms | **2.331 ms** | **−17.6%** |

Mean of medians across the four benches: **−29.5% median time reduction**. Consistent with Core's result (−32.7% on 10k ingest) — the decision tree picking mimalloc for both workload shapes (server and CLI) was validated twice.

This **already exceeds the US-006 ≥5% speedup target by ~6×**, before PGO or BOLT. PGO remains a CI follow-up per the decision tree; given mimalloc alone hits 29.5%, BOLT (conditional on PGO ≥5%) is unlikely to be warranted — we'll skip it unless PGO surprises with another meaningful gain.

Reproducing the A/B:

```bash
cd apps/chronis
# Baseline: comment out the #[global_allocator] block in benches/hot_path.rs
cargo bench --bench hot_path -- --sample-count 50
# Post-mimalloc: uncomment, rerun
cargo bench --bench hot_path -- --sample-count 50
```

## Profiling artifacts

### samply

```bash
cd apps/chronis
cargo bench --bench hot_path --no-run --profile release-debug
samply record --save-only -o perf-artifacts/chronis-flamegraph.json.gz \
  target/release-debug/deps/hot_path-<hash> --sample-count 30
# View: samply load perf-artifacts/chronis-flamegraph.json.gz
#   or drop the .json.gz into https://profiler.firefox.com
```

Captured: `apps/chronis/perf-artifacts/chronis-flamegraph.json.gz` (~5.5 KB, 793 samples). Symbolication in samply's `--save-only` mode happens at view time (Firefox Profiler); the raw JSON stores addresses plus the binary path, so text-mode extraction from the `.json.gz` is unreliable. The file is the deliverable — open in the viewer for a named flamegraph.

### dhat

Not captured in this session. The dhat-heap feature-flag plumbing is in place (`--features dhat-heap` swaps `mimalloc` for `dhat::Alloc`); capture on demand by running an instrumented `cn` invocation or wiring dhat into a bench binary — follow the Core pattern (`apps/core/PERF_NOTES.md`).

## CI scaffolding

### iai-callgrind bench (`benches/iai.rs`)

Mirrors the divan cases in iai form: `bench_workspace_init`, `bench_create_batch` (small/medium), `bench_list_after_create`. Compiles on macOS (`cargo check --bench iai`); requires valgrind ⇒ runs in CI.

### Workflow — `.github/workflows/perf-bench.yml`

Matrix now covers all three Rust binaries:

| Package | Working directory | Allocator | PGO/BOLT |
|---|---|---|---|
| `allsource-core` | `apps/core` | mimalloc | eligible (deferred as US-004b → won't-do) |
| `allsource-prime` | `apps/prime-mcp` | jemallocator | skipped by decision tree |
| `chronis` | `apps/chronis` | mimalloc | eligible — PGO is a natural fit, BOLT conditional |

Each runs the same 3% iai-callgrind regression gate on `ubuntu-latest` with valgrind installed.

### Phase 1a hotpath — available but not run

chronis has `#[cfg_attr(feature = "hotpath", hotpath::main)]` on its `main()`. A hotpath-instrumented CLI run (`cargo run --release --features hotpath -- list`) produces a report on exit. Not run in this session; the dhat + samply evidence from Core plus the clean divan-measured mimalloc delta here is sufficient signal for US-006's scope.

## What this unlocks

Final epic state after US-006 lands: all three Rust binary apps in the monorepo have:
- PERF_NOTES.md with Phase 0 detection + decision-tree rationale
- A divan microbenchmark for wall-clock tracking
- An iai-callgrind bench + CI 3% regression gate
- The allocator the decision tree picked, with actually-measured deltas where the local build allows
- Feature-gated `dhat-heap` harness pattern for on-demand heap profiling

The global `rust-perf` skill itself was augmented in parallel (US-001 → 003) with templates that any future Rust project can adopt in <10 minutes. Epic `t-cab1` is ready to close after this.
