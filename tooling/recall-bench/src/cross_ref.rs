//! Cross-reference accuracy test suite.
//!
//! Replicates zer0dex benchmark methodology: seeds Prime with multi-domain
//! knowledge, runs cross-domain queries, measures retrieval accuracy across
//! three modes: vector-only, vector+graph, vector+graph+compressed-index.

use allsource_core::prime::Prime;
use allsource_core::prime::types::RecallQuery;
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use serde_json::json;
use std::time::Instant;

use crate::BenchmarkResults;

/// Real embedding model for semantic similarity.
struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    fn new() -> Result<Self> {
        tracing::info!("Loading embedding model (first run downloads ~30MB)...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;
        tracing::info!("Embedding model loaded");
        Ok(Self { model })
    }

    fn embed(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let embeddings = self.model.embed(texts.to_vec(), None)?;
        Ok(embeddings)
    }

    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.embed(&[text])?;
        Ok(results.remove(0))
    }
}

/// Domains with their facts.
struct Domain {
    name: &'static str,
    facts: Vec<(&'static str, &'static str)>, // (fact_key, fact_text)
}

/// A cross-domain query with expected facts from both domains.
struct CrossDomainQuery {
    question: String,
    expected_domain_a: String,
    expected_domain_b: String,
}

/// Run the full cross-reference accuracy benchmark.
///
/// Seeds 5 domains, 50+ facts, 20+ cross-domain relationships.
/// Runs 30+ queries. Compares vector-only vs vector+graph vs full recall.
pub async fn run_cross_ref_suite() -> Result<Vec<BenchmarkResults>> {
    let domains = vec![
        Domain {
            name: "engineering",
            facts: vec![
                ("eng-lang", "Engineering team uses Rust for backend services"),
                ("eng-ci", "Engineering runs CI with GitHub Actions"),
                ("eng-db", "Engineering stores events in AllSource Core"),
                ("eng-test", "Engineering requires 80% test coverage"),
                ("eng-review", "Engineering does pair code review"),
                ("eng-deploy", "Engineering deploys to Kubernetes"),
                ("eng-monitor", "Engineering uses Grafana for monitoring"),
                ("eng-sprint", "Engineering runs 2-week sprints"),
                ("eng-oncall", "Engineering has a weekly oncall rotation"),
                ("eng-arch", "Engineering follows clean architecture patterns"),
            ],
        },
        Domain {
            name: "data-science",
            facts: vec![
                ("ds-lang", "Data science uses Python for ML pipelines"),
                ("ds-framework", "Data science uses PyTorch for deep learning"),
                ("ds-data", "Data science processes 10TB of training data daily"),
                ("ds-gpu", "Data science uses A100 GPUs for training"),
                ("ds-notebook", "Data science uses Jupyter for exploration"),
                ("ds-deploy", "Data science deploys models via ONNX"),
                ("ds-metric", "Data science targets >95% AUC on classification"),
                ("ds-feature", "Data science stores features in a feature store"),
                ("ds-ab", "Data science runs A/B tests for model validation"),
                ("ds-version", "Data science versions models with MLflow"),
            ],
        },
        Domain {
            name: "product",
            facts: vec![
                ("prod-method", "Product uses OKRs for quarterly planning"),
                ("prod-tool", "Product manages roadmap in Linear"),
                ("prod-research", "Product does weekly user interviews"),
                ("prod-metric", "Product tracks NPS and activation rate"),
                ("prod-launch", "Product has a 3-stage launch process"),
                ("prod-feedback", "Product collects feedback via Canny"),
                ("prod-priority", "Product uses RICE scoring for prioritization"),
                ("prod-segment", "Product segments users by company size"),
                ("prod-growth", "Product targets 10% MoM growth"),
                ("prod-spec", "Product writes PRDs before engineering starts"),
            ],
        },
        Domain {
            name: "security",
            facts: vec![
                ("sec-auth", "Security requires mTLS between services"),
                ("sec-scan", "Security runs Trivy scans on every image"),
                ("sec-policy", "Security enforces least privilege access"),
                ("sec-audit", "Security does quarterly penetration tests"),
                ("sec-encrypt", "Security encrypts data at rest with AES-256"),
                ("sec-sso", "Security uses SAML SSO for enterprise customers"),
                ("sec-log", "Security ships audit logs to SIEM"),
                ("sec-cert", "Security rotates TLS certificates monthly"),
                ("sec-vuln", "Security has 48-hour SLA for critical CVEs"),
                ("sec-compliance", "Security maintains SOC2 Type II compliance"),
            ],
        },
        Domain {
            name: "marketing",
            facts: vec![
                ("mkt-channel", "Marketing focuses on developer content marketing"),
                ("mkt-blog", "Marketing publishes 2 blog posts per week"),
                ("mkt-social", "Marketing is active on Twitter and LinkedIn"),
                ("mkt-conf", "Marketing sponsors 4 conferences per year"),
                ("mkt-email", "Marketing runs a monthly developer newsletter"),
                ("mkt-seo", "Marketing targets technical keywords for SEO"),
                ("mkt-case", "Marketing produces customer case studies"),
                ("mkt-brand", "Marketing maintains brand guidelines in Figma"),
                ("mkt-metric", "Marketing tracks CAC and LTV ratios"),
                ("mkt-partner", "Marketing has a technology partner program"),
            ],
        },
    ];

    // Cross-domain relationships
    let cross_domain_links: Vec<(&str, &str, &str, &str)> = vec![
        ("engineering", "data-science", "supports", "Engineering provides GPU infrastructure for data science training"),
        ("engineering", "security", "implements", "Engineering implements security's mTLS requirements in all services"),
        ("data-science", "product", "informs", "Data science models inform product's activation rate metrics"),
        ("product", "marketing", "guides", "Product roadmap guides marketing's content calendar"),
        ("security", "engineering", "audits", "Security audits engineering's deployment pipeline quarterly"),
        ("marketing", "product", "reports", "Marketing's CAC data informs product's pricing decisions"),
        ("engineering", "product", "delivers", "Engineering delivers features from product's RICE-prioritized backlog"),
        ("data-science", "security", "analyzes", "Data science analyzes security's audit logs for anomaly detection"),
        ("security", "marketing", "reviews", "Security reviews marketing's data collection for compliance"),
        ("marketing", "engineering", "requests", "Marketing requests engineering support for landing page deployments"),
        ("product", "security", "requires", "Product requires security sign-off before launching enterprise features"),
        ("data-science", "marketing", "provides", "Data science provides marketing with user segmentation models"),
        ("engineering", "marketing", "builds", "Engineering builds marketing's analytics dashboard in Grafana"),
        ("security", "data-science", "governs", "Security governs data science's access to production data"),
        ("product", "engineering", "specs", "Product's PRDs define engineering's sprint priorities"),
        ("marketing", "data-science", "feeds", "Marketing's A/B test results feed data science's user behavior models"),
        ("engineering", "engineering", "internal", "Engineering team knowledge sharing via weekly tech talks"),
        ("product", "product", "internal", "Product does cross-team OKR alignment quarterly"),
        ("security", "security", "internal", "Security team rotates through different audit domains"),
        ("data-science", "data-science", "internal", "Data science collaborates across NLP and CV sub-teams"),
    ];

    // Load real embedding model
    let mut embedder = Embedder::new()?;

    // Seed Prime
    let prime = Prime::open_in_memory().await?;
    let mut domain_nodes: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    // Ingest all facts with real embeddings
    for domain in &domains {
        let mut node_ids = Vec::new();
        for (key, fact_text) in &domain.facts {
            let node_id = prime
                .add_node(
                    domain.name,
                    json!({
                        "content": fact_text,
                        "key": key,
                        "domain": domain.name,
                    }),
                )
                .await?;
            #[allow(deprecated)]
            let entity_id = allsource_core::prime::node_entity_id(domain.name, node_id.as_str());

            let embedding = embedder.embed_one(fact_text)?;
            prime.embed(&entity_id, Some(fact_text), embedding).await?;

            node_ids.push(entity_id);
        }
        domain_nodes.insert(domain.name.to_string(), node_ids);
    }

    // Create cross-domain edges
    for (from_domain, to_domain, relation, description) in &cross_domain_links {
        if let (Some(from_nodes), Some(to_nodes)) = (domain_nodes.get(*from_domain), domain_nodes.get(*to_domain)) {
            if let (Some(from), Some(to)) = (from_nodes.first(), to_nodes.first()) {
                let _ = prime.add_edge(from, to, relation, Some(json!({"description": description}))).await;
            }
        }
    }

    tracing::info!(
        "Seeded: {} domains, {} facts, {} cross-domain links",
        domains.len(),
        domains.iter().map(|d| d.facts.len()).sum::<usize>(),
        cross_domain_links.len()
    );

    // Build queries (30+)
    let mut queries: Vec<CrossDomainQuery> = Vec::new();
    let domain_names: Vec<&str> = domains.iter().map(|d| d.name).collect();
    for i in 0..domain_names.len() {
        for j in (i + 1)..domain_names.len() {
            let a = domain_names[i];
            let b = domain_names[j];
            queries.push(CrossDomainQuery {
                question: format!("How does {a} relate to {b}?"),
                expected_domain_a: a.to_string(),
                expected_domain_b: b.to_string(),
            });
            queries.push(CrossDomainQuery {
                question: format!("What does {a} share with {b}?"),
                expected_domain_a: a.to_string(),
                expected_domain_b: b.to_string(),
            });
            queries.push(CrossDomainQuery {
                question: format!("What connections exist between {b} and {a}?"),
                expected_domain_a: b.to_string(),
                expected_domain_b: a.to_string(),
            });
        }
    }

    tracing::info!("Generated {} cross-domain queries", queries.len());

    // Run three modes
    let mut all_results = Vec::new();

    for mode in &["vector-naive", "vector-cross-domain", "vector+graph", "full-recall"] {
        let mut correct = 0usize;
        let total = queries.len();
        let mut total_latency_ms = 0.0;

        for query in &queries {
            let embedding = embedder.embed_one(&query.question)?;
            let start = Instant::now();

            let found_domains: std::collections::HashSet<String> = if *mode == "vector-naive" {
                // Pure HNSW — no domain balancing (the old 0% baseline)
                let results = prime.vector_search(&embedding, 10);
                results
                    .iter()
                    .filter_map(|r| {
                        r.text
                            .as_ref()
                            .and_then(|_| r.id.split(':').nth(2).map(String::from))
                    })
                    .collect()
            } else if *mode == "vector-cross-domain" {
                // Domain-balanced vector search (the fix)
                let results = prime.vector_search_cross_domain(&embedding, 10);
                results
                    .iter()
                    .filter_map(|r| {
                        r.text
                            .as_ref()
                            .and_then(|_| r.id.split(':').nth(2).map(String::from))
                    })
                    .collect()
            } else {
                let depth = if *mode == "full-recall" { 1 } else { 0 };
                let recall_query = RecallQuery {
                    vector: Some(embedding),
                    text: Some(query.question.clone()),
                    depth,
                    top_k: 15,
                    ..RecallQuery::default()
                };
                match prime.recall(recall_query).await {
                    Ok(result) => result
                        .nodes
                        .iter()
                        .map(|sn| sn.node.node_type.clone())
                        .collect(),
                    Err(_) => std::collections::HashSet::new(),
                }
            };

            let elapsed = start.elapsed();
            total_latency_ms += elapsed.as_secs_f64() * 1000.0;

            // Did we find results from BOTH expected domains?
            let has_a = found_domains.contains(&query.expected_domain_a);
            let has_b = found_domains.contains(&query.expected_domain_b);
            if has_a && has_b {
                correct += 1;
            }
        }

        let accuracy = correct as f64 / total as f64;
        let avg_latency = total_latency_ms / total as f64;
        all_results.push(BenchmarkResults {
            dataset: "CrossRef".to_string(),
            mode: mode.to_string(),
            conversations: 5,
            queries: total,
            precision: accuracy,
            recall: accuracy,
            f1: accuracy,
            cross_ref_accuracy: Some(accuracy),
            avg_latency_ms: avg_latency,
        });
    }

    Ok(all_results)
}

/// Deterministic embedding from text (reproducible, seeded).
fn deterministic_embedding(text: &str, dim: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dim];
    // Use a simple hash-based approach for reproducibility
    let mut hash: u64 = 5381;
    for byte in text.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u64::from(byte));
        let idx = (hash as usize) % dim;
        vec[idx] += 1.0;
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
