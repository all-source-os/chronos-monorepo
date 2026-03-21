//! Prime Vectors — semantic search with graph-aware hybrid recall.
//!
//! Shows: embed → similar → recall (vector + graph + temporal).
//!
//! Run with:
//!   cargo run --no-default-features --features prime-full --example prime_vectors

use allsource_core::prime::{Prime, types::RecallQuery};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let prime = Prime::open_in_memory().await?;

    // ─── 1. Create knowledge graph ──────────────────────────────────────
    println!("=== Building knowledge graph ===\n");

    let rust = prime
        .add_node("topic", json!({"name": "Rust", "category": "language"}))
        .await?;
    let python = prime
        .add_node("topic", json!({"name": "Python", "category": "language"}))
        .await?;
    let wasm = prime
        .add_node(
            "topic",
            json!({"name": "WebAssembly", "category": "runtime"}),
        )
        .await?;
    let ml = prime
        .add_node(
            "topic",
            json!({"name": "Machine Learning", "category": "field"}),
        )
        .await?;

    let rust_e = allsource_core::prime::EntityId::node("topic", rust.as_str()).to_string();
    let python_e = allsource_core::prime::EntityId::node("topic", python.as_str()).to_string();
    let wasm_e = allsource_core::prime::EntityId::node("topic", wasm.as_str()).to_string();
    let ml_e = allsource_core::prime::EntityId::node("topic", ml.as_str()).to_string();

    prime
        .add_edge(&rust_e, &wasm_e, "compiles_to", None)
        .await?;
    prime.add_edge(&python_e, &ml_e, "used_for", None).await?;
    prime
        .add_edge(
            &rust_e,
            &python_e,
            "interops_via",
            Some(json!({"mechanism": "PyO3"})),
        )
        .await?;

    // ─── 2. Embed vectors ───────────────────────────────────────────────
    println!("Embedding vectors...\n");

    // Simple deterministic embeddings for demo (real usage would use fastembed/Ollama)
    prime
        .embed(
            &rust_e,
            Some("Rust is a systems programming language focused on safety and performance"),
            fake_embedding("rust systems safety performance"),
        )
        .await?;

    prime
        .embed(
            &python_e,
            Some("Python is a dynamic language popular for data science and machine learning"),
            fake_embedding("python dynamic data science machine learning"),
        )
        .await?;

    prime
        .embed(
            &wasm_e,
            Some("WebAssembly is a portable compilation target for high-performance web apps"),
            fake_embedding("webassembly portable compilation web performance"),
        )
        .await?;

    prime
        .embed(
            &ml_e,
            Some("Machine learning uses statistical models to make predictions from data"),
            fake_embedding("machine learning statistical models predictions data"),
        )
        .await?;

    // ─── 3. Similarity search ───────────────────────────────────────────
    println!("=== Similarity search ===\n");

    let similar = prime.similar(&rust_e, 3)?;
    println!("Most similar to Rust:");
    for result in &similar {
        println!("  - {} (score: {:.3})", result.id, result.score);
        if let Some(ref text) = result.text {
            println!("    {}", &text[..text.len().min(80)]);
        }
    }

    // ─── 4. Hybrid recall ───────────────────────────────────────────────
    println!("\n=== Hybrid recall (vectors + graph) ===\n");

    let query_vec = fake_embedding("systems programming with safety guarantees");

    let recall_result = prime
        .recall(RecallQuery {
            vector: Some(query_vec),
            text: Some("systems programming safety".to_string()),
            depth: 1, // expand 1 hop from vector matches
            top_k: 5,
            ..RecallQuery::default()
        })
        .await?;

    println!("Recall results:");
    println!("  Vectors matched: {}", recall_result.vectors.len());
    for v in &recall_result.vectors {
        println!("    - {} (score: {:.3})", v.id, v.score);
    }
    println!("  Graph nodes expanded: {}", recall_result.nodes.len());
    for sn in &recall_result.nodes {
        println!(
            "    - {} [{}] (score: {:.3}, depth: {})",
            sn.node.properties["name"], sn.node.node_type, sn.score, sn.depth
        );
    }
    println!("  Edges in subgraph: {}", recall_result.edges.len());

    // ─── 5. Remember + Forget ───────────────────────────────────────────
    println!("\n=== Remember + Forget ===\n");

    let insight_id = prime
        .remember(
            "Rust's borrow checker eliminates data races at compile time",
            fake_embedding("rust borrow checker data races compile time"),
            "memory",
            json!({"source": "documentation", "confidence": 0.95}),
            &[], // no explicit relations
        )
        .await?;
    let insight_entity = allsource_core::prime::EntityId::node("memory", &insight_id).to_string();
    println!("Remembered: {insight_entity}");

    // Verify it exists
    assert!(prime.get_node(&insight_entity).is_some());

    // Forget it (soft-delete — preserved in history)
    prime.forget(&insight_entity).await?;
    println!("Forgotten: {insight_entity}");

    // Gone from current state...
    assert!(prime.get_node(&insight_entity).is_none());

    // ...but preserved in history
    let history = prime.history(&insight_entity).await?;
    println!("History entries: {} (audit trail preserved)", history.len());

    prime.shutdown().await?;
    println!("\nDone.");
    Ok(())
}

/// Deterministic fake embedding from text (for demo purposes only).
fn fake_embedding(text: &str) -> Vec<f32> {
    let dim = 32;
    let mut vec = vec![0.0f32; dim];
    for (i, byte) in text.bytes().enumerate() {
        vec[i % dim] += f32::from(byte) / 255.0;
    }
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}
