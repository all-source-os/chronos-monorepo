# ADR-009: simd-json Zero-Copy Deserialization

**Status:** Accepted
**Date:** 2026-02-03
**Release:** v0.10.0

## Context

Event ingestion throughput was bounded by JSON parsing. Standard `serde_json` copies payload bytes during deserialization. At 469K events/sec baseline, payload parsing was a measurable fraction of ingest time.

## Decision

Add simd-json for zero-copy deserialization of event payloads:

- Uses SIMD instructions (AVX2/NEON) for parallel JSON parsing
- Borrows directly from input buffer instead of copying strings
- Falls back to `serde_json` on platforms without SIMD support
- Applied at the HTTP handler level for incoming event payloads

## Consequences

### Positive
- ~2x throughput improvement for payload-heavy events
- Zero additional memory allocation for string values in payloads
- Transparent fallback — no behavior change on non-SIMD platforms

### Negative
- Input buffer must be mutable (simd-json requirement) — requires a copy from the HTTP body
- Platform-specific codegen (different SIMD paths for x86 vs ARM)
- Adds ~200KB to binary size
