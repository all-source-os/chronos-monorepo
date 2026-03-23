//! Dataset download and parsing for LongMemEval.
//!
//! Downloads from HuggingFace on first run, caches locally.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

// =============================================================================
// Common types used by all benchmarks
// =============================================================================

/// A conversation turn in a memory evaluation dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    /// Facts that should be remembered from this turn.
    pub facts: Vec<String>,
    /// Session ID this turn belongs to (for session-level scoring).
    #[serde(default)]
    pub session_id: String,
    /// Session date string for temporal ordering (e.g. "2023/04/10 (Mon) 17:50").
    #[serde(default)]
    pub session_date: Option<String>,
}

/// A query with expected ground-truth answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalQuery {
    pub question: String,
    /// Expected facts/answer that should appear in the retrieval.
    pub expected_facts: Vec<String>,
    /// Whether this is a cross-domain/multi-session query.
    pub cross_domain: bool,
    /// LongMemEval question type for stratified reporting.
    #[serde(default)]
    pub question_type: String,
    /// Session IDs that contain the answer (for session-level scoring).
    #[serde(default)]
    pub answer_session_ids: Vec<String>,
    /// Query date for temporal reasoning (e.g. "2023/04/10 (Mon) 23:07").
    #[serde(default)]
    pub query_date: Option<String>,
}

/// A complete evaluation scenario: conversation + queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalScenario {
    pub id: String,
    pub conversation: Vec<ConversationTurn>,
    pub queries: Vec<EvalQuery>,
}

// =============================================================================
// LongMemEval dataset
// =============================================================================

/// Raw LongMemEval record from the HuggingFace JSON.
#[derive(Debug, Deserialize)]
struct LongMemEvalRecord {
    question_id: String,
    question_type: String,
    question: String,
    answer: serde_json::Value, // Can be string or int
    question_date: Option<String>,
    haystack_dates: Vec<String>,
    haystack_sessions: Vec<Vec<LongMemEvalTurn>>,
    haystack_session_ids: Vec<String>,
    answer_session_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LongMemEvalTurn {
    role: String,
    content: String,
}

const LONGMEMEVAL_URL: &str = "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/longmemeval_oracle.json";

/// Download and parse LongMemEval dataset.
pub async fn load_longmemeval(data_dir: &Path, limit: usize) -> Result<Vec<EvalScenario>> {
    let cache_path = data_dir.join("longmemeval_oracle.json");

    if !cache_path.exists() {
        tracing::info!("Downloading LongMemEval from HuggingFace...");
        tokio::fs::create_dir_all(data_dir).await?;
        download_file(LONGMEMEVAL_URL, &cache_path).await?;
        tracing::info!("Downloaded LongMemEval to {:?}", cache_path);
    } else {
        tracing::info!("Using cached LongMemEval from {:?}", cache_path);
    }

    let data = tokio::fs::read_to_string(&cache_path).await?;
    let records: Vec<LongMemEvalRecord> = serde_json::from_str(&data)?;

    tracing::info!("Loaded {} LongMemEval records", records.len());

    let scenarios: Vec<EvalScenario> = records
        .into_iter()
        .take(limit)
        .map(convert_longmemeval_record)
        .collect();

    Ok(scenarios)
}

/// Convert a LongMemEval record to our EvalScenario format.
///
/// Key change: chunks each turn into sentences for better embedding granularity.
/// A 200-word turn becomes 5-8 sentence-level chunks, each independently embeddable
/// and matchable against the short expected answer.
fn convert_longmemeval_record(record: LongMemEvalRecord) -> EvalScenario {
    let mut conversation = Vec::new();

    for (session_idx, session) in record.haystack_sessions.iter().enumerate() {
        let session_id = record
            .haystack_session_ids
            .get(session_idx)
            .cloned()
            .unwrap_or_else(|| format!("session-{session_idx}"));

        let session_date = record
            .haystack_dates
            .get(session_idx)
            .cloned();

        for turn in session {
            // Chunk into sentences for finer-grained embedding
            let sentences = chunk_into_sentences(&turn.content);

            for sentence in &sentences {
                if sentence.len() < 10 {
                    continue; // Skip very short fragments
                }
                conversation.push(ConversationTurn {
                    role: turn.role.clone(),
                    content: sentence.clone(),
                    facts: vec![sentence.clone()],
                    session_id: session_id.clone(),
                    session_date: session_date.clone(),
                });
            }
        }
    }

    // Extract the answer as a string
    let answer = match &record.answer {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    };

    let is_multi_session = record.question_type.contains("multi-session");

    EvalScenario {
        id: record.question_id.clone(),
        conversation,
        queries: vec![EvalQuery {
            question: record.question.clone(),
            expected_facts: vec![answer],
            cross_domain: is_multi_session,
            question_type: record.question_type,
            answer_session_ids: record.answer_session_ids,
            query_date: record.question_date,
        }],
    }
}

/// Split text into sentences. Simple heuristic: split on `. `, `? `, `! `,
/// and keep sentences that are at least 10 chars.
fn chunk_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if (ch == '.' || ch == '?' || ch == '!') && current.len() > 10 {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current = String::new();
        }
    }

    // Don't lose the last fragment
    let trimmed = current.trim().to_string();
    if trimmed.len() >= 10 {
        sentences.push(trimmed);
    }

    // If no sentence boundaries found, return the full text as one chunk
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim().to_string());
    }

    sentences
}

/// Download a file from a URL to a local path.
async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    tokio::fs::write(dest, &bytes).await?;
    Ok(())
}

// =============================================================================
// Synthetic LOCOMO (fallback)
// =============================================================================

/// Generate synthetic LoCoMo-style evaluation scenarios (fallback if no download).
pub async fn load_locomo(_data_dir: &Path, limit: usize) -> Result<Vec<EvalScenario>> {
    tracing::info!("Using synthetic LoCoMo scenarios (limit={})", limit);
    Ok(generate_synthetic_locomo(limit))
}

fn generate_synthetic_locomo(limit: usize) -> Vec<EvalScenario> {
    let mut scenarios = Vec::new();

    for i in 0..limit.min(10) {
        scenarios.push(EvalScenario {
            id: format!("locomo-{i}"),
            conversation: vec![
                ConversationTurn {
                    role: "user".into(),
                    content: format!("I'm working on Project Alpha with engineer-{i}"),
                    facts: vec![
                        "User works on Project Alpha".into(),
                        format!("engineer-{i} is on Project Alpha"),
                    ],
                    session_id: "s0".into(),
                    session_date: None,
                },
                ConversationTurn {
                    role: "user".into(),
                    content: format!("engineer-{i} prefers Rust over Go for this project"),
                    facts: vec![format!("engineer-{i} prefers Rust over Go")],
                    session_id: "s0".into(),
                    session_date: None,
                },
                ConversationTurn {
                    role: "user".into(),
                    content: "The deadline for Project Alpha is next Friday".into(),
                    facts: vec!["Project Alpha deadline is next Friday".into()],
                    session_id: "s0".into(),
                    session_date: None,
                },
            ],
            queries: vec![
                EvalQuery {
                    question: "What do you know about Project Alpha?".into(),
                    expected_facts: vec![
                        "User works on Project Alpha".into(),
                        format!("engineer-{i} is on Project Alpha"),
                        "Project Alpha deadline is next Friday".into(),
                    ],
                    cross_domain: false,
                    question_type: "single-session-user".into(),
                    answer_session_ids: vec!["s0".into()],
                    query_date: None,
                },
                EvalQuery {
                    question: format!("What are engineer-{i}'s preferences?"),
                    expected_facts: vec![format!("engineer-{i} prefers Rust over Go")],
                    cross_domain: false,
                    question_type: "single-session-user".into(),
                    answer_session_ids: vec!["s0".into()],
                    query_date: None,
                },
            ],
        });
    }

    scenarios
}
