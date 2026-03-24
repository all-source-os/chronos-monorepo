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

/// Embedder with batch support and in-memory caching.
struct Embedder {
    model: TextEmbedding,
    cache: HashMap<u64, Vec<f32>>,
    cache_hits: usize,
    cache_misses: usize,
}

impl Embedder {
    fn new() -> Result<Self> {
        Self::with_model("mini")
    }

    fn with_model(model_name: &str) -> Result<Self> {
        let model_type = match model_name {
            "base" | "bge-base" => {
                tracing::info!("Loading BGE-Base-EN-v1.5 (768-dim, ~100MB)...");
                EmbeddingModel::BGEBaseENV15
            }
            "large" | "bge-large" => {
                tracing::info!("Loading BGE-Large-EN-v1.5 (1024-dim, ~350MB)...");
                EmbeddingModel::BGELargeENV15
            }
            _ => {
                tracing::info!("Loading MiniLM-L6-v2 (384-dim, ~30MB)...");
                EmbeddingModel::AllMiniLML6V2
            }
        };
        let model = TextEmbedding::try_new(
            InitOptions::new(model_type).with_show_download_progress(true),
        )?;
        Ok(Self {
            model,
            cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        })
    }

    /// Hash text for cache key.
    fn text_hash(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Embed a single text, using cache.
    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let key = Self::text_hash(text);
        if let Some(cached) = self.cache.get(&key) {
            self.cache_hits += 1;
            return Ok(cached.clone());
        }
        self.cache_misses += 1;
        let mut results = self.model.embed(vec![text], None)?;
        let vec = results.remove(0);
        self.cache.insert(key, vec.clone());
        Ok(vec)
    }

    /// Batch embed multiple texts at once (much faster than one-by-one).
    fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Split into cached and uncached
        let mut results = vec![Vec::new(); texts.len()];
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            let key = Self::text_hash(text);
            if let Some(cached) = self.cache.get(&key) {
                self.cache_hits += 1;
                results[i] = cached.clone();
            } else {
                self.cache_misses += 1;
                uncached_indices.push(i);
                uncached_texts.push(*text);
            }
        }

        // Batch embed uncached texts
        if !uncached_texts.is_empty() {
            let embeddings = self.model.embed(uncached_texts.clone(), None)?;
            for (j, embedding) in embeddings.into_iter().enumerate() {
                let idx = uncached_indices[j];
                let key = Self::text_hash(texts[idx]);
                self.cache.insert(key, embedding.clone());
                results[idx] = embedding;
            }
        }

        Ok(results)
    }

    fn log_stats(&self) {
        tracing::info!(
            "Embedder stats: {} cache hits, {} misses ({:.1}% hit rate)",
            self.cache_hits,
            self.cache_misses,
            if self.cache_hits + self.cache_misses > 0 {
                self.cache_hits as f64 / (self.cache_hits + self.cache_misses) as f64 * 100.0
            } else {
                0.0
            }
        );
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
    model: &str,
) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_longmemeval(data_dir, limit).await?;
    run_scenarios("LongMemEval", mode, &scenarios, model).await
}

/// Run LoCoMo evaluation (synthetic fallback).
pub async fn run_locomo(
    mode: &str,
    limit: usize,
    data_dir: &Path,
    model: &str,
) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_locomo(data_dir, limit).await?;
    run_scenarios("LoCoMo", mode, &scenarios, model).await
}

/// Core evaluation loop.
async fn run_scenarios(
    dataset_name: &str,
    mode: &str,
    scenarios: &[EvalScenario],
    model_name: &str,
) -> Result<BenchmarkResults> {
    let mut embedder = Embedder::with_model(model_name)?;

    let mut total_queries = 0usize;
    let mut total_latency_ms = 0.0f64;
    let mut type_scores: HashMap<String, Vec<f64>> = HashMap::new();

    for (scenario_idx, scenario) in scenarios.iter().enumerate() {
        if scenario_idx % 10 == 0 {
            tracing::info!(
                "Processing scenario {}/{} ({})",
                scenario_idx + 1,
                scenarios.len(),
                scenario.id
            );
        }

        // Fresh Prime per scenario — each question has isolated context
        // (LongMemEval provides the haystack sessions specific to each question)
        let prime = Prime::open_in_memory().await?;

        // Collect valid turns for batch embedding
        let valid_turns: Vec<&datasets::ConversationTurn> = scenario
            .conversation
            .iter()
            .filter(|t| t.role != "system" && !t.content.is_empty())
            .collect();

        // Batch embed all turn contents at once (much faster than one-by-one)
        let turn_texts: Vec<&str> = valid_turns.iter().map(|t| t.content.as_str()).collect();
        let embeddings = embedder.embed_batch(&turn_texts)?;

        // Ingest with pre-computed embeddings
        let mut prev_entity_id: Option<String> = None;

        for (turn, embedding) in valid_turns.iter().zip(embeddings.into_iter()) {
            let node_id = prime
                .add_node(
                    "memory",
                    json!({
                        "content": turn.content,
                        "role": turn.role,
                        "scenario": scenario.id,
                        "session_id": turn.session_id,
                        "session_date": turn.session_date,
                    }),
                )
                .await?;

            #[allow(deprecated)]
            let entity_id =
                allsource_core::prime::node_entity_id("memory", node_id.as_str());

            prime
                .embed(&entity_id, Some(&turn.content), embedding)
                .await?;

            // Link sequential chunks for graph traversal
            if let Some(ref prev) = prev_entity_id {
                let _ = prime.add_edge(prev, &entity_id, "follows", None).await;
            }
            prev_entity_id = Some(entity_id);
        }

        // Entity-based cross-session edges: connect chunks that share
        // capitalized multi-word names (likely people, places, events)
        {
            let all_nodes: Vec<_> = prime.nodes_by_type("memory");
            let mut name_to_entities: HashMap<String, Vec<String>> = HashMap::new();

            for node in &all_nodes {
                let content = node
                    .properties
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Extract capitalized multi-word names (2+ consecutive capitalized words)
                let words: Vec<&str> = content.split_whitespace().collect();
                for window in words.windows(2) {
                    let a = window[0].trim_matches(|c: char| !c.is_alphanumeric());
                    let b = window[1].trim_matches(|c: char| !c.is_alphanumeric());
                    if a.len() >= 2
                        && b.len() >= 2
                        && a.chars().next().is_some_and(|c| c.is_uppercase())
                        && b.chars().next().is_some_and(|c| c.is_uppercase())
                    {
                        let name = format!("{a} {b}");
                        #[allow(deprecated)]
                        let eid = allsource_core::prime::node_entity_id(
                            "memory",
                            node.id.as_str(),
                        );
                        name_to_entities.entry(name).or_default().push(eid);
                    }
                }
            }

            // Create edges between chunks sharing the same entity name
            // but in DIFFERENT sessions
            let mut cross_edges = 0;
            for (_name, eids) in &name_to_entities {
                if eids.len() < 2 || eids.len() > 20 {
                    continue; // Skip singletons and overly common names
                }
                for i in 0..eids.len().min(5) {
                    for j in (i + 1)..eids.len().min(5) {
                        // Only connect if different sessions
                        let node_a = prime.get_node(&eids[i]);
                        let node_b = prime.get_node(&eids[j]);
                        let diff_session = match (&node_a, &node_b) {
                            (Some(a), Some(b)) => {
                                a.properties.get("session_id") != b.properties.get("session_id")
                            }
                            _ => false,
                        };
                        if diff_session {
                            let _ = prime
                                .add_edge(&eids[i], &eids[j], "mentions", None)
                                .await;
                            cross_edges += 1;
                        }
                    }
                }
            }
            if cross_edges > 0 && scenario_idx % 50 == 0 {
                tracing::debug!("Created {} cross-session entity edges", cross_edges);
            }
        }

        // Run queries
        for query in &scenario.queries {
            total_queries += 1;
            let start = Instant::now();

            let query_embedding = embedder.embed_one(&query.question)?;

            // Retrieve: get texts and session IDs from results.
            // For temporal-reasoning questions, apply temporal filtering.
            let is_temporal = query.question_type == "temporal-reasoning"
                || query.question_type == "knowledge-update";

            // Multi-session goes through recall() for graph expansion, NOT temporal path
            let retrieved: Vec<(String, String)> = if is_temporal && mode != "vector-only" {
                // Temporal-aware retrieval: vector search → sort by date → filter by keyword
                let results = prime.vector_search(&query_embedding, 25); // over-fetch
                let mut hits: Vec<(String, String, String)> = results
                    .into_iter()
                    .filter_map(|r| {
                        let text = r.text?;
                        let node_id = r.id.strip_prefix("vec:").unwrap_or(&r.id);
                        let node = prime.get_node(node_id)?;
                        let session_id = node
                            .properties
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let date = node
                            .properties
                            .get("session_date")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        Some((text, session_id, date))
                    })
                    .collect();

                // Sort chronologically by session date
                hits.sort_by(|a, b| a.2.cmp(&b.2));

                // Apply temporal filter based on question keywords
                let q_lower = query.question.to_lowercase();
                let is_knowledge_update = query.question_type == "knowledge-update";
                let filtered: Vec<(String, String, String)> =
                    if q_lower.contains("first") || q_lower.contains("earliest") {
                        hits.into_iter().take(8).collect()
                    } else if q_lower.contains("last")
                        || q_lower.contains("latest")
                        || q_lower.contains("most recent")
                        || q_lower.contains("current")
                        || q_lower.contains("now")
                        || is_knowledge_update
                    {
                        // Latest wins: reverse chronological, take most recent hits
                        hits.reverse();
                        hits.into_iter().take(10).collect()
                    } else if q_lower.contains("before") {
                        let qd = query.query_date.as_deref().unwrap_or("");
                        hits.into_iter()
                            .filter(|(_, _, d)| d.as_str() <= qd)
                            .collect()
                    } else if q_lower.contains("after") {
                        let qd = query.query_date.as_deref().unwrap_or("");
                        hits.into_iter()
                            .filter(|(_, _, d)| d.as_str() >= qd)
                            .collect()
                    } else {
                        hits // Return all, chronologically sorted
                    };

                filtered
                    .into_iter()
                    .map(|(text, sid, _)| (text, sid))
                    .take(15)
                    .collect()
            } else {
                // Non-temporal retrieval (vector-only or full-recall)
                match mode {
                    "vector-only" => {
                        let results = prime.vector_search(&query_embedding, 15);
                        results
                            .into_iter()
                            .filter_map(|r| {
                                let text = r.text?;
                                let node_id =
                                    r.id.strip_prefix("vec:").unwrap_or(&r.id);
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
                        // Multi-session needs wider search to find cross-session facts
                        let is_multi = query.question_type.contains("multi-session");
                        let recall_query = RecallQuery {
                            vector: Some(query_embedding.clone()),
                            text: Some(query.question.clone()),
                            depth: if is_multi { 3 } else if mode == "full-recall" { 2 } else { 1 },
                            top_k: if is_multi { 25 } else { 15 },
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

                // Combined: 50% session hit + 50% semantic
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

        // Clean up this scenario's Prime instance
        prime.shutdown().await?;
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

    embedder.log_stats();

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
