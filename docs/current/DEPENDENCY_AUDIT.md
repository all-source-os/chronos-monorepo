# Dependency Audit & Compile-Time Baseline — US-005

**Date**: 2026-03-11
**Package**: `allsource-core v0.14.7`
**Rust edition**: 2024 (MSRV 1.92)
**Build profile**: dev (unoptimized + debuginfo)
**Features tested**: `server` + `analytics` (default set)

---

## Summary

| Metric | Value |
|--------|-------|
| Direct dependencies | 49 (+ 5 dev-dependencies) |
| Transitive dependency tree lines | 2,480 |
| Total compilation units | 476 |
| Full build time (cold, dev profile) | **3m 28s** |
| Duplicate crate families | 24 |

---

## Top 10 Slowest Crates (Cold Build)

| Rank | Crate | Time | Frontend | Codegen |
|------|-------|------|----------|---------|
| 1 | sqlparser v0.59.0 | 41.7s | 26.0s (62%) | 15.7s (38%) |
| 2 | aws-lc-sys v0.37.1 (build script) | 39.1s | — | — |
| 3 | datafusion-catalog v52.2.0 | 37.8s | 36.9s (98%) | 0.9s (2%) |
| 4 | datafusion-pruning v52.2.0 | 37.8s | 0.7s (2%) | 37.1s (98%) |
| 5 | datafusion v52.1.0 | 28.2s | 25.1s (89%) | 3.1s (11%) |
| 6 | **allsource-core v0.14.7** | 26.8s | 17.6s (66%) | 9.1s (34%) |
| 7 | arrow-ord v57.3.0 | 25.0s | 2.5s (10%) | 22.6s (90%) |
| 8 | parquet v57.3.0 | 24.1s | 17.0s (70%) | 7.1s (30%) |
| 9 | arrow-cast v57.3.0 | 22.2s | 7.5s (34%) | 14.7s (66%) |
| 10 | arrow-select v57.3.0 | 21.4s | 2.7s (12%) | 18.8s (88%) |

**Key observations:**
- DataFusion + Arrow + Parquet dominate the top 10, occupying 7 of 10 slots and accounting for ~197s of wall time on the critical path.
- `aws-lc-sys` build script compiles a C library (AWS LibCrypto) from source — 39.1s of pure build-script time.
- `sqlparser` is the single slowest Rust crate due to its large recursive AST.
- allsource-core itself takes 26.8s, with 66% in frontend (type checking/monomorphization).

---

## Duplicate Crate Versions (24 Families)

Crates where multiple semver-incompatible versions coexist in the dependency tree:

| Crate | Versions | Root cause |
|-------|----------|------------|
| **base64** | 0.13.1, 0.21.7, 0.22.1 | fastembed (via spm_precompiled) pulls 0.13; testcontainers pulls 0.21; direct + arrow use 0.22 |
| **rand** | 0.8.5, 0.9.2, 0.10.0 | Ecosystem-wide migration; allsource-core uses 0.10, transitive deps lag behind |
| **rand_core** | 0.6.4, 0.9.5, 0.10.0 | Tracks rand versioning |
| **rand_chacha** | 0.3.1, 0.9.0 | Same rand migration |
| **getrandom** | 0.2.17, 0.3.4, 0.4.1 | Same rand migration |
| **hashbrown** | 0.14.5, 0.15.5, 0.16.1 | indexmap/dashmap/tantivy ecosystem split |
| **darling** | 0.20.11, 0.21.3, 0.23.0 | Proc-macro derive crates (sqlx, serde) |
| **thiserror** | 1.0.69, 2.0.18 | allsource-core uses v2; many transitive deps still on v1 |
| **reqwest** | 0.12.28, 0.13.2 | allsource-core uses 0.13; fastembed (via hf-hub) pulls 0.12 |
| **ureq** | 2.12.1, 3.2.0 | hf-hub uses v2; bollard/ort-sys use v3 |
| **itertools** | 0.13.0, 0.14.0 | datafusion ecosystem split |
| **nom** | 7.1.3, 8.0.0 | rocksdb/tantivy on v7; newer deps on v8 |
| **rustix** | 0.38.44, 1.1.4 | Major version bump in progress |
| **lz4_flex** | 0.11.5, 0.12.0 | parquet vs tantivy |
| **foldhash** | 0.1.5, 0.2.0 | tantivy ecosystem |
| **compact_str** | 0.8.1, 0.9.0 | tantivy ecosystem |
| **crossterm** | 0.28.1, 0.29.0 | criterion terminal output |
| **core-foundation** | 0.9.4, 0.10.1 | macOS TLS (native-tls ecosystem) |
| **ordered-float** | 2.10.1, 3.9.2 | tantivy uses v2; others use v3 |
| **rustc-hash** | 1.1.0, 2.1.1 | tantivy/datafusion split |
| **unicode-width** | 0.1.14, 0.2.0 | Terminal formatting |
| **webpki-roots** | 0.26.11, 1.0.6 | TLS root cert bundles |
| **zune-core / zune-jpeg** | 0.4.x, 0.5.x | Image processing (fastembed) |

---

## Optional Dependencies & Feature Gates

| Dependency | Feature gate | Correctly gated? |
|------------|-------------|-------------------|
| `axum` | `server` | Yes (`dep:axum`) |
| `tower` | `server` | Yes (`dep:tower`) |
| `tower-http` | `server` | Yes (`dep:tower-http`) |
| `reqwest` | `server`, `embedded-sync` | Yes (`dep:reqwest`) |
| `jsonwebtoken` | `server` | Yes (`dep:jsonwebtoken`) |
| `argon2` | `server` | Yes (`dep:argon2`) |
| `aes-gcm` | `server` | Yes (`dep:aes-gcm`) |
| `http` | `server` | Yes (`dep:http`) |
| `datafusion` | `analytics` | Yes (`dep:datafusion`) |
| `arrow-flight` | `flight` | Yes (`dep:arrow-flight`) |
| `sqlx` | `postgres` | Yes (implicit `dep:sqlx`) |
| `rocksdb` | `rocksdb-storage` | Yes (implicit `dep:rocksdb`) |
| `fastembed` | `vector-search` | Yes (implicit `dep:fastembed`) |
| `instant-distance` | `vector-search` | Yes (`dep:instant-distance`) |
| `tantivy` | `keyword-search` | Yes (implicit `dep:tantivy`) |
| `hotpath` | `hotpath` | Yes (`dep:hotpath`) |
| `toon-format` | `embedded-toon` | Yes (`dep:toon-format`) |

**Total optional deps**: 17
**All correctly gated**: Yes. Every optional dependency uses either the explicit `dep:` syntax or implicit feature activation. No optional dependency is unconditionally compiled.

---

## Unused Dependency Analysis

`cargo-machete` and `cargo-udeps` were not available on this system. Manual inspection notes:

| Dependency | Assessment | Notes |
|------------|-----------|-------|
| `bumpalo` | **Used** | Arena allocator used in `arena_pool.rs`, `batch_processor.rs`, `simd_json.rs`. Keep. |
| `crossbeam-queue` | **Redundant** | Not imported directly anywhere. Code uses `crossbeam::queue::ArrayQueue` via the `crossbeam` crate re-export. Safe to remove. |
| `flate2` | **Used** | Used directly in `backup.rs` for gzip compression. Keep. |
| `lz4` | **Unused** | Not imported anywhere in source. Parquet uses `lz4_flex` internally. Safe to remove. |
| `hex` | Low concern | Small crate, likely used for hash display. |
| `hmac` | Low concern | Used for webhook signatures. |
| `prometheus` | Verify | Check if `metrics` ecosystem would be lighter. |

**Recommendation**: Install `cargo-machete` (`cargo install cargo-machete`) and run it for a definitive unused-dep list.

---

## Recommendations

### High Impact — Compile Time Reduction

1. **Make `analytics` (DataFusion) non-default feature**
   - DataFusion alone accounts for 4 of the top 10 slowest crates (~132s).
   - Most dev iterations don't need SQL analytics. Making it opt-in (`cargo build -p allsource-core --features server`) would cut cold builds from 3m28s to ~2m.
   - Production Docker builds would still enable it: `--features server,analytics`.

2. **Pre-built aws-lc-sys or switch to ring**
   - `aws-lc-sys` build script takes 39.1s compiling C code.
   - Consider `ring` as the TLS backend or pre-cache the build artifact in CI.
   - Alternatively, use `rustls` with `aws-lc-rs` pre-built binaries if available for target platforms.

3. **Gate `fastembed` more aggressively**
   - Already behind `vector-search` feature (good), but it brings in `reqwest v0.12` (duplicate), `ureq v2` (duplicate), `base64 v0.13` (duplicate), ONNX runtime, and tokenizers.
   - Ensure CI and local dev never enable `vector-search` unless testing that feature.

### Medium Impact — Dependency Deduplication

4. **Remove `crossbeam-queue` direct dependency**
   - `crossbeam` already includes `crossbeam-queue`. Drop the separate dep.

5. **Remove `lz4` direct dependency**
   - Not imported anywhere in allsource-core source code. Parquet handles LZ4 compression internally via `lz4_flex`. Removing `lz4` eliminates a C library dependency (lz4-sys) and its build-script overhead.

### Low Impact — Hygiene

7. **Pin `thiserror` v2 across the workspace**
   - Both v1 and v2 coexist. As transitive deps migrate to v2, the duplication will resolve naturally. No action needed now.

8. **Track `rand` 0.8 → 0.10 migration**
   - Three versions coexist. This is an ecosystem-wide issue. Monitor upstream crate updates.

9. **Install `cargo-machete` in CI**
   - Add `cargo machete --with-metadata` to the quality gate to catch unused deps going forward.

10. **Consider `cargo-hakari` for workspace-hack crate**
    - With 476 compilation units, a workspace-hack crate could improve incremental build times by unifying feature resolution.

---

## Appendix: Build Environment

```
Platform: darwin (Apple Silicon, arm64)
Rust: edition 2024, MSRV 1.92
Profile: dev [unoptimized + debuginfo]
Timings report: target/cargo-timings/cargo-timing-20260311T110156.086253Z.html
```
