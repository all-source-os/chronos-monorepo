//! LLM-as-judge for layer 3b: is what the engine said about us *true*?
//!
//! ## Why a judge at all
//!
//! Accuracy cannot be string-matched. "£18.99/month" and "about nineteen
//! pounds a month" are the same answer; "$19/month" is a different one, and
//! whether that difference matters is a judgement call about currency, not a
//! diff. So a model reads the answer against a ground-truth expectation
//! extracted from this repository and returns a verdict.
//!
//! ## Why the reasoning is stored, not just the verdict
//!
//! A judge is another stochastic model. Every verdict lands in Core with the
//! reasoning and the quoted excerpt it was drawn from, so a human can overrule
//! it — and so a wrong verdict is visible as a wrong *argument* rather than as
//! an unexplained number in a trend line. A verdict with no reasoning is not
//! evidence; [`parse_verdict`] refuses to produce one.
//!
//! ## Known conflict of interest
//!
//! The judge runs on Claude, and one of the engines probed is Claude. A model
//! grading its own homework is a real bias, and this module does not pretend
//! otherwise: the engine is recorded on every event, so per-engine accuracy
//! can be read with that caveat, and the runbook says to sanity-check the
//! `claude` column by hand.

use serde_json::Value;

use crate::{
    probe::Engine,
    prompts::{Claim, Prompt},
};

/// Environment variable overriding the judge model.
pub const JUDGE_MODEL_ENV: &str = "GEO_JUDGE_MODEL";

/// Default judge model.
///
/// Accuracy grading is the most intelligence-sensitive step in the programme —
/// a judge that misreads a hedge as a claim poisons the backlog — so this is
/// deliberately not the cheap option.
pub const DEFAULT_JUDGE_MODEL: &str = "claude-opus-5";

/// The engine the judge runs on.
pub const JUDGE_ENGINE: Engine = Engine::Claude;

/// The fixed verdict vocabulary. The event contract reserves the field and
/// leaves the vocabulary to this slice; this is it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// The answer states the claim correctly.
    Accurate,
    /// Right in substance, wrong in a detail that a buyer would notice.
    PartiallyAccurate,
    /// The answer states something false about the claim.
    Inaccurate,
    /// The answer does not address the claim at all — including an explicit
    /// "I don't know". Not the same as being wrong, and kept separate because
    /// the fix is different: silence is a content gap, wrongness is a
    /// correction.
    Absent,
    /// The judge could not be run, or its reply could not be read. Never
    /// counted as an accuracy result in either direction.
    Unscored,
}

impl Verdict {
    /// Every verdict, best first.
    pub const ALL: [Self; 5] = [
        Self::Accurate,
        Self::PartiallyAccurate,
        Self::Inaccurate,
        Self::Absent,
        Self::Unscored,
    ];

    /// The `payload.verdict` string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accurate => "accurate",
            Self::PartiallyAccurate => "partially_accurate",
            Self::Inaccurate => "inaccurate",
            Self::Absent => "absent",
            Self::Unscored => "unscored",
        }
    }

    /// Parse a wire string.
    pub fn parse(s: &str) -> Option<Self> {
        let normalised = s.trim().to_lowercase().replace([' ', '-'], "_");
        Self::ALL.into_iter().find(|v| v.as_str() == normalised)
    }

    /// The `payload.score` for this verdict, 0.0–1.0.
    pub fn score(self) -> f64 {
        match self {
            Self::Accurate => 1.0,
            Self::PartiallyAccurate => 0.5,
            // `Absent` scores 0 because the buyer did not learn the true
            // thing — but it is reported in its own column, never folded into
            // "wrong".
            Self::Inaccurate | Self::Absent | Self::Unscored => 0.0,
        }
    }

    /// Whether this verdict counts toward an accuracy denominator.
    pub fn is_scored(self) -> bool {
        !matches!(self, Self::Unscored)
    }

    /// Whether this verdict belongs in the remediation backlog.
    pub fn needs_remediation(self) -> bool {
        matches!(self, Self::Inaccurate | Self::PartiallyAccurate)
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One graded claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judgement {
    /// The verdict.
    pub verdict: Verdict,
    /// The judge's own reasoning, stored so a human can overrule it.
    pub reasoning: String,
    /// The part of the answer the verdict was drawn from, quoted verbatim.
    /// Empty when the verdict is `absent`.
    pub excerpt: String,
}

impl Judgement {
    /// A judgement for a claim that could not be graded.
    pub fn unscored(reason: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Unscored,
            reasoning: reason.into(),
            excerpt: String::new(),
        }
    }
}

/// Cap on the judge's own output.
pub const JUDGE_MAX_TOKENS: u32 = 2000;

/// Build the grading prompt for one claim.
///
/// Deliberately narrow: the judge grades **one** claim against **one**
/// expectation, and is told to ignore everything else in the answer. A judge
/// asked "is this answer good?" grades tone; a judge asked "does this answer
/// say the licence is Apache-2.0?" grades the licence.
pub fn judge_prompt(prompt: &Prompt, claim: &Claim, engine: Engine, answer: &str) -> String {
    format!(
        "You are grading one factual claim in an AI assistant's answer about a software \
product. You are not grading style, helpfulness, or whether the answer is a good answer. \
Grade only the claim named below.\n\
\n\
QUESTION PUT TO {engine} (prompt id: {prompt_id}):\n\
{question}\n\
\n\
THE ASSISTANT'S FULL ANSWER:\n\
<answer>\n{answer}\n</answer>\n\
\n\
THE CLAIM UNDER TEST (id: {claim_id}):\n\
{expectation}\n\
\n\
Choose exactly one verdict:\n\
- \"accurate\"           — the answer states this claim correctly.\n\
- \"partially_accurate\" — right in substance but wrong in a detail a buyer would notice \
(wrong currency, an out-of-date number, one tier name wrong).\n\
- \"inaccurate\"         — the answer states something false about this claim.\n\
- \"absent\"             — the answer does not address this claim at all, or explicitly says \
it does not know. Absent is NOT the same as inaccurate; do not punish silence as if it were \
an error.\n\
\n\
Rules:\n\
1. Judge only against the claim text above. If the answer is right about something else, that \
is irrelevant here.\n\
2. Quote the exact sentence(s) from the answer that your verdict rests on, verbatim, in \
\"excerpt\". If the verdict is \"absent\", use an empty string.\n\
3. Explain your verdict in \"reasoning\" in one or two sentences, naming the specific \
discrepancy. A human will read this to decide whether to overrule you.\n\
4. Do not be generous. A confidently stated wrong price is \"inaccurate\", not \
\"partially_accurate\".\n\
\n\
Reply with ONLY a JSON object, no prose and no code fence:\n\
{{\"verdict\": \"...\", \"excerpt\": \"...\", \"reasoning\": \"...\"}}",
        engine = engine,
        prompt_id = prompt.id,
        question = prompt.text,
        answer = answer,
        claim_id = claim.id,
        expectation = claim.expectation,
    )
}

/// Read the judge's reply into a [`Judgement`].
///
/// Lenient about packaging (code fences, a stray sentence before the object)
/// and strict about content: an unparseable reply, an unknown verdict or a
/// missing rationale all become `Unscored` with the raw reply preserved,
/// because a verdict nobody can audit must never enter an accuracy rate.
pub fn parse_verdict(reply: &str) -> Judgement {
    let Some(json) = extract_object(reply) else {
        return Judgement::unscored(format!(
            "judge reply contained no JSON object; raw reply: {}",
            clip(reply)
        ));
    };
    let value: Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(e) => {
            return Judgement::unscored(format!(
                "judge reply was not valid JSON ({e}); raw reply: {}",
                clip(reply)
            ));
        }
    };

    let verdict_str = value.get("verdict").and_then(Value::as_str).unwrap_or("");
    let Some(verdict) = Verdict::parse(verdict_str) else {
        return Judgement::unscored(format!(
            "judge returned an unknown verdict {verdict_str:?}; raw reply: {}",
            clip(reply)
        ));
    };
    if verdict == Verdict::Unscored {
        return Judgement::unscored(
            "judge returned 'unscored', which is a harness state and not a verdict it may pick"
                .to_string(),
        );
    }

    let reasoning = value
        .get("reasoning")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if reasoning.is_empty() {
        return Judgement::unscored(format!(
            "judge returned {verdict} with no reasoning, which is not auditable; raw reply: {}",
            clip(reply)
        ));
    }
    let excerpt = value
        .get("excerpt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if excerpt.is_empty() && verdict != Verdict::Absent {
        return Judgement::unscored(format!(
            "judge returned {verdict} without quoting the answer, so the claim cannot be \
             checked; raw reply: {}",
            clip(reply)
        ));
    }

    Judgement {
        verdict,
        reasoning,
        excerpt,
    }
}

/// The outermost `{...}` in a reply, if any.
fn extract_object(reply: &str) -> Option<String> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(reply[start..=end].to_string())
}

fn clip(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() <= 400 {
        return s.to_string();
    }
    s.chars().take(400).collect::<String>() + "…"
}

/// The judge model this run will use.
pub fn judge_model() -> String {
    crate::config::init_env();
    std::env::var(JUDGE_MODEL_ENV)
        .ok()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| DEFAULT_JUDGE_MODEL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::{Family, Intent, Severity};

    fn claim() -> Claim {
        Claim {
            id: "license.apache".to_string(),
            expectation: "AllSource's community edition is Apache-2.0 licensed.".to_string(),
            source: "LICENSE".to_string(),
            severity: Severity::High,
        }
    }

    fn prompt() -> Prompt {
        Prompt {
            id: "license".to_string(),
            family: Family::Interrogation,
            intent: Intent::Brand,
            text: "What licence is AllSource released under?".to_string(),
            claims: vec![claim()],
        }
    }

    #[test]
    fn verdicts_round_trip_and_score_in_order() {
        for verdict in Verdict::ALL {
            assert_eq!(Verdict::parse(verdict.as_str()), Some(verdict));
        }
        assert!(Verdict::Accurate.score() > Verdict::PartiallyAccurate.score());
        assert!(Verdict::PartiallyAccurate.score() > Verdict::Inaccurate.score());
        for verdict in Verdict::ALL {
            assert!((0.0..=1.0).contains(&verdict.score()), "{verdict}");
        }
    }

    #[test]
    fn verdict_parsing_tolerates_the_judges_formatting() {
        assert_eq!(Verdict::parse(" Accurate "), Some(Verdict::Accurate));
        assert_eq!(
            Verdict::parse("partially-accurate"),
            Some(Verdict::PartiallyAccurate)
        );
        assert_eq!(Verdict::parse("mostly right"), None);
    }

    #[test]
    fn silence_is_not_wrongness() {
        // The distinction the whole backlog ordering rests on: `absent` is a
        // content gap, `inaccurate` is a correction.
        assert!(!Verdict::Absent.needs_remediation());
        assert!(Verdict::Inaccurate.needs_remediation());
        assert!(Verdict::PartiallyAccurate.needs_remediation());
        assert!(Verdict::Absent.is_scored());
        assert!(!Verdict::Unscored.is_scored());
    }

    #[test]
    fn the_prompt_names_the_claim_the_answer_and_the_verdicts() {
        let text = judge_prompt(&prompt(), &claim(), Engine::Chatgpt, "It is MIT licensed.");
        assert!(text.contains("Apache-2.0"), "expectation missing");
        assert!(text.contains("It is MIT licensed."), "answer missing");
        assert!(text.contains("license.apache"), "claim id missing");
        for verdict in [
            Verdict::Accurate,
            Verdict::PartiallyAccurate,
            Verdict::Inaccurate,
            Verdict::Absent,
        ] {
            assert!(text.contains(verdict.as_str()), "{verdict} not offered");
        }
        assert!(
            !text.contains("\"unscored\""),
            "the judge must not be offered the harness's own failure state"
        );
    }

    #[test]
    fn a_clean_reply_parses() {
        let judged = parse_verdict(
            r#"{"verdict": "inaccurate", "excerpt": "AllSource is MIT licensed.", "reasoning": "The answer says MIT; the community edition is Apache-2.0."}"#,
        );
        assert_eq!(judged.verdict, Verdict::Inaccurate);
        assert_eq!(judged.excerpt, "AllSource is MIT licensed.");
        assert!(judged.reasoning.contains("Apache-2.0"));
    }

    #[test]
    fn a_fenced_reply_still_parses() {
        let judged = parse_verdict(
            "Here you go:\n```json\n{\"verdict\":\"accurate\",\"excerpt\":\"Apache-2.0.\",\"reasoning\":\"Matches.\"}\n```",
        );
        assert_eq!(judged.verdict, Verdict::Accurate);
    }

    #[test]
    fn an_absent_verdict_may_quote_nothing() {
        let judged = parse_verdict(
            r#"{"verdict":"absent","excerpt":"","reasoning":"The answer never mentions a licence."}"#,
        );
        assert_eq!(judged.verdict, Verdict::Absent);
        assert!(judged.excerpt.is_empty());
    }

    #[test]
    fn a_verdict_with_no_reasoning_is_refused() {
        // A verdict nobody can audit must never enter an accuracy rate.
        let judged = parse_verdict(r#"{"verdict":"accurate","excerpt":"x","reasoning":"  "}"#);
        assert_eq!(judged.verdict, Verdict::Unscored);
        assert!(judged.reasoning.contains("not auditable"), "{judged:?}");
    }

    #[test]
    fn a_non_absent_verdict_with_no_excerpt_is_refused() {
        let judged =
            parse_verdict(r#"{"verdict":"inaccurate","excerpt":"","reasoning":"It is wrong."}"#);
        assert_eq!(judged.verdict, Verdict::Unscored);
        assert!(judged.reasoning.contains("without quoting"), "{judged:?}");
    }

    #[test]
    fn garbage_becomes_unscored_and_keeps_the_raw_reply() {
        let judged = parse_verdict("I'm sorry, I can't help with that.");
        assert_eq!(judged.verdict, Verdict::Unscored);
        assert!(judged.reasoning.contains("I'm sorry"), "{judged:?}");
    }

    #[test]
    fn the_judge_cannot_award_itself_the_harness_failure_state() {
        let judged =
            parse_verdict(r#"{"verdict":"unscored","excerpt":"x","reasoning":"dunno"}"#);
        assert_eq!(judged.verdict, Verdict::Unscored);
        assert!(judged.reasoning.contains("harness state"), "{judged:?}");
    }

    #[test]
    fn an_unknown_verdict_is_not_guessed_into_a_neighbour() {
        let judged =
            parse_verdict(r#"{"verdict":"mostly_ok","excerpt":"x","reasoning":"eh"}"#);
        assert_eq!(judged.verdict, Verdict::Unscored);
        assert!(judged.reasoning.contains("unknown verdict"), "{judged:?}");
    }

    #[test]
    fn the_judge_model_is_overridable() {
        assert_eq!(DEFAULT_JUDGE_MODEL, "claude-opus-5");
        assert!(judge_model().starts_with("claude-") || !judge_model().is_empty());
    }
}
