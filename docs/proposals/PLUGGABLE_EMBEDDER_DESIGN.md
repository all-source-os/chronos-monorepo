# Pluggable embedder for Prime

**Status:** Implemented (v1) behind the `prime-remote-embed` feature
**Issue:** [#200](https://github.com/all-source-os/all-source/issues/200) (suggestions #4 and #6)
**Scope:** `apps/core/src/prime/vectors/embedder.rs`, `apps/prime-mcp`

## What shipped

The remote backend (OpenAI- / Ollama-compatible) is implemented behind the
`prime-remote-embed` cargo feature, selected by `PRIME_EMBED_ENDPOINT`. The
sync-in-async blocker (below) was resolved with the `block_in_place` + `block_on`
approach; proxy support (#6) comes via reqwest honoring `*_PROXY`. The companion
`crates/allsource-prime-models` + `prime-bundled-model` feature bakes the model
into the binary for zero-runtime-fetch offline.

**Still open:** the dimension-mismatch guard (record embedder identity as a
`prime.embedder.*` event and refuse a mismatched backend on a populated
`--data-dir`) is documented but not yet enforced — today a switch is caught only
at store time by the vector repo's dimension check. That hardening is the
remaining follow-up.

---

## Original design (for reference)

## Why

Today `Prime::embed_text` is hard-wired to one network-fetched model
(`fastembed` → `all-MiniLM-L6-v2` from HuggingFace). #200 fixed the worst of it
— offline vendored-dir load (`PRIME_EMBED_MODEL_DIR`), `--mode warm`, an
actionable error, and the bring-your-own-`vector` escape hatch. What's still
single-point-of-failure: the *in-process* embedder is the only server-side
text→vector path. A user who wants to point at OpenAI / Cohere / Voyage / a
local Ollama daemon must precompute every vector client-side.

This proposal adds a **remote embedder backend** selected by env, leaving the
local fastembed path as the default.

## Interface (env-driven, no schema change)

| Var | Meaning |
|-----|---------|
| `PRIME_EMBED_ENDPOINT` | When set, use the remote backend. OpenAI-compatible `POST {endpoint}` taking `{"model": ..., "input": [text]}` and returning `{"data":[{"embedding":[...]}]}`. Ollama's `/api/embeddings` shape supported via a `PRIME_EMBED_PROTOCOL=ollama` switch. |
| `PRIME_EMBED_API_KEY` | Optional bearer token (`Authorization: Bearer …`). |
| `PRIME_EMBED_MODEL` | Model name sent in the request (e.g. `text-embedding-3-small`, `nomic-embed-text`). |
| `PRIME_EMBED_PROTOCOL` | `openai` (default) or `ollama`. |

Precedence: `PRIME_EMBED_ENDPOINT` (remote) → `PRIME_EMBED_MODEL_DIR` (offline
local) → network fastembed (default). All three already converge on the same
`TextEmbedder` facade, so callers (`embed_text`, recall, HTTP) are unchanged.

**Proxy for free (#6):** the remote backend uses an HTTP client that honors
`HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY` / `NO_PROXY`. Pointing
`PRIME_EMBED_ENDPOINT` at a local Ollama (`http://127.0.0.1:11434/api/embeddings`)
sidesteps every upstream-network concern in the original report.

## The two real blockers (why this is a separate PR, not a one-liner)

1. **Sync embed in an async runtime.** `TextEmbedder::embed` is **synchronous**
   (it's called from `Prime::embed_text`, itself called from inside the
   `#[tokio::main]` MCP/HTTP handlers). A naive `reqwest::blocking` call panics
   with *"Cannot start a runtime from within a runtime."* Options, in order of
   preference:
   - `tokio::task::block_in_place` + `Handle::current().block_on(fut)` with the
     async `reqwest` already in the tree (gated behind `prime-recall`). Requires
     the multi-threaded runtime (prime-mcp uses `tokio = ["full"]`, so OK) and
     making `prime-vectors` pull `reqwest`, or introducing a
     `prime-remote-embed` feature.
   - A dedicated `std::thread` running a sync `ureq` call, joined. Simple, no
     runtime coupling, but adds a `ureq` direct dep (two majors already in the
     tree transitively — pick 3.x, accept the duplicate or unify).
   - Make `embed_text` async end-to-end. Cleanest long-term, largest diff
     (touches `vector_search_engine`, `hybrid_search_engine`, facade, both
     transports). Probably the right eventual move.

2. **Dimension mixing is a data-dir footgun.** The HNSW index auto-learns its
   dimension from the first stored vector
   (`in_memory_vector_search_repository::set_dimensions`). fastembed is 384-dim;
   `text-embedding-3-small` is 1536. Switching embedders against an existing
   `--data-dir` silently mixes incompatible vectors and corrupts similarity.
   Mitigation: record the embedder identity+dimension in a `prime.embedder.*`
   event on first embed and **refuse** a mismatched backend on a populated
   data-dir with an actionable error (offer `--data-dir <new>` or a re-embed
   path). This is the part that needs care, not the HTTP call.

## Plan

1. Refactor `TextEmbedder` into an enum backend (`Local(Mutex<TextEmbedding>)`
   / `Remote(RemoteEmbedder)`), keep `new()` / `embed()` / `dimensions()`.
2. `RemoteEmbedder`: probe-embed at construction to learn dimension and
   fail-fast with an actionable error (consistent with #200's error work).
3. Persist embedder identity (`model`, `dim`, `backend`) as a Prime event; guard
   against dimension mismatch on open.
4. Tests: mock endpoint (openai + ollama shapes), dimension-mismatch guard,
   proxy-env honored. Keep network-touching tests `#[ignore]`.
5. Docs: extend the prime-mcp README "Offline embeddings" section with a
   "Remote / pluggable embedder" subsection.

## Not in scope

- In-binary model bundling. `all-MiniLM-L6-v2` q-none is ~22 MB; committing it to
  the crate exceeds the crates.io default size budget, so `cargo install`
  can't carry it without an external fetch anyway. The runtime
  `PRIME_EMBED_MODEL_DIR` path already delivers offline without a rebuild and is
  the better mechanism. Revisit only if a `-models` companion crate is wanted.
