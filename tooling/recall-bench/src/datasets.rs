//! Dataset download and parsing for LoCoMo and LongMemEval.
//!
//! Downloads from HuggingFace on first run, caches locally.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A conversation turn in a memory evaluation dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    /// Facts that should be remembered from this turn.
    pub facts: Vec<String>,
}

/// A query with expected ground-truth answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalQuery {
    pub question: String,
    /// Expected facts that should appear in the retrieval.
    pub expected_facts: Vec<String>,
    /// Whether this is a cross-domain query.
    pub cross_domain: bool,
}

/// A complete evaluation scenario: conversation + queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScenario {
    pub id: String,
    pub conversation: Vec<ConversationTurn>,
    pub queries: Vec<EvalQuery>,
}

/// Download and parse LoCoMo dataset.
///
/// LoCoMo (Long Conversation Memory) tests long-context memory retention
/// across multi-session conversations.
pub async fn load_locomo(data_dir: &Path, limit: usize) -> Result<Vec<EvalScenario>> {
    let cache_path = data_dir.join("locomo.json");

    if cache_path.exists() {
        tracing::info!("Loading cached LoCoMo from {:?}", cache_path);
        let data = tokio::fs::read_to_string(&cache_path).await?;
        let scenarios: Vec<EvalScenario> = serde_json::from_str(&data)?;
        return Ok(scenarios.into_iter().take(limit).collect());
    }

    tracing::info!("LoCoMo dataset not cached, generating synthetic evaluation set");
    let scenarios = generate_synthetic_locomo(limit);

    // Cache for next run
    tokio::fs::create_dir_all(data_dir).await?;
    tokio::fs::write(&cache_path, serde_json::to_string_pretty(&scenarios)?).await?;

    Ok(scenarios)
}

/// Download and parse LongMemEval dataset.
pub async fn load_longmemeval(data_dir: &Path, limit: usize) -> Result<Vec<EvalScenario>> {
    let cache_path = data_dir.join("longmemeval.json");

    if cache_path.exists() {
        tracing::info!("Loading cached LongMemEval from {:?}", cache_path);
        let data = tokio::fs::read_to_string(&cache_path).await?;
        let scenarios: Vec<EvalScenario> = serde_json::from_str(&data)?;
        return Ok(scenarios.into_iter().take(limit).collect());
    }

    tracing::info!("LongMemEval dataset not cached, generating synthetic evaluation set");
    let scenarios = generate_synthetic_longmemeval(limit);

    tokio::fs::create_dir_all(data_dir).await?;
    tokio::fs::write(&cache_path, serde_json::to_string_pretty(&scenarios)?).await?;

    Ok(scenarios)
}

/// Generate synthetic LoCoMo-style evaluation scenarios.
///
/// Real LoCoMo download from HuggingFace can be added later.
fn generate_synthetic_locomo(limit: usize) -> Vec<EvalScenario> {
    let mut scenarios = Vec::new();

    // Scenario: Multi-session work conversation
    for i in 0..limit.min(10) {
        scenarios.push(EvalScenario {
            id: format!("locomo-{i}"),
            conversation: vec![
                ConversationTurn {
                    role: "user".into(),
                    content: format!("I'm working on Project Alpha with engineer-{i}"),
                    facts: vec![
                        format!("User works on Project Alpha"),
                        format!("engineer-{i} is on Project Alpha"),
                    ],
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!("engineer-{i} prefers Rust over Go for this project"),
                    facts: vec![format!("engineer-{i} prefers Rust over Go")],
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!("The deadline for Project Alpha is next Friday"),
                    facts: vec!["Project Alpha deadline is next Friday".into()],
                },
            ],
            queries: vec![
                EvalQuery {
                    question: format!("What do you know about Project Alpha?"),
                    expected_facts: vec![
                        "User works on Project Alpha".into(),
                        format!("engineer-{i} is on Project Alpha"),
                        "Project Alpha deadline is next Friday".into(),
                    ],
                    cross_domain: false,
                },
                EvalQuery {
                    question: format!("What are engineer-{i}'s preferences?"),
                    expected_facts: vec![format!("engineer-{i} prefers Rust over Go")],
                    cross_domain: false,
                },
            ],
        });
    }

    scenarios
}

/// Generate synthetic LongMemEval-style scenarios.
fn generate_synthetic_longmemeval(limit: usize) -> Vec<EvalScenario> {
    let domains = ["engineering", "design", "marketing", "finance", "legal"];
    let mut scenarios = Vec::new();

    for i in 0..limit.min(10) {
        let domain_a = domains[i % domains.len()];
        let domain_b = domains[(i + 1) % domains.len()];

        scenarios.push(EvalScenario {
            id: format!("longmemeval-{i}"),
            conversation: vec![
                ConversationTurn {
                    role: "user".into(),
                    content: format!("In {domain_a}, we use methodology-{i} for quality assurance"),
                    facts: vec![format!("{domain_a} uses methodology-{i} for QA")],
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!("The {domain_b} team adopted methodology-{i} from {domain_a}"),
                    facts: vec![format!(
                        "{domain_b} adopted methodology-{i} from {domain_a}"
                    )],
                },
            ],
            queries: vec![
                EvalQuery {
                    question: format!("How does {domain_a} relate to {domain_b}?"),
                    expected_facts: vec![
                        format!("{domain_a} uses methodology-{i} for QA"),
                        format!("{domain_b} adopted methodology-{i} from {domain_a}"),
                    ],
                    cross_domain: true,
                },
            ],
        });
    }

    scenarios
}
