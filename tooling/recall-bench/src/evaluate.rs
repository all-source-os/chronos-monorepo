//! Evaluation runner — ingests conversations into Prime, runs queries, computes metrics.

use std::path::Path;
use std::time::Instant;

use allsource_core::prime::Prime;
use anyhow::Result;
use serde_json::json;

use crate::datasets::{self, EvalScenario};
use crate::BenchmarkResults;

/// Run LoCoMo evaluation.
pub async fn run_locomo(mode: &str, limit: usize, data_dir: &Path) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_locomo(data_dir, limit).await?;
    run_scenarios("LoCoMo", mode, &scenarios).await
}

/// Run LongMemEval evaluation.
pub async fn run_longmemeval(mode: &str, limit: usize, data_dir: &Path) -> Result<BenchmarkResults> {
    let scenarios = datasets::load_longmemeval(data_dir, limit).await?;
    run_scenarios("LongMemEval", mode, &scenarios).await
}

/// Run cross-reference evaluation (synthetic, no download needed).
pub async fn run_cross_ref(mode: &str, limit: usize) -> Result<BenchmarkResults> {
    let scenarios = generate_cross_ref_scenarios(limit);
    run_scenarios("CrossRef", mode, &scenarios).await
}

/// Core evaluation loop.
async fn run_scenarios(
    dataset_name: &str,
    mode: &str,
    scenarios: &[EvalScenario],
) -> Result<BenchmarkResults> {
    let prime = Prime::open_in_memory().await?;

    let mut total_queries = 0usize;
    let mut total_true_positives = 0usize;
    let mut total_retrieved = 0usize;
    let mut total_relevant = 0usize;
    let mut total_cross_ref_correct = 0usize;
    let mut total_cross_ref_queries = 0usize;
    let mut total_latency_ms = 0.0f64;

    for scenario in scenarios {
        tracing::info!("Ingesting scenario {}", scenario.id);

        // Ingest conversation turns as graph nodes
        let mut fact_nodes = Vec::new();
        for (turn_idx, turn) in scenario.conversation.iter().enumerate() {
            for (fact_idx, fact) in turn.facts.iter().enumerate() {
                let node_id = prime
                    .add_node(
                        "fact",
                        json!({
                            "content": fact,
                            "source": turn.role,
                            "turn": turn_idx,
                            "scenario": scenario.id,
                        }),
                    )
                    .await?;

                #[allow(deprecated)]
                let entity_id =
                    allsource_core::prime::node_entity_id("fact", node_id.as_str());

                // Generate a simple deterministic embedding from the fact text
                let embedding = text_to_embedding(fact);
                prime.embed(&entity_id, Some(fact), embedding).await?;

                fact_nodes.push((entity_id, fact.clone()));

                // Link sequential facts
                if fact_idx > 0 || turn_idx > 0 {
                    if let Some((prev_id, _)) = fact_nodes.get(fact_nodes.len().wrapping_sub(2)) {
                        let _ = prime
                            .add_edge(prev_id, fact_nodes.last().unwrap().0.as_str(), "follows", None)
                            .await;
                    }
                }
            }
        }

        // Run queries
        for query in &scenario.queries {
            total_queries += 1;
            let start = Instant::now();

            let query_embedding = text_to_embedding(&query.question);

            let retrieved_facts: Vec<String> = if mode == "vector-only" {
                // Ablation: vector search only
                let results = prime.vector_search(&query_embedding, 10);
                results
                    .iter()
                    .filter_map(|r| r.text.clone())
                    .collect()
            } else {
                // Full recall: vector + graph + compressed index
                use allsource_core::prime::types::RecallQuery;
                let recall_query = RecallQuery {
                    vector: Some(query_embedding),
                    text: Some(query.question.clone()),
                    depth: 1,
                    top_k: 10,
                    ..RecallQuery::default()
                };
                match prime.recall(recall_query).await {
                    Ok(result) => result
                        .nodes
                        .iter()
                        .filter_map(|sn| sn.node.properties.get("content").and_then(|v| v.as_str()).map(String::from))
                        .collect(),
                    Err(_) => Vec::new(),
                }
            };

            let latency = start.elapsed().as_secs_f64() * 1000.0;
            total_latency_ms += latency;

            // Compute metrics
            let expected: std::collections::HashSet<&str> =
                query.expected_facts.iter().map(String::as_str).collect();
            let retrieved: std::collections::HashSet<&str> =
                retrieved_facts.iter().map(String::as_str).collect();

            let true_positives = expected.intersection(&retrieved).count();
            total_true_positives += true_positives;
            total_retrieved += retrieved.len();
            total_relevant += expected.len();

            if query.cross_domain {
                total_cross_ref_queries += 1;
                // Cross-ref accuracy: did we retrieve facts from both domains?
                if true_positives >= 2 {
                    total_cross_ref_correct += 1;
                }
            }
        }
    }

    let precision = if total_retrieved > 0 {
        total_true_positives as f64 / total_retrieved as f64
    } else {
        0.0
    };
    let recall = if total_relevant > 0 {
        total_true_positives as f64 / total_relevant as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    let cross_ref = if total_cross_ref_queries > 0 {
        Some(total_cross_ref_correct as f64 / total_cross_ref_queries as f64)
    } else {
        None
    };

    Ok(BenchmarkResults {
        dataset: dataset_name.to_string(),
        mode: mode.to_string(),
        conversations: scenarios.len(),
        queries: total_queries,
        precision,
        recall,
        f1,
        cross_ref_accuracy: cross_ref,
        avg_latency_ms: if total_queries > 0 {
            total_latency_ms / total_queries as f64
        } else {
            0.0
        },
    })
}

/// Generate simple deterministic embedding from text.
///
/// Uses character-level hashing to produce a fixed-dim vector.
/// Not semantically meaningful — just gives distinct vectors per text.
fn text_to_embedding(text: &str) -> Vec<f32> {
    let dim = 32;
    let mut vec = vec![0.0f32; dim];
    for (i, byte) in text.bytes().enumerate() {
        vec[i % dim] += f32::from(byte) / 255.0;
    }
    // Normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}

/// Generate cross-reference evaluation scenarios.
fn generate_cross_ref_scenarios(limit: usize) -> Vec<EvalScenario> {
    use crate::datasets::{ConversationTurn, EvalQuery, EvalScenario};

    let domains = [
        ("engineering", "Rust", "systems programming"),
        ("data-science", "Python", "machine learning"),
        ("devops", "Kubernetes", "container orchestration"),
        ("security", "TLS", "encryption protocols"),
        ("product", "OKRs", "goal setting"),
    ];

    let mut scenarios = Vec::new();

    for i in 0..limit.min(5) {
        let (domain_a, tool_a, topic_a) = domains[i];
        let (domain_b, tool_b, topic_b) = domains[(i + 1) % domains.len()];

        scenarios.push(EvalScenario {
            id: format!("crossref-{i}"),
            conversation: vec![
                ConversationTurn {
                    role: "user".into(),
                    content: format!("In {domain_a}, we use {tool_a} for {topic_a}"),
                    facts: vec![format!("{domain_a} uses {tool_a} for {topic_a}")],
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!("In {domain_b}, we use {tool_b} for {topic_b}"),
                    facts: vec![format!("{domain_b} uses {tool_b} for {topic_b}")],
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!(
                        "The {domain_a} team's {tool_a} insights helped {domain_b} improve {topic_b}"
                    ),
                    facts: vec![format!(
                        "{domain_a} {tool_a} insights helped {domain_b} {topic_b}"
                    )],
                },
            ],
            queries: vec![EvalQuery {
                question: format!("How does {domain_a} relate to {domain_b}?"),
                expected_facts: vec![
                    format!("{domain_a} uses {tool_a} for {topic_a}"),
                    format!("{domain_a} {tool_a} insights helped {domain_b} {topic_b}"),
                ],
                cross_domain: true,
            }],
        });
    }

    scenarios
}
