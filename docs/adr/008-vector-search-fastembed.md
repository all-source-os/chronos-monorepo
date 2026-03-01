# ADR-008: Vector Search with fastembed (Pure Rust)

**Status:** Accepted
**Date:** 2026-02-03
**Release:** v0.10.0

## Context

Users wanted semantic search over events — finding events by meaning rather than exact keyword match. Options considered:

1. **External vector DB** (Qdrant, Pinecone): Adds another service to deploy and sync
2. **Python-based embedding** (sentence-transformers): Requires Python runtime, FFI complexity
3. **fastembed (Rust)**: Pure Rust embedding library, no external dependencies

## Decision

Use fastembed for in-process vector embeddings with a custom geo-aware index:

- Events are embedded at ingest time using fastembed's ONNX-based models
- Vector index stored alongside event data in Core's DashMap
- `POST /api/v1/demo/seed` auto-generates embeddings for demo data
- Semantic search exposed via query API with cosine similarity ranking

## Consequences

### Positive
- Zero external dependencies — no vector DB to deploy
- Sub-millisecond vector lookup (in-process, no network hop)
- Pure Rust — compiles for all targets including embedded/Tauri
- Demo seed endpoint creates rich searchable data out of the box

### Negative
- Embedding models increase binary size (~30MB for ONNX runtime)
- Limited to models supported by fastembed (no custom fine-tuned models)
- Vector index is in-memory only (not persisted to WAL/Parquet yet)
