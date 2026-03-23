//! Evaluation runner — ingests conversations into Prime, runs queries, computes metrics.
//!
//! Uses semantic similarity (MiniLM embeddings) for scoring instead of substring
//! matching, so "GPS system" matches "GPS not functioning" correctly.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use allsource_core::prime::Prime;
use allsource_core::prime::types::RecallQuery;
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use serde_json::json;

use crate::datasets::{self, EvalScenario};
use crate::BenchmarkResults;

/// Embedder for both ingestion and evaluation scoring.
struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    fn new() -> Result<Self> {
        tracing::info!("Loading embedding model...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }

    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.model.embed(vec![text], None)?;
        Ok(results.remove(0))
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < f32::EPSILON || norm_b < f32::EPSILON {
        return 0.0;
    }
    f64::from(dot / (norm_a * norm_b))
}

/// Semantic match score: how well does the retrieved text match the expected answer?
///
/// Returns the maximum cosine similarity between the expected answer embedding
/// and any of the retrieved text embeddings.
fn semantic_match_score(
    expected_embedding: &[f32],
    retrieved_texts: &[String],
    embedder: &mut Embedder,
) -> f64 {
    if retrieved_texts.is_empty() {
        return 0.0;
    }

    retrieved_texts
        .iter()
        .filter_map(|text| {
            let emb = embedder.embed_one(text).ok()?;
            Some(cosine_similarity(expected_embedding, &emb))
        })
        .fold(0.0f64, f64::max)
}

/// Hit threshold: a semantic similarity score above this counts as a successful retrieval.
const HIT_THRESHOLD: f64 = 0.5;

// =============================================================================
// Public API
// =============================================================================

/// Run LongMemEval evaluation.
pub async fn run_longmemeval(
    mode: &str,
    limit: usize,
    data_dir: &Path,
) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_longmemeval(data_dir, limit).await?;
    run_scenarios("LongMemEval", mode, &scenarios).await
}

/// Run LoCoMo evaluation (synthetic fallback).
pub async fn run_locomo(
    mode: &str,
    limit: usize,
    data_dir: &Path,
) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_locomo(data_dir, limit).await?;
    run_scenarios("LoCoMo", mode, &scenarios).await
}

/// Core evaluation loop.
async fn run_scenarios(
    dataset_name: &str,
    mode: &str,
    scenarios: &[EvalScenario],
) -> Result<BenchmarkResults> {
    let mut embedder = Embedder::new()?;
    let prime = Prime::open_in_memory().await?;

    let mut total_queries = 0usize;
    let mut total_latency_ms = 0.0f64;
    let mut type_scores: HashMap<String, Vec<f64>> = HashMap::new();

    for (scenario_idx, scenario) in scenarios.iter().enumerate() {
        if scenario_idx % 50 == 0 {
            tracing::info!(
                "Processing scenario {}/{} ({})",
                scenario_idx + 1,
                scenarios.len(),
                scenario.id
            );
        }

        // Ingest conversation turns (sentence-chunked) with session IDs
        let mut prev_entity_id: Option<String> = None;

        for turn in &scenario.conversation {
            if turn.role == "system" || turn.content.is_empty() {
                continue;
            }

            let node_id = prime
                .add_node(
                    "memory",
                    json!({
                        "content": turn.content,
                        "role": turn.role,
                        "scenario": scenario.id,
                        "session_id": turn.session_id,
                    }),
                )
                .await?;

            #[allow(deprecated)]
            let entity_id =
                allsource_core::prime::node_entity_id("memory", node_id.as_str());

            let embedding = embedder.embed_one(&turn.content)?;
            prime
                .embed(&entity_id, Some(&turn.content), embedding)
                .await?;

            // Link sequential chunks for graph traversal
            if let Some(ref prev) = prev_entity_id {
                let _ = prime.add_edge(prev, &entity_id, "follows", None).await;
            }
            prev_entity_id = Some(entity_id);
        }

        // Run queries
        for query in &scenario.queries {
            total_queries += 1;
            let start = Instant::now();

            let query_embedding = embedder.embed_one(&query.question)?;

            // Retrieve: get both texts and session IDs from results
            let retrieved: Vec<(String, String)> = match mode {
                "vector-only" => {
                    // Vector search returns VectorSearchResult — no session_id directly.
                    // We get the text and look up session_id from node properties.
                    let results = prime.vector_search(&query_embedding, 15);
                    results
                        .into_iter()
                        .filter_map(|r| {
                            let text = r.text?;
                            // Extract session_id from the node if possible
                            let node_id = r.id.strip_prefix("vec:").unwrap_or(&r.id);
                            let session_id = prime
                                .get_node(node_id)
                                .and_then(|n| {
                                    n.properties
                                        .get("session_id")
                                        .and_then(|v| v.as_str())
                                        .map(String::from)
                                })
                                .unwrap_or_default();
                            Some((text, session_id))
                        })
                        .collect()
                }
                _ => {
                    let recall_query = RecallQuery {
                        vector: Some(query_embedding.clone()),
                        text: Some(query.question.clone()),
                        depth: if mode == "full-recall" { 2 } else { 1 },
                        top_k: 15,
                        ..RecallQuery::default()
                    };
                    match prime.recall(recall_query).await {
                        Ok(result) => result
                            .nodes
                            .iter()
                            .filter_map(|sn| {
                                let text = sn
                                    .node
                                    .properties
                                    .get("content")
                                    .and_then(|v| v.as_str())
                                    .map(String::from)?;
                                let session_id = sn
                                    .node
                                    .properties
                                    .get("session_id")
                                    .and_then(|v| v.as_str())
                                    .map(String::from)
                                    .unwrap_or_default();
                                Some((text, session_id))
                            })
                            .collect(),
                        Err(_) => Vec::new(),
                    }
                }
            };

            let retrieved_texts: Vec<String> =
                retrieved.iter().map(|(t, _)| t.clone()).collect();
            let retrieved_session_ids: Vec<String> =
                retrieved.iter().map(|(_, s)| s.clone()).collect();

            let latency = start.elapsed().as_secs_f64() * 1000.0;
            total_latency_ms += latency;

            // Score using TWO metrics:
            // 1. Session-level: did we retrieve from the correct answer session?
            // 2. Semantic: how well does retrieved text match the expected answer?
            let is_abstention = query.question_type.contains("abs")
                || query.question_type.contains("abstention");

            let score = if is_abstention {
                // Abstention: score 1.0 if retrieved texts have LOW similarity to the answer
                let expected_emb = embedder.embed_one(&query.expected_facts[0])?;
                let max_sim =
                    semantic_match_score(&expected_emb, &retrieved_texts, &mut embedder);
                if max_sim < HIT_THRESHOLD {
                    1.0
                } else {
                    0.0
                }
            } else {
                // Combined scoring: session-level hit + semantic similarity

                // Session-level: did we retrieve from any answer session?
                let session_hit = if !query.answer_session_ids.is_empty() {
                    let found_answer_session = retrieved_session_ids
                        .iter()
                        .any(|sid| query.answer_session_ids.contains(sid));
                    if found_answer_session { 1.0 } else { 0.0 }
                } else {
                    0.0 // No session IDs to check
                };

                // Semantic: best match between expected answer and retrieved chunks
                let mut semantic_score: f64 = 0.0;
                for expected in &query.expected_facts {
                    let expected_emb = embedder.embed_one(expected)?;
                    let sim =
                        semantic_match_score(&expected_emb, &retrieved_texts, &mut embedder);
                    semantic_score = semantic_score.max(sim);
                }

                // Combined: session hit is binary, semantic is continuous
                // If we found the right session, that's already good (0.5 base)
                // Semantic adds up to 0.5 more
                if !query.answer_session_ids.is_empty() {
                    session_hit * 0.5 + semantic_score.min(1.0) * 0.5
                } else {
                    // No session IDs available — pure semantic scoring
                    if semantic_score >= HIT_THRESHOLD {
                        1.0
                    } else {
                        semantic_score
                    }
                }
            };

            let q_type = if query.question_type.is_empty() {
                "unknown".to_string()
            } else {
                query.question_type.clone()
            };
            type_scores.entry(q_type).or_default().push(score);
        }
    }

    // Compute aggregates
    let all_scores: Vec<f64> = type_scores.values().flatten().copied().collect();
    let overall_recall = if all_scores.is_empty() {
        0.0
    } else {
        all_scores.iter().sum::<f64>() / all_scores.len() as f64
    };
    let pass_rate = if all_scores.is_empty() {
        0.0
    } else {
        all_scores.iter().filter(|&&s| s >= 0.75).count() as f64 / all_scores.len() as f64
    };

    // Log per-type results
    let mut type_names: Vec<&String> = type_scores.keys().collect();
    type_names.sort();
    for type_name in &type_names {
        let scores = &type_scores[*type_name];
        let avg = scores.iter().sum::<f64>() / scores.len().max(1) as f64;
        let pass = scores.iter().filter(|&&s| s >= 0.75).count() as f64
            / scores.len().max(1) as f64;
        tracing::info!(
            "  {type_name}: n={} recall={:.1}% pass@0.75={:.1}%",
            scores.len(),
            avg * 100.0,
            pass * 100.0,
        );
    }

    // Multi-session cross-ref accuracy
    let cross_ref_scores: Vec<f64> = type_scores
        .iter()
        .filter(|(k, _)| k.contains("multi-session"))
        .flat_map(|(_, v)| v.iter().copied())
        .collect();
    let cross_ref = if cross_ref_scores.is_empty() {
        None
    } else {
        Some(
            cross_ref_scores
                .iter()
                .filter(|&&s| s >= 0.75)
                .count() as f64
                / cross_ref_scores.len() as f64,
        )
    };

    tracing::info!(
        "{dataset_name} ({mode}): overall_recall={:.1}% pass@0.75={:.1}% queries={total_queries}",
        overall_recall * 100.0,
        pass_rate * 100.0,
    );

    prime.shutdown().await?;

    Ok(BenchmarkResults {
        dataset: dataset_name.to_string(),
        mode: mode.to_string(),
        conversations: scenarios.len(),
        queries: total_queries,
        precision: pass_rate,
        recall: overall_recall,
        f1: if pass_rate + overall_recall > 0.0 {
            2.0 * pass_rate * overall_recall / (pass_rate + overall_recall)
        } else {
            0.0
        },
        cross_ref_accuracy: cross_ref,
        avg_latency_ms: if total_queries > 0 {
            total_latency_ms / total_queries as f64
        } else {
            0.0
        },
    })
}
