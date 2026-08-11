//! The discovery-source vocabulary for layer 4 (self-report).
//!
//! This is the **canonical** list. Three producers/consumers speak it:
//!
//! | side | file |
//! |---|---|
//! | web signup form + its route handler | `apps/web/src/lib/geo-discovery-sources.ts` |
//! | `POST /api/v1/onboard/start` validation | `apps/control-plane/geo_selfreport.go` |
//! | `geo report` layer-4 section | this crate |
//!
//! Three copies in three languages is a drift risk, and drift here is
//! expensive in a specific way: if the web form writes `"ChatGPT"` and the API
//! path writes `"chatgpt"`, the report shows two channels where there is one,
//! and the AI-sourced share — the single headline number of the layer — is
//! silently halved. So the list is serialised to
//! `docs/contracts/geo-events/discovery-sources.json` by a test in this crate,
//! and the other two sides assert against that committed file.
//!
//! **`id` is the stored value and is never renamed.** A rename splits a
//! historical series in two with no way to stitch it back together. Add
//! entries; do not re-letter them.

use serde::{Deserialize, Serialize};

/// One answer to "how did you find us?".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySource {
    /// Stored in `geo.selfreport.captured.surface`. Stable forever.
    pub id: &'static str,
    /// Human label, shown in the form and in the report.
    pub label: &'static str,
    /// Whether this counts toward the **AI-sourced share** — the number layer
    /// 4 exists to produce.
    ///
    /// `other-ai` is `true` on purpose: an assistant we have not named is
    /// still an assistant, and excluding it would bias the correction that
    /// this whole layer is a correction *for*.
    pub ai: bool,
}

impl DiscoverySource {
    /// Whether the form should offer the free-text "what did you ask it?"
    /// field for this option.
    ///
    /// Only the AI options: asking someone who came from Hacker News what
    /// they "asked it" is nonsense, and a nonsense question costs completion
    /// rate on the one question we get to ask.
    pub fn prompts_for_verbatim(&self) -> bool {
        self.ai
    }
}

/// The vocabulary. Order is the order the form renders — AI options first,
/// because they are the ones the layer is measuring and burying them under
/// "Google" costs answers.
pub const DISCOVERY_SOURCES: &[DiscoverySource] = &[
    DiscoverySource { id: "chatgpt", label: "ChatGPT", ai: true },
    DiscoverySource { id: "claude", label: "Claude", ai: true },
    DiscoverySource { id: "perplexity", label: "Perplexity", ai: true },
    DiscoverySource { id: "gemini", label: "Gemini", ai: true },
    DiscoverySource { id: "copilot", label: "Microsoft Copilot", ai: true },
    DiscoverySource { id: "other-ai", label: "Another AI assistant", ai: true },
    DiscoverySource { id: "search", label: "Google or another search engine", ai: false },
    DiscoverySource { id: "x-twitter", label: "X / Twitter", ai: false },
    DiscoverySource { id: "hn-reddit", label: "Hacker News or Reddit", ai: false },
    DiscoverySource { id: "github", label: "GitHub", ai: false },
    DiscoverySource { id: "word-of-mouth", label: "Someone told me", ai: false },
    DiscoverySource { id: "other", label: "Something else", ai: false },
];

/// Look an id up in the vocabulary.
pub fn discovery_source(id: &str) -> Option<&'static DiscoverySource> {
    DISCOVERY_SOURCES.iter().find(|s| s.id == id)
}

/// Whether a stored `surface` value counts as AI-sourced.
///
/// An id this build does not know is **not** counted as AI. A newer producer
/// adding a source must not retroactively inflate the AI share of an old
/// window read by an old binary; the report names the unknown ids instead.
pub fn is_ai_source(id: &str) -> bool {
    discovery_source(id).is_some_and(|s| s.ai)
}

/// The label for an id, falling back to the id itself so an unknown source is
/// still readable in a report rather than rendered as `(unknown)`.
pub fn discovery_label(id: &str) -> &str {
    discovery_source(id).map_or(id, |s| s.label)
}

// ───────────────────────────────────────────────────────────────────────────
// Where the capture happened
// ───────────────────────────────────────────────────────────────────────────

/// Stored in `geo.selfreport.captured.source`: which signup path collected the
/// answer.
///
/// Kept separate from `surface` (what the human said sent them) because they
/// answer different questions, and because the API path is the one that
/// captures exactly the AI-native users this programme is about — a blended
/// number would hide whether that path is capturing anything at all.
pub mod capture_path {
    /// The web signup/onboarding form.
    pub const WEB: &str = "signup-form";
    /// `POST /api/v1/onboard/start` — an agent or a human with `curl`.
    pub const API: &str = "onboard-api";

    /// Both, in report order.
    pub const ALL: &[&str] = &[WEB, API];

    /// Human label for a stored capture path.
    pub fn label(path: &str) -> &str {
        match path {
            WEB => "web signup form",
            API => "POST /api/v1/onboard/start",
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ids_are_unique_and_url_safe() {
        let mut seen = BTreeSet::new();
        for source in DISCOVERY_SOURCES {
            assert!(seen.insert(source.id), "duplicate id {}", source.id);
            assert!(
                source
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not a lowercase kebab id",
                source.id
            );
            assert!(!source.label.is_empty());
        }
    }

    #[test]
    fn every_requirement_option_is_present() {
        // The framework's option list, verbatim from the slice spec. A missing
        // option is a channel that can never be reported.
        for id in [
            "chatgpt",
            "claude",
            "perplexity",
            "gemini",
            "copilot",
            "other-ai",
            "search",
            "x-twitter",
            "hn-reddit",
            "github",
            "word-of-mouth",
            "other",
        ] {
            assert!(discovery_source(id).is_some(), "missing option {id}");
        }
    }

    #[test]
    fn unknown_sources_are_never_counted_as_ai() {
        assert!(!is_ai_source("chatgpt-5-turbo-max"));
        assert!(!is_ai_source(""));
        assert!(is_ai_source("chatgpt"));
        assert!(is_ai_source("other-ai"));
        assert!(!is_ai_source("github"));
    }

    #[test]
    fn only_ai_options_ask_what_you_asked_it() {
        for source in DISCOVERY_SOURCES {
            assert_eq!(source.prompts_for_verbatim(), source.ai, "{}", source.id);
        }
    }
}
