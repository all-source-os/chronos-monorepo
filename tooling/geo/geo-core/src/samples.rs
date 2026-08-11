//! The canonical example of every `geo.*` event.
//!
//! These are the single source of the JSON in
//! `docs/contracts/geo-events/examples/`. `tests/contract_examples.rs`
//! asserts that the committed files and these samples render identically, and
//! `geo report --dry-run` prints them — so the contract doc, the types and the
//! CLI's dry-run output cannot drift apart silently.

use chrono::{DateTime, Utc};

use crate::{
    bots::{BotCategory, TAXONOMY_VERSION},
    events::{
        Aggregation, CrawlObserved, ExperimentScored, ExperimentStarted, GeoEvent, GeoEventType,
        InterrogationProbed, ReferralObserved, SCHEMA_VERSION, SelfReportCaptured, SovProbed,
    },
};

/// Parse a fixed RFC 3339 timestamp. Panics only on a malformed literal in
/// this file, which a test would catch immediately.
fn ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .expect("sample timestamp is valid RFC 3339")
        .with_timezone(&Utc)
}

/// The canonical example for one event type.
pub fn sample(event_type: GeoEventType) -> GeoEvent {
    match event_type {
        GeoEventType::ReferralObserved => GeoEvent::Referral(ReferralObserved {
            schema_version: SCHEMA_VERSION,
            observed_at: ts("2026-08-11T09:15:00Z"),
            surface: "chatgpt.com".to_string(),
            referrer_url: Some("https://chatgpt.com/".to_string()),
            landing_path: "/pricing".to_string(),
            session_id: Some("sess_01J8Z9QK3M".to_string()),
            user_agent: Some(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36".to_string(),
            ),
            converted: false,
            conversion_kind: None,
        }),
        GeoEventType::CrawlObserved => GeoEvent::Crawl(CrawlObserved {
            schema_version: SCHEMA_VERSION,
            observed_at: ts("2026-08-11T09:20:31Z"),
            bot: "gptbot".to_string(),
            category: BotCategory::TrainingCrawler.as_str().to_string(),
            taxonomy_version: TAXONOMY_VERSION,
            verified: true,
            user_agent: "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); \
                         compatible; GPTBot/1.2; +https://openai.com/gptbot"
                .to_string(),
            path: "/llms.txt".to_string(),
            status: 200,
            source: "vercel-log-drain".to_string(),
            aggregation: Aggregation::Hit.as_str().to_string(),
            hits: 1,
            window_end: None,
            request_id: Some("iad1::abcde-1754904031000-0f1e2d3c4b5a".to_string()),
        }),
        GeoEventType::SovProbed => GeoEvent::Sov(SovProbed {
            schema_version: SCHEMA_VERSION,
            observed_at: ts("2026-08-11T10:00:00Z"),
            // `<family>-<date>-<prompt-set digest>#r<repetition>`: the digest
            // makes two sweeps comparable only if they used the same frozen
            // set, and the repetition keeps the N samples of one prompt as N
            // entities instead of N versions of one.
            run_id: "sov-2026-08-11-9f2c41ab#r1".to_string(),
            engine: "chatgpt".to_string(),
            prompt_id: "cat-agent-long-term-memory".to_string(),
            prompt_text: "What should I use to give my AI agent long-term memory?".to_string(),
            intent: "category".to_string(),
            mentioned: true,
            rank: Some(3),
            competitors: vec!["mem0".to_string(), "zep".to_string()],
            cited_urls: vec!["https://www.all-source.xyz/".to_string()],
            score: 0.66,
        }),
        GeoEventType::InterrogationProbed => GeoEvent::Interrogation(InterrogationProbed {
            schema_version: SCHEMA_VERSION,
            observed_at: ts("2026-08-11T10:05:00Z"),
            run_id: "interrogation-2026-08-11-4d81be07#r1".to_string(),
            engine: "claude".to_string(),
            prompt_id: "int-storage-and-durability".to_string(),
            prompt_text: "How does AllSource actually store data? Is it durable if the \
                          process restarts?"
                .to_string(),
            claim_id: "durability.wal-parquet".to_string(),
            verdict: "accurate".to_string(),
            reasoning: "Correctly describes the WAL plus Parquet persistence and the \
                        in-memory read path."
                .to_string(),
            judge_model: "claude-opus-5".to_string(),
            answer_excerpt: "AllSource is an event store that persists events to a \
                             write-ahead log and Parquet files."
                .to_string(),
            cited_urls: vec!["https://www.all-source.xyz/docs".to_string()],
            score: 1.0,
        }),
        GeoEventType::SelfReportCaptured => GeoEvent::SelfReport(SelfReportCaptured {
            schema_version: SCHEMA_VERSION,
            observed_at: ts("2026-08-11T11:30:00Z"),
            source: "signup-form".to_string(),
            surface: "ChatGPT".to_string(),
            verbatim: Some("asked ChatGPT for an event store for agent memory".to_string()),
            contact_ref: Some("tenant_01J8ZB4R7T".to_string()),
        }),
        GeoEventType::ExperimentStarted => GeoEvent::ExperimentStarted(ExperimentStarted {
            schema_version: SCHEMA_VERSION,
            started_at: ts("2026-08-12T08:00:00Z"),
            experiment_id: "exp-llms-txt-rewrite".to_string(),
            iteration: 1,
            hypothesis: "Naming the agent-memory use case in llms.txt raises \
                         mention rate on agent-memory prompts."
                .to_string(),
            target_layer: "sov".to_string(),
            changes: vec!["apps/web/public/llms.txt".to_string()],
            baseline_score: Some(0.42),
        }),
        GeoEventType::ExperimentScored => GeoEvent::ExperimentScored(ExperimentScored {
            schema_version: SCHEMA_VERSION,
            scored_at: ts("2026-08-26T08:00:00Z"),
            experiment_id: "exp-llms-txt-rewrite".to_string(),
            iteration: 1,
            metric: "sov.mention_rate".to_string(),
            score: 0.58,
            window_start: ts("2026-08-12T00:00:00Z"),
            window_end: ts("2026-08-26T00:00:00Z"),
            verdict: "promote".to_string(),
            notes: Some("Two-week window; 4 engines, 12 prompts per sweep.".to_string()),
        }),
    }
}

/// One canonical example of every event type, in contract order.
pub fn canonical_events() -> Vec<GeoEvent> {
    GeoEventType::ALL.into_iter().map(sample).collect()
}

/// The file name a type's example lives under in
/// `docs/contracts/geo-events/examples/`.
pub fn example_file_name(event_type: GeoEventType) -> String {
    format!("{}.json", event_type.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_exactly_one_sample_per_event_type() {
        let events = canonical_events();
        assert_eq!(events.len(), GeoEventType::ALL.len());
        for (event, expected) in events.iter().zip(GeoEventType::ALL) {
            assert_eq!(event.event_type(), expected);
        }
    }
}
