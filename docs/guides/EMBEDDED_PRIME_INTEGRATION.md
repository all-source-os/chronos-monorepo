# Embedded Prime Integration Guide

How to use AllSource Prime as an embedded library for agent memory — graph, vectors, and tiered recall — without running a separate server.

## Table of Contents

1. [Feature Flags](#feature-flags)
2. [Initialization](#initialization)
3. [Migration from EmbeddedCore](#migration-from-embeddedcore)
4. [VectorIndexProjection Usage](#vectorindexprojection-usage)
5. [Local Embeddings with fastembed](#local-embeddings-with-fastembed)
6. [RecallEngine and Tiered Retrieval](#recallengine-and-tiered-retrieval)
7. [Hybrid Recall (Graph + Vectors)](#hybrid-recall-graph--vectors)
8. [Configuration Reference](#configuration-reference)

---

## Feature Flags

Add `allsource-core` with the features you need:

```toml
[dependencies]
allsource-core = { version = "0.17.1", features = ["prime-full", "prime-recall"] }
```

| Feature | What it enables | Compile cost |
|---------|----------------|-------------|
| `prime` | Graph engine (nodes, edges, schema, projections) | Low |
| `prime-vectors` | HNSW vector index + `fastembed` + `instant-distance` | Medium (fastembed model download on first run) |
| `prime-full` | `prime` + `prime-vectors` | Medium |
| `prime-recall` | RecallEngine with L0/L1/L2 tiers + index compression | Low (adds `reqwest` for optional Ollama LLM) |
| `vector-search` | Standalone vector search (used by `prime-vectors` internally) | Medium |

For a Tauri app replacing O(n) cosine search with HNSW + tiered recall:

```toml
allsource-core = { version = "0.17.1", features = ["prime-full", "prime-recall"] }
```

---

## Initialization

### `Prime::open(path)` — Durable, persistent

```rust
use allsource_core::prime::Prime;

let prime = Prime::open("/path/to/app/data/prime").await?;

// All data persists across restarts via WAL + Parquet.
// Graph projections are rebuilt from WAL on startup.
```

In a Tauri app:

```rust
use allsource_core::prime::Prime;
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
async fn init(app: tauri::AppHandle) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let prime_dir = data_dir.join("prime");
    let prime = Prime::open(&prime_dir).await.map_err(|e| e.to_string())?;
    app.manage(Arc::new(prime));
    Ok(())
}
```

### `Prime::open_in_memory()` — Testing only

```rust
let prime = Prime::open_in_memory().await?;
// No persistence. Data lost on drop.
```

### Graceful shutdown

```rust
prime.shutdown().await?; // Flushes WAL, Parquet checkpoint
```

---

## Migration from EmbeddedCore

### Architecture

Prime **wraps** EmbeddedCore — it is not a replacement. When you call `Prime::open()`, it internally creates an `EmbeddedCore` with graph-specific merge strategies and registers 8+ graph projections on it.

```
Prime
 └── EmbeddedCore (accessible via prime.core())
      └── EventStore (WAL + Parquet + DashMap)
           └── ProjectionManager
                ├── 8 graph projections (node_state, adjacency, etc.)
                ├── 1 vector projection (if prime-vectors enabled)
                └── your domain projections (registered separately)
```

### Option A: Prime wraps a fresh Core (recommended for new projects)

```rust
let prime = Prime::open(&data_dir).await?;

// Access the underlying EmbeddedCore for raw event operations:
let core = prime.core();
core.ingest(IngestEvent { ... }).await?;
core.query(Query::new().entity_id("workflow:123")).await?;
```

### Option B: Register domain projections alongside Prime's

If you have existing domain projections (e.g., 39 projections in Longhand), register them on the same underlying store:

```rust
use allsource_core::application::services::projection::Projection;

let prime = Prime::open(&data_dir).await?;
let store = prime.core().inner(); // Arc<EventStore>

// Register your existing domain projections
let my_proj = Arc::new(MyWorkflowProjection::new());
store.register_projection_with_backfill(
    &(Arc::clone(&my_proj) as Arc<dyn Projection>)
);

// Both Prime graph projections and your domain projections
// now process the same event stream.
```

### Option C: Separate data directories

If you want complete isolation between your domain events and Prime's graph events:

```rust
// Your existing EmbeddedCore for domain events
let domain_core = EmbeddedCore::open(
    Config::builder().data_dir(&domain_dir).build()?
).await?;

// Separate Prime instance for agent memory
let prime = Prime::open(&prime_dir).await?;
```

This keeps event streams separate but means you can't query across them in a single call.

### What changes from raw EmbeddedCore

| Before (EmbeddedCore) | After (Prime) |
|----------------------|--------------|
| `core.ingest(event)` | `prime.add_node(type, props)` / `prime.add_edge(...)` |
| Manual O(n) cosine search | `prime.vector_search(query, top_k)` — HNSW O(log n) |
| Custom projection for entity state | `prime.get_node(id)` — O(1) via NodeStateProjection |
| No graph traversal | `prime.neighbors(id, depth)` — BFS with direction filter |
| No recall API | `recall.context(query)` — tiered retrieval |

---

## VectorIndexProjection Usage

### Storing embeddings

Prime stores embeddings as events. The vector goes in event metadata; text and user metadata go in the payload.

```rust
// Generate embedding externally (fastembed, OpenAI, Ollama, etc.)
let embedding: Vec<f32> = generate_embedding("skill output text")?;

// Store in Prime
prime.embed("output-42", Some("skill output text"), embedding).await?;

// With metadata
prime.embed_with_metadata(
    "output-42",
    Some("skill output text"),
    embedding,
    Some(serde_json::json!({
        "project_id": "proj-1",
        "skill": "summarize",
        "created_at": "2026-03-26T12:00:00Z"
    })),
).await?;
```

**Event stored:** `prime.vector.stored` with entity ID `vec:output-42`.

### Querying by similarity

```rust
// Direct vector search — returns top K by cosine similarity
let query_vec: Vec<f32> = generate_embedding("revenue analysis")?;
let results = prime.vector_search(&query_vec, 5);

for result in &results {
    println!("{}: score={:.3} text={:?}", result.id, result.score, result.text);
}
```

### Find similar to an existing embedding

```rust
// Find the 5 most similar vectors to a stored embedding
let similar = prime.similar("output-42", 5)?;
```

### Cross-domain search

Over-fetches 3x, groups by domain, round-robin interleaves so results span multiple domains:

```rust
let results = prime.vector_search_cross_domain(&query_vec, 5);
```

### How the HNSW index works

- **Batch rebuild, not incremental** — the `instant-distance` crate is build-once. The index is lazily rebuilt when a search is performed and the generation counter has advanced.
- **Generation counter** — avoids TOCTOU races. Each `embed()` / delete increments the counter; `search()` checks if it needs to rebuild.
- **Cosine distance** — `distance = 1.0 - cosine_similarity`. Results are converted back to similarity scores (0.0–1.0).
- **Config** — `ef_construction: 100`, `ef_search: 100` (defaults). Higher values = more accurate but slower.

---

## Local Embeddings with fastembed

The `vector-search` feature (pulled in by `prime-vectors`) includes `fastembed` — a pure-Rust embedding library with no Python dependency.

### Using the built-in VectorSearchEngine

```rust
use allsource_core::infrastructure::search::vector_search_engine::{
    VectorSearchEngine, VectorSearchEngineConfig,
};

let config = VectorSearchEngineConfig {
    model_name: "AllMiniLmL6V2".to_string(), // 384 dimensions
    embedding_dimensions: 384,
    ..Default::default()
};

let engine = VectorSearchEngine::new(config)?;
let embeddings = engine.embed_texts(&["skill output text"])?;
```

### Direct fastembed usage

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::AllMiniLmL6V2,
    show_download_progress: true,
    ..Default::default()
})?;

let embeddings = model.embed(vec!["skill output text"], None)?;
// embeddings[0] is Vec<f32> with 384 dimensions
```

### Model selection for short-form content (100–500 chars)

| Model | Dimensions | Speed | Quality for short text | Cache size |
|-------|-----------|-------|----------------------|-----------|
| **AllMiniLmL6V2** (default) | 384 | Fast | Good | ~23 MB |
| AllMiniLmL12V2 | 384 | Medium | Better | ~33 MB |
| BGESmallENV15 | 384 | Fast | Good | ~33 MB |

**Recommendation for skill outputs (100–500 chars):** `AllMiniLmL6V2` is the default and a good fit. It's optimized for short passages, fast enough for real-time use, and the 384-dimension vectors keep memory usage low.

The model files are downloaded on first use and cached in `.fastembed_cache/` (excluded from git via Core's Cargo.toml).

### Replacing your O(n) cosine search

Before:
```rust
// O(n) brute force over ALL outputs
let query_embedding = embedding::generate_embedding(skill_prompt, &embed_config).await?;
for output in all_outputs {
    let similarity = cosine_similarity(&query_embedding, &output.embedding);
    candidates.push((similarity, output.content));
}
candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
candidates.truncate(5);
```

After:
```rust
// O(log n) HNSW search
let model = TextEmbedding::try_new(InitOptions {
    model_name: EmbeddingModel::AllMiniLmL6V2,
    ..Default::default()
})?;
let query_vec = model.embed(vec![skill_prompt], None)?[0].clone();
let results = prime.vector_search(&query_vec, 5);
```

---

## RecallEngine and Tiered Retrieval

The RecallEngine provides three retrieval tiers with increasing cost and context size.

### Setup

```rust
use allsource_core::prime::recall::{RecallEngine, RecallContextQuery, RecallDeps};
use allsource_core::prime::recall::types::{ContextTier, IndexConfig};

// Preferred: share Prime's projections (enables L0/L1 tiers)
let recall = RecallEngine::with_deps(prime.recall_deps(), &IndexConfig::default());
```

### L0: Stats only (~100–200 tokens)

O(1) retrieval. Returns graph statistics — total nodes, edges, types, domains.

```rust
let ctx = recall.context(RecallContextQuery {
    query: "what do I know?".to_string(),
    tier: ContextTier::L0,
    ..Default::default()
}).await;

// ctx.stats — Some(PrimeStats { total_nodes, total_edges, ... })
// ctx.token_count — ~100-200
```

Use case: orientation queries, "do I have any data about X?"

### L1: Conversation context (~500–1500 tokens)

Stats + recent conversation-scoped nodes + 1-hop edges. No vector search.

```rust
let ctx = recall.context(RecallContextQuery {
    query: "what was that project name?".to_string(),
    tier: ContextTier::L1,
    conversation_id: Some("conv-abc".to_string()),
    ..Default::default()
}).await;

// ctx.stats + ctx.nodes (recent from this conversation) + ctx.edges (1-hop)
```

Use case: follow-up questions within the same session.

### L2: Full hybrid recall (~2000–5000 tokens)

Compressed index + vector search + graph expansion. This is the default tier.

```rust
let ctx = recall.context(RecallContextQuery {
    query: "revenue analysis across projects".to_string(),
    tier: ContextTier::L2,  // default
    top_k: 5,
    include_index: true,
    ..Default::default()
}).await;

// ctx.index — compressed markdown summary of all knowledge
// ctx.vectors — top K similar memories (RankedMemory)
// ctx.nodes — graph-expanded context nodes
// ctx.edges — related edges
// ctx.stats — graph statistics
// ctx.token_count — ~2000-5000
```

Use case: cross-domain queries, complex retrieval, what the issue author wants:

```rust
// Replace ~70 lines of brute-force cosine search with:
let ctx = recall.context(RecallContextQuery {
    query: skill_prompt.to_string(),
    tier: ContextTier::L2,
    top_k: 5,
    ..Default::default()
}).await;
let context_string = ctx.index; // Compressed markdown of relevant knowledge
```

### With LLM-powered index compression (optional)

The compressed index can be generated by a heuristic (default) or an LLM (Ollama):

```rust
let config = IndexConfig {
    max_tokens: 1000,
    llm_endpoint: Some("http://localhost:11434/api/generate".to_string()),
    llm_model: Some("mistral:7b".to_string()),
    refresh_interval_events: 100,  // Rebuild every 100 events
    refresh_interval_seconds: 300, // Or every 5 minutes
};

let recall = RecallEngine::with_deps(prime.recall_deps(), &config);
```

Without an LLM endpoint, the index is a deterministic heuristic markdown summary — domain counts, type distributions, example entities, cross-references. This works well and has zero latency.

---

## Hybrid Recall (Graph + Vectors)

For full hybrid scoring that combines cosine similarity, graph proximity, and temporal recency:

```rust
use allsource_core::prime::types::RecallQuery;

let result = prime.recall(RecallQuery {
    vector: Some(query_vec),
    text: Some("revenue analysis".to_string()),
    depth: 1,           // BFS expansion depth from vector hits
    top_k: 10,
    similarity_weight: 0.5,
    proximity_weight: 0.3,
    recency_weight: 0.2,
    ..RecallQuery::default()
}).await?;

// result.nodes — scored and ranked by hybrid score
// result.vectors — raw vector search results
// result.edges — edges discovered during graph expansion
```

**Scoring formula:**
```
score = sw * cosine_similarity + pw * 1/(1 + depth) + rw * exp_decay(age)
```

Weights are normalized to sum to 1.0. Vector hits seed the search at depth 0 (max proximity), then BFS expands through the graph.

---

## Configuration Reference

### IndexConfig

| Field | Default | Description |
|-------|---------|-------------|
| `max_tokens` | 1000 | Token budget for compressed index (adaptive 500–2000) |
| `llm_endpoint` | `None` | Ollama endpoint for LLM compression (e.g., `http://localhost:11434/api/generate`) |
| `llm_model` | `None` | LLM model name (e.g., `mistral:7b`). Falls back to `"mistral"` |
| `refresh_interval_events` | 100 | Rebuild index every N new events |
| `refresh_interval_seconds` | 300 | Rebuild index every M seconds |

### VectorIndexConfig

| Field | Default | Description |
|-------|---------|-------------|
| `ef_construction` | 100 | HNSW build-time accuracy. Higher = better recall, slower build |
| `ef_search` | 100 | HNSW search-time accuracy. Higher = better recall, slower search |

### RecallContextQuery

| Field | Default | Description |
|-------|---------|-------------|
| `query` | `""` | Natural language query string |
| `agent_id` | `None` | Scope to a specific agent |
| `top_k` | 5 | Max vector results |
| `as_of` | `None` | Time-travel: only knowledge that existed at this timestamp |
| `include_index` | `true` | Include compressed index in response |
| `max_tokens` | `None` | Truncate response to this token budget |
| `tier` | `L2` | Retrieval tier: `L0`, `L1`, or `L2` |
| `conversation_id` | `None` | Scope L1 to a specific conversation |

### RecallQuery (hybrid recall)

| Field | Default | Description |
|-------|---------|-------------|
| `vector` | `None` | Query embedding vector |
| `text` | `None` | Query text |
| `depth` | 1 | BFS expansion depth from vector hits |
| `top_k` | 10 | Number of results |
| `similarity_weight` | 0.5 | Weight for cosine similarity |
| `proximity_weight` | 0.3 | Weight for graph proximity |
| `recency_weight` | 0.2 | Weight for temporal recency |

---

## Complete Example: Tauri App with Prime

```rust
use allsource_core::prime::Prime;
use allsource_core::prime::recall::{RecallEngine, RecallContextQuery};
use allsource_core::prime::recall::types::{ContextTier, IndexConfig};
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::Arc;
use tauri::Manager;

struct AppState {
    prime: Prime,
    recall: RecallEngine,
    embedder: TextEmbedding,
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;

    let rt = tokio::runtime::Handle::current();
    let prime = rt.block_on(Prime::open(data_dir.join("prime")))?;
    let recall = RecallEngine::with_deps(prime.recall_deps(), &IndexConfig::default());
    let embedder = TextEmbedding::try_new(InitOptions {
        model_name: EmbeddingModel::AllMiniLmL6V2,
        ..Default::default()
    })?;

    app.manage(Arc::new(AppState { prime, recall, embedder }));
    Ok(())
}

/// Ingest a skill output with embedding
#[tauri::command]
async fn store_output(
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    text: String,
    project_id: String,
) -> Result<(), String> {
    let embedding = state.embedder
        .embed(vec![&text], None)
        .map_err(|e| e.to_string())?
        .remove(0);

    state.prime
        .embed_with_metadata(
            &id,
            Some(&text),
            embedding,
            Some(serde_json::json!({ "project_id": project_id })),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve cross-project context (replaces O(n) brute-force)
#[tauri::command]
async fn retrieve_context(
    state: tauri::State<'_, Arc<AppState>>,
    skill_prompt: String,
) -> Result<String, String> {
    let ctx = state.recall
        .context(RecallContextQuery {
            query: skill_prompt,
            tier: ContextTier::L2,
            top_k: 5,
            ..Default::default()
        })
        .await;

    Ok(ctx.index)
}
```

This replaces ~70 lines of brute-force O(n) cosine similarity with O(log n) HNSW search, eliminates the external embedding API dependency, and adds tiered retrieval with graph expansion.
