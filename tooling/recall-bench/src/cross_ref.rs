//! Cross-reference accuracy test suite.
//!
//! Inspired by zer0dex's evaluation methodology (n=97, 86 memories):
//! - Direct recall: can the system find a specific fact?
//! - Cross-reference: can the system connect facts from different contexts?
//! - Negative: does the system correctly reject absent knowledge?
//!
//! Uses real embeddings (MiniLM-L6-v2) and natural-language queries that
//! don't contain domain keywords — testing actual semantic understanding.

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

    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.model.embed(vec![text], None)?;
        Ok(results.remove(0))
    }
}

/// A test case with question, expected facts, and category.
#[derive(Debug, Clone)]
struct TestCase {
    question: String,
    /// Facts that MUST appear in retrieval for a correct answer.
    expected_facts: Vec<String>,
    /// Test category for stratified reporting.
    category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    DirectRecall,
    CrossReference,
    Negative,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Self::DirectRecall => "direct-recall",
            Self::CrossReference => "cross-reference",
            Self::Negative => "negative",
        }
    }
}

/// A memory fact to ingest.
struct Fact {
    text: String,
    context: String, // e.g. "project", "person", "process"
    domain: String,
}

/// Run the full cross-reference accuracy benchmark.
///
/// Modeled after zer0dex's methodology:
/// - ~90 facts across 6 contexts (like zer0dex's 86 memories)
/// - ~30 direct recall, ~15 cross-reference, ~5 negative queries
/// - Cross-ref questions use IMPLICIT connections (no domain names in query)
/// - Scoring: % of expected facts found in retrieval (substring match)
pub async fn run_cross_ref_suite() -> Result<Vec<BenchmarkResults>> {
    let mut embedder = Embedder::new()?;

    // Build the memory corpus — simulates an agent's accumulated knowledge
    // about a software company (similar to zer0dex's personal memory store)
    let facts = build_corpus();
    let test_cases = build_test_cases();

    tracing::info!(
        "Corpus: {} facts, {} test cases ({} direct, {} cross-ref, {} negative)",
        facts.len(),
        test_cases.len(),
        test_cases.iter().filter(|t| t.category == Category::DirectRecall).count(),
        test_cases.iter().filter(|t| t.category == Category::CrossReference).count(),
        test_cases.iter().filter(|t| t.category == Category::Negative).count(),
    );

    // Seed Prime with facts — track actual entity IDs for edge creation
    let prime = Prime::open_in_memory().await?;
    let mut entity_ids: Vec<String> = Vec::new();

    for fact in &facts {
        let node_id = prime
            .add_node(
                &fact.context,
                json!({
                    "content": fact.text,
                    "domain": fact.domain,
                }),
            )
            .await?;

        #[allow(deprecated)]
        let entity_id = allsource_core::prime::node_entity_id(&fact.context, node_id.as_str());
        let embedding = embedder.embed_one(&fact.text)?;
        prime.embed(&entity_id, Some(&fact.text), embedding).await?;
        entity_ids.push(entity_id);
    }

    // Create cross-domain edges using actual entity IDs
    create_relationship_edges(&prime, &facts, &entity_ids).await?;

    tracing::info!("Seeded {} facts with embeddings + edges", facts.len());

    // Run four modes
    let mut all_results = Vec::new();

    for mode in &["vector-naive", "vector-cross-domain", "vector+graph", "full-recall"] {
        let mut category_scores: std::collections::HashMap<&str, Vec<f64>> =
            std::collections::HashMap::new();
        let mut total_latency_ms = 0.0;

        for test in &test_cases {
            let start = Instant::now();

            let retrieved_texts = retrieve(&prime, &mut embedder, *mode, &test.question).await?;

            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            total_latency_ms += elapsed;

            // Score: what % of expected facts were found?
            let score = if test.category == Category::Negative {
                // Negative: score 1.0 if NO relevant content returned
                if retrieved_texts.is_empty() {
                    1.0
                } else {
                    // Check if any retrieved text mentions the negative topic
                    let any_match = test.expected_facts.iter().any(|neg| {
                        retrieved_texts
                            .iter()
                            .any(|r| r.to_lowercase().contains(&neg.to_lowercase()))
                    });
                    if any_match { 0.0 } else { 1.0 }
                }
            } else {
                // Direct/Cross-ref: % of expected facts found
                let found = test
                    .expected_facts
                    .iter()
                    .filter(|expected| {
                        retrieved_texts.iter().any(|retrieved| {
                            retrieved
                                .to_lowercase()
                                .contains(&expected.to_lowercase())
                        })
                    })
                    .count();
                found as f64 / test.expected_facts.len().max(1) as f64
            };

            category_scores
                .entry(test.category.label())
                .or_default()
                .push(score);
        }

        // Compute aggregates
        let all_scores: Vec<f64> = category_scores.values().flatten().copied().collect();
        let overall_recall = all_scores.iter().sum::<f64>() / all_scores.len().max(1) as f64;
        let pass_rate = all_scores.iter().filter(|&&s| s >= 0.75).count() as f64
            / all_scores.len().max(1) as f64;

        let cross_ref_scores = category_scores.get("cross-reference").cloned().unwrap_or_default();
        let cross_ref_accuracy = if cross_ref_scores.is_empty() {
            None
        } else {
            Some(
                cross_ref_scores.iter().filter(|&&s| s >= 0.75).count() as f64
                    / cross_ref_scores.len() as f64,
            )
        };

        let direct_scores = category_scores.get("direct-recall").cloned().unwrap_or_default();
        let direct_recall = if direct_scores.is_empty() {
            0.0
        } else {
            direct_scores.iter().sum::<f64>() / direct_scores.len() as f64
        };

        tracing::info!(
            "{mode}: recall={:.1}% pass_rate={:.1}% cross_ref={:.1}% direct={:.1}% latency={:.1}ms",
            overall_recall * 100.0,
            pass_rate * 100.0,
            cross_ref_accuracy.unwrap_or(0.0) * 100.0,
            direct_recall * 100.0,
            total_latency_ms / test_cases.len() as f64,
        );

        all_results.push(BenchmarkResults {
            dataset: "CrossRef-v2".to_string(),
            mode: mode.to_string(),
            conversations: facts.len(),
            queries: test_cases.len(),
            precision: pass_rate,
            recall: overall_recall,
            f1: if pass_rate + overall_recall > 0.0 {
                2.0 * pass_rate * overall_recall / (pass_rate + overall_recall)
            } else {
                0.0
            },
            cross_ref_accuracy,
            avg_latency_ms: total_latency_ms / test_cases.len().max(1) as f64,
        });
    }

    Ok(all_results)
}

/// Retrieve facts using the specified mode.
async fn retrieve(
    prime: &Prime,
    embedder: &mut Embedder,
    mode: &str,
    question: &str,
) -> Result<Vec<String>> {
    let embedding = embedder.embed_one(question)?;

    match mode {
        "vector-naive" => {
            let results = prime.vector_search(&embedding, 10);
            Ok(results.into_iter().filter_map(|r| r.text).collect())
        }
        "vector-cross-domain" => {
            let results = prime.vector_search_cross_domain(&embedding, 10);
            Ok(results.into_iter().filter_map(|r| r.text).collect())
        }
        "vector+graph" | "full-recall" => {
            let depth = if mode == "full-recall" { 2 } else { 1 };
            let query = RecallQuery {
                vector: Some(embedding),
                text: Some(question.to_string()),
                depth,
                top_k: 10,
                ..RecallQuery::default()
            };
            match prime.recall(query).await {
                Ok(result) => Ok(result
                    .nodes
                    .iter()
                    .filter_map(|sn| {
                        sn.node
                            .properties
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .collect()),
                Err(_) => Ok(Vec::new()),
            }
        }
        _ => Ok(Vec::new()),
    }
}

/// Build the memory corpus — 90 facts about a fictional software company.
///
/// Facts span 6 contexts: people, projects, processes, tools, incidents, decisions.
/// Cross-domain connections are implicit (shared entities, not explicit links).
fn build_corpus() -> Vec<Fact> {
    vec![
        // === People (15 facts) ===
        Fact { text: "Alice Chen is the VP of Engineering, she joined in 2023".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Bob Martinez leads the data platform team".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Carol Wu is the head of product, previously at Stripe".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Dave Kim is the security architect, holds CISSP certification".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Eve Johnson is the marketing director, focuses on developer relations".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Frank Lee manages the frontend team, expert in React and TypeScript".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Grace Park is the DevOps lead, manages the Kubernetes infrastructure".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Henry Zhao is a senior ML engineer on Bob's data platform team".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Alice Chen reports directly to the CEO and has budget authority".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Carol Wu and Alice Chen meet weekly to align product and engineering priorities".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Dave Kim conducted the Q3 penetration test and found 3 critical vulnerabilities".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Eve Johnson organized the annual developer conference with 500 attendees".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Frank Lee rewrote the dashboard frontend, reducing load time by 60%".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Grace Park automated the deployment pipeline, reducing release time from 4 hours to 15 minutes".into(), context: "person".into(), domain: "people".into() },
        Fact { text: "Henry Zhao built the recommendation engine that increased user engagement by 25%".into(), context: "person".into(), domain: "people".into() },

        // === Projects (15 facts) ===
        Fact { text: "Project Mercury is the real-time analytics platform, built with Rust and Arrow".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Venus is the customer-facing dashboard redesign".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mars is the SOC2 compliance initiative led by the security team".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mercury processes 500K events per second in production".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Venus uses React Server Components and was shipped in Q2".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mars required 6 months of audit preparation and passed on first attempt".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mercury was approved by Alice Chen and uses Bob's data platform infrastructure".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Venus was designed by Carol Wu based on user interview findings".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mars uncovered the same vulnerabilities Dave Kim found in the Q3 pen test".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "The mobile app project was deprioritized in favor of Project Venus".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Neptune is the upcoming ML-powered search feature using Henry's recommendation engine".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Project Mercury's latency dropped 40% after Grace Park migrated it to the new Kubernetes cluster".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Eve Johnson is planning the marketing launch for Project Neptune".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "The legacy billing system is being replaced as part of Project Mercury".into(), context: "project".into(), domain: "projects".into() },
        Fact { text: "Frank Lee's frontend team is building the UI for Project Neptune".into(), context: "project".into(), domain: "projects".into() },

        // === Processes (15 facts) ===
        Fact { text: "Engineering runs two-week sprints with Monday planning and Friday demos".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "All production changes require two code reviewers and a passing CI pipeline".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "The on-call rotation cycles weekly across 6 engineers".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Quarterly OKR reviews are led by Carol Wu and include all team leads".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Security reviews are required before any new API endpoint is deployed".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "The incident response process has a 15-minute SLA for P0 incidents".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Feature flags are managed through LaunchDarkly and require product sign-off".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "The hiring pipeline averages 3 weeks from first screen to offer".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Technical RFCs must be approved by Alice Chen for cross-team changes".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Monthly tech talks are organized by the engineering team leads".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Data access requires approval from both the data team lead and security".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "The release train runs every Thursday with a 2-hour deployment window".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Post-incident reviews are mandatory within 48 hours of resolution".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "Product requirements documents are written before sprint planning".into(), context: "process".into(), domain: "processes".into() },
        Fact { text: "API versioning follows semantic versioning with a 6-month deprecation policy".into(), context: "process".into(), domain: "processes".into() },

        // === Tools & Infrastructure (15 facts) ===
        Fact { text: "The primary database is PostgreSQL 16 with read replicas".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Redis is used for caching and rate limiting across all services".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Monitoring uses Grafana dashboards with PagerDuty alerting".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "CI/CD runs on GitHub Actions with self-hosted runners".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "The event streaming platform uses Kafka with 3 brokers".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Infrastructure is managed with Terraform and deployed on AWS us-east-1".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Secrets are stored in AWS Secrets Manager and rotated quarterly".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "The search infrastructure uses Elasticsearch for full-text search".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Feature flags are managed through LaunchDarkly".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Error tracking uses Sentry with automatic issue grouping".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "The internal wiki runs on Confluence and is updated weekly".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Log aggregation uses the ELK stack with 30-day retention".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "The staging environment mirrors production but uses smaller instance sizes".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "Database migrations run automatically on deploy via Flyway".into(), context: "tool".into(), domain: "tools".into() },
        Fact { text: "The CDN is CloudFront with edge locations in 12 regions".into(), context: "tool".into(), domain: "tools".into() },

        // === Incidents (15 facts) ===
        Fact { text: "The March outage was caused by a Redis memory leak that took down the caching layer".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The June data breach exposed 10K user email addresses due to an API misconfiguration".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The September Kafka partition rebalance caused 3 hours of event processing delays".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The March outage was resolved by Grace Park who restarted the Redis cluster".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The June breach triggered Project Mars and the SOC2 compliance effort".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The September Kafka incident led to the decision to add a second event streaming cluster".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "Post-incident review for the March outage identified missing Redis monitoring alerts".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "Dave Kim led the breach investigation and recommended mTLS between all services".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The November deployment failure was caused by an incompatible database migration".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "Alice Chen mandated post-incident reviews within 48 hours after the March outage".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The API rate limiting was added after a DDoS attempt in April".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The staging environment was rebuilt after it drifted from production in August".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "Customer-facing error pages were redesigned after the March outage feedback".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "The Kafka incident post-mortem recommended consumer group monitoring".into(), context: "incident".into(), domain: "incidents".into() },
        Fact { text: "Response time for the June breach was 4 hours, exceeding the 15-minute P0 SLA".into(), context: "incident".into(), domain: "incidents".into() },

        // === Decisions (15 facts) ===
        Fact { text: "The decision to use Rust for Project Mercury was made by Alice Chen".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The team chose React Server Components over Next.js Pages Router for Project Venus".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "SOC2 Type II was chosen over Type I based on enterprise customer requirements".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The mobile app was deprioritized because 85% of users access via desktop".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "Kubernetes was chosen over ECS for container orchestration due to multi-cloud strategy".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The engineering team decided to freeze hiring until Q4 to focus on Project Mercury delivery".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "PostgreSQL was kept over switching to DynamoDB based on query complexity needs".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The monorepo migration was approved after the November deployment failure".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "Carol Wu decided to launch Project Venus to enterprise customers first".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The decision to add a second Kafka cluster was driven by the September incident".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "Alice Chen approved the budget for Project Neptune based on Henry's prototype results".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The team switched from Jenkins to GitHub Actions for CI/CD in Q1".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "Eve Johnson proposed and got approval for the developer conference budget".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "The data retention policy was changed from 90 days to 30 days after the breach".into(), context: "decision".into(), domain: "decisions".into() },
        Fact { text: "Frank Lee's proposal to rewrite the frontend was approved after the 60% performance improvement demo".into(), context: "decision".into(), domain: "decisions".into() },
    ]
}

/// Build test cases — direct recall, cross-reference, and negative.
///
/// Cross-reference questions deliberately avoid mentioning domain names.
/// They ask about RELATIONSHIPS between facts that span contexts.
fn build_test_cases() -> Vec<TestCase> {
    vec![
        // === Direct Recall (30 questions) ===
        TestCase { question: "Who is the VP of Engineering?".into(), expected_facts: vec!["Alice Chen".into()], category: Category::DirectRecall },
        TestCase { question: "What does Bob Martinez do?".into(), expected_facts: vec!["data platform".into()], category: Category::DirectRecall },
        TestCase { question: "What is Project Mercury?".into(), expected_facts: vec!["analytics platform".into(), "Rust".into()], category: Category::DirectRecall },
        TestCase { question: "What happened in the March outage?".into(), expected_facts: vec!["Redis".into(), "memory leak".into()], category: Category::DirectRecall },
        TestCase { question: "What database does the company use?".into(), expected_facts: vec!["PostgreSQL".into()], category: Category::DirectRecall },
        TestCase { question: "Who is the security architect?".into(), expected_facts: vec!["Dave Kim".into()], category: Category::DirectRecall },
        TestCase { question: "What is the release schedule?".into(), expected_facts: vec!["Thursday".into()], category: Category::DirectRecall },
        TestCase { question: "What monitoring tools are used?".into(), expected_facts: vec!["Grafana".into()], category: Category::DirectRecall },
        TestCase { question: "What is the incident response SLA?".into(), expected_facts: vec!["15-minute".into()], category: Category::DirectRecall },
        TestCase { question: "Who leads the frontend team?".into(), expected_facts: vec!["Frank Lee".into()], category: Category::DirectRecall },
        TestCase { question: "What is Project Venus?".into(), expected_facts: vec!["dashboard".into()], category: Category::DirectRecall },
        TestCase { question: "Where is the infrastructure hosted?".into(), expected_facts: vec!["AWS".into()], category: Category::DirectRecall },
        TestCase { question: "What event streaming platform is used?".into(), expected_facts: vec!["Kafka".into()], category: Category::DirectRecall },
        TestCase { question: "Who manages DevOps?".into(), expected_facts: vec!["Grace Park".into()], category: Category::DirectRecall },
        TestCase { question: "What is the sprint cadence?".into(), expected_facts: vec!["two-week".into()], category: Category::DirectRecall },
        TestCase { question: "What happened in the June breach?".into(), expected_facts: vec!["email addresses".into(), "API misconfiguration".into()], category: Category::DirectRecall },
        TestCase { question: "What is Project Mars?".into(), expected_facts: vec!["SOC2".into(), "compliance".into()], category: Category::DirectRecall },
        TestCase { question: "Who organized the developer conference?".into(), expected_facts: vec!["Eve Johnson".into()], category: Category::DirectRecall },
        TestCase { question: "What CI/CD system is used?".into(), expected_facts: vec!["GitHub Actions".into()], category: Category::DirectRecall },
        TestCase { question: "What is the code review requirement?".into(), expected_facts: vec!["two".into(), "reviewer".into()], category: Category::DirectRecall },
        TestCase { question: "What CDN is used?".into(), expected_facts: vec!["CloudFront".into()], category: Category::DirectRecall },
        TestCase { question: "Who built the recommendation engine?".into(), expected_facts: vec!["Henry Zhao".into()], category: Category::DirectRecall },
        TestCase { question: "What is the data retention policy?".into(), expected_facts: vec!["30 days".into()], category: Category::DirectRecall },
        TestCase { question: "How long does the hiring pipeline take?".into(), expected_facts: vec!["3 weeks".into()], category: Category::DirectRecall },
        TestCase { question: "What container orchestration is used?".into(), expected_facts: vec!["Kubernetes".into()], category: Category::DirectRecall },
        TestCase { question: "What caused the September incident?".into(), expected_facts: vec!["Kafka".into(), "partition".into()], category: Category::DirectRecall },
        TestCase { question: "What secret management is used?".into(), expected_facts: vec!["Secrets Manager".into()], category: Category::DirectRecall },
        TestCase { question: "What is Project Neptune?".into(), expected_facts: vec!["ML".into(), "search".into()], category: Category::DirectRecall },
        TestCase { question: "What error tracking tool is used?".into(), expected_facts: vec!["Sentry".into()], category: Category::DirectRecall },
        TestCase { question: "Where was Carol Wu before joining?".into(), expected_facts: vec!["Stripe".into()], category: Category::DirectRecall },

        // === Cross-Reference (15 questions) ===
        // These require connecting facts from DIFFERENT contexts without naming them
        TestCase {
            question: "What infrastructure changes resulted from security incidents?".into(),
            expected_facts: vec!["mTLS".into(), "Redis".into(), "Kafka".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How did the data breach affect the company's compliance strategy?".into(),
            expected_facts: vec!["SOC2".into(), "Project Mars".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "Which technical leaders are responsible for the analytics stack?".into(),
            expected_facts: vec!["Alice Chen".into(), "Bob".into(), "Mercury".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What process improvements came out of outages?".into(),
            expected_facts: vec!["post-incident".into(), "48 hours".into(), "monitoring".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How are product and engineering priorities aligned?".into(),
            expected_facts: vec!["Carol Wu".into(), "Alice Chen".into(), "weekly".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What role does the security team play in the deployment process?".into(),
            expected_facts: vec!["security review".into(), "API endpoint".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How did performance issues lead to technology decisions?".into(),
            expected_facts: vec!["Rust".into(), "Mercury".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What is the connection between the user research and the frontend rewrite?".into(),
            expected_facts: vec!["Carol Wu".into(), "Venus".into(), "user interview".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How does the ML team contribute to upcoming product launches?".into(),
            expected_facts: vec!["Henry".into(), "Neptune".into(), "recommendation".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What decisions were driven by customer feedback or user data?".into(),
            expected_facts: vec!["desktop".into(), "enterprise".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How do incidents affect the team's investment priorities?".into(),
            expected_facts: vec!["Kafka".into(), "second".into(), "cluster".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What is the relationship between the penetration test and the compliance project?".into(),
            expected_facts: vec!["Dave Kim".into(), "vulnerabilities".into(), "Mars".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How has the deployment process evolved over time?".into(),
            expected_facts: vec!["Grace Park".into(), "15 minutes".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "What marketing activities support the product roadmap?".into(),
            expected_facts: vec!["Eve Johnson".into(), "Neptune".into()],
            category: Category::CrossReference,
        },
        TestCase {
            question: "How did the November failure change engineering practices?".into(),
            expected_facts: vec!["monorepo".into(), "migration".into(), "deployment".into()],
            category: Category::CrossReference,
        },

        // === Negative (5 questions) ===
        TestCase { question: "What is the company's stock price?".into(), expected_facts: vec!["stock".into()], category: Category::Negative },
        TestCase { question: "What is the weather forecast for next week?".into(), expected_facts: vec!["weather".into()], category: Category::Negative },
        TestCase { question: "What is the recipe for chocolate cake?".into(), expected_facts: vec!["chocolate".into(), "cake".into()], category: Category::Negative },
        TestCase { question: "Who won the 2024 World Cup?".into(), expected_facts: vec!["World Cup".into()], category: Category::Negative },
        TestCase { question: "What is the capital of Australia?".into(), expected_facts: vec!["Canberra".into()], category: Category::Negative },
    ]
}

/// Create edges between facts that share entities (people, projects, tools).
///
/// Uses actual entity IDs from `add_node()` (not reconstructed from indices).
async fn create_relationship_edges(
    prime: &Prime,
    facts: &[Fact],
    entity_ids: &[String],
) -> Result<()> {
    // Shared entities to detect connections
    let entities = [
        "Alice Chen", "Bob Martinez", "Carol Wu", "Dave Kim", "Eve Johnson",
        "Frank Lee", "Grace Park", "Henry Zhao", "Mercury", "Venus", "Mars",
        "Neptune", "Redis", "Kafka", "PostgreSQL", "Kubernetes",
        "SOC2", "breach", "outage", "OKR", "sprint",
    ];

    // Build entity → fact_index mapping
    let mut entity_facts: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();
    for (idx, fact) in facts.iter().enumerate() {
        for entity in &entities {
            if fact.text.to_lowercase().contains(&entity.to_lowercase()) {
                entity_facts.entry(entity).or_default().push(idx);
            }
        }
    }

    // Create edges between facts that share an entity, using real entity IDs
    let mut edge_count = 0;
    let mut failed = 0;
    for (_entity, indices) in &entity_facts {
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let a_id = &entity_ids[indices[i]];
                let b_id = &entity_ids[indices[j]];
                match prime.add_edge(a_id, b_id, "related", None).await {
                    Ok(_) => edge_count += 1,
                    Err(_) => failed += 1,
                }
            }
        }
    }

    tracing::info!(
        "Created {} relationship edges ({} failed)",
        edge_count,
        failed
    );
    Ok(())
}
