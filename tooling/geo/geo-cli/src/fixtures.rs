//! Recorded engine and judge replies, for `geo probe --dry-run`.
//!
//! These are **compiled into the binary** (`include_str!`), for the same
//! reason the probe set is: a dry run must produce identical output on a
//! developer's laptop and in CI, from any working directory, with no API keys
//! and no network. That is what makes the CI run evidence rather than
//! decoration.
//!
//! ## What the fixtures do and do not stand in for
//!
//! The fixture bank replaces exactly one thing — the HTTPS call. Scoring,
//! rank detection, competitor extraction, the Wilson intervals, the judge's
//! reply parsing, the verdict vocabulary, event construction and the
//! idempotency keys are all the same code a live sweep runs.
//!
//! The reply *texts* are synthetic: hand-written to exercise the paths that
//! matter (named first, named late, not named at all, a hedge, a wrong price,
//! a wrong licence, an unparseable judge reply). **Nothing derived from them
//! is a measurement**, and nothing derived from them may be written into
//! `docs/marketing/geo-baseline-*.md`. The dry run prints a banner saying so.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use geo_core::{Engine, Family, ProbeAnswer, ProbeOutcome};
use serde::Deserialize;

/// One recorded engine reply.
#[derive(Debug, Clone, Deserialize)]
pub struct FixtureReply {
    /// The response text.
    pub text: String,
    /// Citations the vendor would have returned.
    #[serde(default)]
    pub cited_urls: Vec<String>,
}

/// One engine's recorded replies.
#[derive(Debug, Clone, Deserialize)]
struct EngineFixtures {
    /// Used for any SOV prompt without its own entry.
    default_sov: FixtureReply,
    /// Used for any interrogation prompt without its own entry.
    default_interrogation: FixtureReply,
    /// Per-prompt overrides, keyed by prompt id.
    #[serde(default)]
    by_prompt: BTreeMap<String, FixtureReply>,
}

/// Recorded judge replies, as raw model output so the real parser runs.
#[derive(Debug, Clone, Deserialize)]
struct JudgeFixtures {
    /// Used for any claim without its own entry.
    default: String,
    /// Keyed `"<engine>/<prompt_id>/<claim_id>"`, then `"<prompt_id>/<claim_id>"`.
    #[serde(default)]
    by_claim: BTreeMap<String, String>,
}

/// Every recorded reply, engine and judge.
#[derive(Debug, Clone)]
pub struct FixtureBank {
    engines: BTreeMap<&'static str, EngineFixtures>,
    judge: JudgeFixtures,
}

/// The committed fixture files, compiled in.
const FILES: &[(&str, &str)] = &[
    ("chatgpt", include_str!("../tests/fixtures/probe/chatgpt.json")),
    ("claude", include_str!("../tests/fixtures/probe/claude.json")),
    (
        "perplexity",
        include_str!("../tests/fixtures/probe/perplexity.json"),
    ),
    ("gemini", include_str!("../tests/fixtures/probe/gemini.json")),
];

const JUDGE_FILE: &str = include_str!("../tests/fixtures/probe/judge.json");

impl FixtureBank {
    /// Load the compiled-in bank.
    pub fn load() -> Result<Self> {
        let mut engines = BTreeMap::new();
        for (name, body) in FILES {
            let parsed: EngineFixtures = serde_json::from_str(body)
                .with_context(|| format!("fixture file for {name} is not valid JSON"))?;
            engines.insert(*name, parsed);
        }
        let judge: JudgeFixtures =
            serde_json::from_str(JUDGE_FILE).context("judge fixture file is not valid JSON")?;

        for engine in Engine::ALL {
            if !engines.contains_key(engine.as_str()) {
                bail!(
                    "no fixture file for {engine}: --dry-run would silently skip that engine, \
                     which is exactly the failure mode the harness refuses elsewhere"
                );
            }
        }
        Ok(Self { engines, judge })
    }

    /// The recorded reply for one probe.
    pub fn answer(&self, engine: Engine, family: Family, prompt_id: &str) -> ProbeOutcome {
        let Some(bank) = self.engines.get(engine.as_str()) else {
            return ProbeOutcome::Failed {
                reason: format!("no fixtures for {engine}"),
                attempts: 0,
            };
        };
        let reply = bank.by_prompt.get(prompt_id).unwrap_or(match family {
            Family::Sov => &bank.default_sov,
            Family::Interrogation => &bank.default_interrogation,
        });
        ProbeOutcome::Answered(Box::new(ProbeAnswer {
            text: reply.text.clone(),
            cited_urls: reply.cited_urls.clone(),
            model: format!("fixture:{engine}"),
            input_tokens: None,
            output_tokens: None,
        }))
    }

    /// The recorded judge reply for one claim, as raw text.
    pub fn judge_reply(&self, engine: Engine, prompt_id: &str, claim_id: &str) -> String {
        let scoped = format!("{engine}/{prompt_id}/{claim_id}");
        let generic = format!("{prompt_id}/{claim_id}");
        self.judge
            .by_claim
            .get(&scoped)
            .or_else(|| self.judge.by_claim.get(&generic))
            .unwrap_or(&self.judge.default)
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_core::judge::{Verdict, parse_verdict};

    #[test]
    fn the_committed_bank_loads_and_covers_every_engine() {
        let bank = FixtureBank::load().expect("fixtures load");
        for engine in Engine::ALL {
            let outcome = bank.answer(engine, Family::Sov, "no-such-prompt");
            let answer = outcome.answer().expect("every engine has a default");
            assert!(!answer.text.trim().is_empty(), "{engine} default is empty");
        }
    }

    #[test]
    fn a_per_prompt_override_beats_the_default() {
        let bank = FixtureBank::load().expect("fixtures load");
        let default = bank
            .answer(Engine::Chatgpt, Family::Sov, "no-such-prompt")
            .answer()
            .expect("default")
            .text
            .clone();
        let override_text = bank
            .answer(Engine::Chatgpt, Family::Sov, "cat-agent-long-term-memory")
            .answer()
            .expect("override")
            .text
            .clone();
        assert_ne!(default, override_text);
    }

    #[test]
    fn sov_and_interrogation_get_different_defaults() {
        let bank = FixtureBank::load().expect("fixtures load");
        let sov = bank.answer(Engine::Claude, Family::Sov, "x");
        let interrogation = bank.answer(Engine::Claude, Family::Interrogation, "x");
        assert_ne!(
            sov.answer().unwrap().text,
            interrogation.answer().unwrap().text
        );
    }

    #[test]
    fn every_judge_fixture_parses_into_the_real_verdict_vocabulary() {
        // The point of recording raw judge *text* rather than a verdict enum:
        // the dry run exercises the real parser. A fixture that only parsed
        // because it was already a Verdict would prove nothing.
        let bank = FixtureBank::load().expect("fixtures load");
        let mut seen: Vec<Verdict> = Vec::new();
        for (key, reply) in &bank.judge.by_claim {
            let judged = parse_verdict(reply);
            assert!(
                !judged.reasoning.is_empty(),
                "{key} produced a verdict with no reasoning"
            );
            seen.push(judged.verdict);
        }
        // The bank has to exercise a wrong answer and an unreadable reply, or
        // the dry run never demonstrates the two paths that matter most.
        assert!(
            seen.contains(&Verdict::Inaccurate),
            "no fixture produces an `inaccurate` verdict"
        );
        assert!(
            seen.contains(&Verdict::Unscored),
            "no fixture exercises an unreadable judge reply"
        );
        assert!(seen.contains(&Verdict::Accurate));
    }

    #[test]
    fn the_judge_falls_back_to_a_default_for_unknown_claims() {
        let bank = FixtureBank::load().expect("fixtures load");
        let judged = parse_verdict(&bank.judge_reply(Engine::Gemini, "nope", "nope"));
        assert_eq!(judged.verdict, Verdict::Absent);
    }
}
