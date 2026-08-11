//! Typed `geo.*` payloads.
//!
//! These mirror `docs/contracts/geo-events/README.md` exactly. The contract is
//! the authority; these types are derived from it. A test in
//! `tests/contract_examples.rs` asserts the serialised shape of every type
//! against the committed example files, so the doc cannot drift from the code.

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error::GeoError, idempotency::derive_key};

/// Payload schema version, stamped on every `geo.*` payload.
///
/// Later slices add fields additively at version 1. Bump only when an existing
/// field changes meaning or type, so replays over a mixed-version stream can
/// branch on it.
pub const SCHEMA_VERSION: u32 = 1;

/// Every GEO entity id starts with this, so GEO telemetry is trivially
/// separable from — and can never be confused with — customer entity streams.
pub const ENTITY_PREFIX: &str = "geo";

/// The `geo.*` event family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GeoEventType {
    /// A human session arriving from an AI surface.
    ReferralObserved,
    /// A verified AI bot hit.
    CrawlObserved,
    /// One scored share-of-voice probe result.
    SovProbed,
    /// One scored brand-accuracy probe result.
    InterrogationProbed,
    /// A human telling us an AI sent them.
    SelfReportCaptured,
    /// One optimization iteration opened.
    ExperimentStarted,
    /// One optimization iteration scored.
    ExperimentScored,
}

impl GeoEventType {
    /// Every variant, in contract order.
    pub const ALL: [Self; 7] = [
        Self::ReferralObserved,
        Self::CrawlObserved,
        Self::SovProbed,
        Self::InterrogationProbed,
        Self::SelfReportCaptured,
        Self::ExperimentStarted,
        Self::ExperimentScored,
    ];

    /// The wire `event_type` string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReferralObserved => "geo.referral.observed",
            Self::CrawlObserved => "geo.crawl.observed",
            Self::SovProbed => "geo.sov.probed",
            Self::InterrogationProbed => "geo.interrogation.probed",
            Self::SelfReportCaptured => "geo.selfreport.captured",
            Self::ExperimentStarted => "geo.experiment.started",
            Self::ExperimentScored => "geo.experiment.scored",
        }
    }

    /// Parse a wire `event_type` string. Unknown types return `None` — a
    /// reader must not guess at an event family it does not know.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }

    /// The `entity_id` namespace segment for this type.
    fn namespace(self) -> &'static str {
        match self {
            Self::ReferralObserved => "referral",
            Self::CrawlObserved => "crawl",
            Self::SovProbed => "sov",
            Self::InterrogationProbed => "interrogation",
            Self::SelfReportCaptured => "selfreport",
            Self::ExperimentStarted | Self::ExperimentScored => "experiment",
        }
    }
}

impl std::fmt::Display for GeoEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The event-type prefix to query Core with to read the whole family.
pub const EVENT_TYPE_PREFIX: &str = "geo.";

/// Second-resolution UTC rendering, used only for idempotency-key inputs.
///
/// Sub-second jitter between two reads of the same log line must not split one
/// observation into two, so key inputs are truncated to whole seconds. The
/// payload keeps full precision.
fn key_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 1 — referral
// ───────────────────────────────────────────────────────────────────────────

/// `geo.referral.observed` — a human session arriving from an AI surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferralObserved {
    /// Payload schema version. Always [`SCHEMA_VERSION`] on write.
    pub schema_version: u32,
    /// When the session was observed (UTC).
    pub observed_at: DateTime<Utc>,
    /// The AI surface, as a free-form host-ish identifier (`"chatgpt.com"`).
    /// Deliberately not an enum — the surface taxonomy is a later slice's job.
    pub surface: String,
    /// Full referrer URL when the surface sent one.
    pub referrer_url: Option<String>,
    /// Path landed on, site-relative.
    pub landing_path: String,
    /// Opaque analytics session id, when one exists.
    pub session_id: Option<String>,
    /// Raw user agent of the arriving browser.
    pub user_agent: Option<String>,
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 2 — crawl
// ───────────────────────────────────────────────────────────────────────────

/// `geo.crawl.observed` — one verified AI bot hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlObserved {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the request hit the edge (UTC).
    pub observed_at: DateTime<Utc>,
    /// Normalised bot identifier (`"gptbot"`, `"claudebot"`). The bot
    /// *taxonomy* — families, owners, whether a bot trains or retrieves — is
    /// owned by the crawl slice, not by this contract.
    pub bot: String,
    /// Whether the claimed bot identity was verified (reverse DNS / IP range).
    /// The verification *method* is the crawl slice's concern; this contract
    /// only records the verdict.
    pub verified: bool,
    /// Raw `User-Agent` header.
    pub user_agent: String,
    /// Path requested, site-relative.
    pub path: String,
    /// HTTP status served.
    pub status: u16,
    /// Where the log line came from (`"vercel-log-drain"`).
    pub source: String,
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 3a — share of voice
// ───────────────────────────────────────────────────────────────────────────

/// `geo.sov.probed` — one scored share-of-voice probe result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SovProbed {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the probe answer came back (UTC).
    pub observed_at: DateTime<Utc>,
    /// Groups every probe in one sweep so a sweep can be scored as a unit.
    pub run_id: String,
    /// Engine probed (`"chatgpt"`, `"claude"`, `"perplexity"`, `"gemini"`).
    pub engine: String,
    /// Stable id of the prompt within the probe set. The probe *set* itself is
    /// owned by the probe slice.
    pub prompt_id: String,
    /// The prompt as sent, kept verbatim so a historical score stays readable
    /// after the probe set is edited.
    pub prompt_text: String,
    /// Whether AllSource was named in the answer at all.
    pub mentioned: bool,
    /// 1-based position among named products, `null` when absent.
    pub rank: Option<u32>,
    /// Other products named, in the order they appeared.
    pub competitors: Vec<String>,
    /// URLs the engine cited.
    pub cited_urls: Vec<String>,
    /// Normalised 0.0–1.0 score. The *formula* is owned by the probe slice;
    /// this contract only reserves the slot and the range.
    pub score: f64,
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 3b — interrogation
// ───────────────────────────────────────────────────────────────────────────

/// `geo.interrogation.probed` — one scored brand-accuracy probe result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterrogationProbed {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the probe answer came back (UTC).
    pub observed_at: DateTime<Utc>,
    /// Groups every probe in one sweep.
    pub run_id: String,
    /// Engine probed.
    pub engine: String,
    /// Stable id of the prompt within the probe set.
    pub prompt_id: String,
    /// The prompt as sent.
    pub prompt_text: String,
    /// Which factual claim about AllSource was under test
    /// (`"pricing"`, `"durability"`, `"license"`).
    pub claim_id: String,
    /// Free-form verdict string. The allowed vocabulary is fixed by the probe
    /// slice, not here — this contract only guarantees the field exists.
    pub verdict: String,
    /// The part of the answer the verdict was drawn from.
    pub answer_excerpt: String,
    /// URLs the engine cited.
    pub cited_urls: Vec<String>,
    /// Normalised 0.0–1.0 score.
    pub score: f64,
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 4 — self-report
// ───────────────────────────────────────────────────────────────────────────

/// `geo.selfreport.captured` — a human telling us an AI sent them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfReportCaptured {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the human told us (UTC).
    pub observed_at: DateTime<Utc>,
    /// Where the answer was collected (`"signup-form"`, `"onboarding-survey"`).
    pub source: String,
    /// What the human said sent them, verbatim-ish (`"ChatGPT"`).
    pub surface: String,
    /// The full free-text answer, when the question allowed one.
    pub verbatim: Option<String>,
    /// Opaque reference back to the person (tenant id, hashed handle). Never a
    /// raw email address — GEO telemetry is not a place to accumulate PII.
    pub contact_ref: Option<String>,
}

// ───────────────────────────────────────────────────────────────────────────
// Layer 5 — experiments
// ───────────────────────────────────────────────────────────────────────────

/// `geo.experiment.started` — one optimization iteration opened.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentStarted {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the iteration was opened (UTC).
    pub started_at: DateTime<Utc>,
    /// Stable id for the experiment; shared by every event in its lifecycle.
    pub experiment_id: String,
    /// Iteration number within the experiment, starting at 1.
    pub iteration: u32,
    /// What we think the change will do, in one sentence.
    pub hypothesis: String,
    /// Which measurement layer the hypothesis is aimed at
    /// (`"referral"`, `"crawl"`, `"sov"`, `"interrogation"`, `"selfreport"`).
    pub target_layer: String,
    /// What was actually changed — paths, surfaces, files.
    pub changes: Vec<String>,
    /// Score of the target metric before the change, when one is known.
    pub baseline_score: Option<f64>,
}

/// `geo.experiment.scored` — one optimization iteration scored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentScored {
    /// Payload schema version.
    pub schema_version: u32,
    /// When the score was computed (UTC).
    pub scored_at: DateTime<Utc>,
    /// The experiment this scores.
    pub experiment_id: String,
    /// Iteration scored.
    pub iteration: u32,
    /// Which metric was scored (`"sov.mention_rate"`).
    pub metric: String,
    /// The score itself.
    pub score: f64,
    /// Start of the measurement window (UTC, inclusive).
    pub window_start: DateTime<Utc>,
    /// End of the measurement window (UTC, exclusive).
    pub window_end: DateTime<Utc>,
    /// What the loop decided. Vocabulary is owned by the optimization slice.
    pub verdict: String,
    /// Anything a human should read before trusting the verdict.
    pub notes: Option<String>,
}

// ───────────────────────────────────────────────────────────────────────────
// The event union
// ───────────────────────────────────────────────────────────────────────────

/// One `geo.*` event, ready to be emitted.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoEvent {
    Referral(ReferralObserved),
    Crawl(CrawlObserved),
    Sov(SovProbed),
    Interrogation(InterrogationProbed),
    SelfReport(SelfReportCaptured),
    ExperimentStarted(ExperimentStarted),
    ExperimentScored(ExperimentScored),
}

impl GeoEvent {
    /// Wire `event_type`.
    pub fn event_type(&self) -> GeoEventType {
        match self {
            Self::Referral(_) => GeoEventType::ReferralObserved,
            Self::Crawl(_) => GeoEventType::CrawlObserved,
            Self::Sov(_) => GeoEventType::SovProbed,
            Self::Interrogation(_) => GeoEventType::InterrogationProbed,
            Self::SelfReport(_) => GeoEventType::SelfReportCaptured,
            Self::ExperimentStarted(_) => GeoEventType::ExperimentStarted,
            Self::ExperimentScored(_) => GeoEventType::ExperimentScored,
        }
    }

    /// The idempotency token derived from this event's natural key.
    ///
    /// Re-deriving it from a re-read of the same source data yields the same
    /// token, which is what makes a re-ingest safe.
    pub fn idempotency_key(&self) -> String {
        match self {
            Self::Referral(p) => derive_key(&[
                &key_ts(p.observed_at),
                &p.surface,
                &p.landing_path,
                p.session_id.as_deref().unwrap_or(""),
                p.referrer_url.as_deref().unwrap_or(""),
            ]),
            Self::Crawl(p) => derive_key(&[
                &key_ts(p.observed_at),
                &p.bot,
                &p.path,
                &p.status.to_string(),
                &p.source,
            ]),
            Self::Sov(p) => derive_key(&[&p.run_id, &p.engine, &p.prompt_id]),
            Self::Interrogation(p) => {
                derive_key(&[&p.run_id, &p.engine, &p.prompt_id, &p.claim_id])
            }
            Self::SelfReport(p) => derive_key(&[
                &key_ts(p.observed_at),
                &p.source,
                &p.surface,
                p.contact_ref.as_deref().unwrap_or(""),
            ]),
            Self::ExperimentStarted(p) => {
                derive_key(&[&p.experiment_id, &p.iteration.to_string(), "started"])
            }
            Self::ExperimentScored(p) => derive_key(&[
                &p.experiment_id,
                &p.iteration.to_string(),
                &p.metric,
                "scored",
            ]),
        }
    }

    /// Core `entity_id`.
    ///
    /// Observation events get one entity per observation
    /// (`geo:<namespace>:<idempotency-key>`), so a replay appends a version to
    /// the same entity instead of creating a second one. Experiments get one
    /// entity per experiment (`geo:experiment:<experiment_id>`), so
    /// `started` + `scored` share a stream and a single ordered read yields
    /// current state.
    pub fn entity_id(&self) -> String {
        let ns = self.event_type().namespace();
        match self {
            Self::ExperimentStarted(p) => {
                format!("{ENTITY_PREFIX}:{ns}:{}", p.experiment_id)
            }
            Self::ExperimentScored(p) => {
                format!("{ENTITY_PREFIX}:{ns}:{}", p.experiment_id)
            }
            _ => format!("{ENTITY_PREFIX}:{ns}:{}", self.idempotency_key()),
        }
    }

    /// The payload as JSON, exactly as it goes on the wire.
    pub fn payload(&self) -> Result<Value, GeoError> {
        let result = match self {
            Self::Referral(p) => serde_json::to_value(p),
            Self::Crawl(p) => serde_json::to_value(p),
            Self::Sov(p) => serde_json::to_value(p),
            Self::Interrogation(p) => serde_json::to_value(p),
            Self::SelfReport(p) => serde_json::to_value(p),
            Self::ExperimentStarted(p) => serde_json::to_value(p),
            Self::ExperimentScored(p) => serde_json::to_value(p),
        };
        result.map_err(|source| GeoError::Serialize {
            event_type: self.event_type().as_str(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples;

    #[test]
    fn every_type_parses_back_from_its_wire_string() {
        for t in GeoEventType::ALL {
            assert_eq!(GeoEventType::parse(t.as_str()), Some(t), "{t}");
            assert!(
                t.as_str().starts_with(EVENT_TYPE_PREFIX),
                "{t} is outside the geo.* family"
            );
        }
    }

    #[test]
    fn unknown_event_types_are_not_guessed() {
        assert_eq!(GeoEventType::parse("geo.made.up"), None);
        assert_eq!(GeoEventType::parse("email.received"), None);
    }

    #[test]
    fn every_entity_id_is_namespaced() {
        for event in samples::canonical_events() {
            let id = event.entity_id();
            assert!(
                id.starts_with(&format!("{ENTITY_PREFIX}:")),
                "{id} would pollute a customer entity namespace"
            );
        }
    }

    #[test]
    fn observation_entity_ids_are_replay_stable() {
        for event in samples::canonical_events() {
            let first = event.entity_id();
            let second = event.clone().entity_id();
            assert_eq!(
                first,
                second,
                "{} entity id is not stable",
                event.event_type()
            );
        }
    }

    #[test]
    fn experiment_lifecycle_shares_one_entity() {
        let started = samples::sample(GeoEventType::ExperimentStarted);
        let scored = samples::sample(GeoEventType::ExperimentScored);
        assert_eq!(started.entity_id(), scored.entity_id());
        assert_ne!(started.idempotency_key(), scored.idempotency_key());
    }

    #[test]
    fn every_payload_stamps_the_schema_version() {
        for event in samples::canonical_events() {
            let payload = event.payload().expect("payload serialises");
            assert_eq!(
                payload.get("schema_version").and_then(Value::as_u64),
                Some(u64::from(SCHEMA_VERSION)),
                "{} is missing schema_version",
                event.event_type()
            );
        }
    }
}
