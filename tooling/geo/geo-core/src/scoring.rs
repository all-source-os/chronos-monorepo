//! Turning answers into numbers — and into numbers that are honest about how
//! little one answer proves.
//!
//! ## The statistical position this module takes
//!
//! LLM answers are stochastic. The same prompt to the same model twice does
//! not give the same answer, and the difference between "we are named in 3 of
//! 12 answers" and "we are named in 5 of 12" is nowhere near significant. So:
//!
//! - Every rate is reported as a **[`RateEstimate`] with a Wilson score
//!   interval**, never as a bare percentage. [`RateEstimate::render`] refuses
//!   to print a point estimate on its own.
//! - The interval is Wilson rather than the textbook normal approximation
//!   because our n is small and our p is near 0 — exactly where the normal
//!   approximation produces negative lower bounds and a zero-width interval at
//!   0/n. A 0/12 run is *not* "0% ± 0", it is "0%, and anything up to ~24% is
//!   consistent with what we saw".
//! - Nothing here computes a trend. A trend over fewer than the framework's
//!   12 weeks is noise with a slope, and the runbook says so.
//!
//! ## The SOV score itself
//!
//! Per probe: **reciprocal rank** — `1/rank` among the products named, `0` when
//! we are not named. Rank 1 scores 1.0, rank 4 scores 0.25. It is the standard
//! MRR formulation, it lands in the contract's reserved 0.0–1.0 slot, and it
//! separates "named first" from "named in a list of nine" — which a bare
//! mention rate cannot.
//!
//! **Mention rate remains the headline**, not this score. A score that folds
//! rank and presence together is convenient and unfalsifiable; the report
//! prints both, and prompt 027 optimises against the mention rate.

use std::collections::{BTreeMap, BTreeSet};

use crate::brand::{self, COMPETITORS};

/// The z value for a 95% two-sided interval.
const Z95: f64 = 1.959_964;

/// Minimum answers a term must appear in before it enters the key-term map.
/// One model saying a word once is a coincidence, not vocabulary.
pub const MIN_TERM_DOCUMENTS: usize = 2;

/// Longest n-gram extracted for the key-term map.
const MAX_NGRAM: usize = 3;

// ───────────────────────────────────────────────────────────────────────────
// Per-probe scoring
// ───────────────────────────────────────────────────────────────────────────

/// One answer, scored for share of voice.
#[derive(Debug, Clone, PartialEq)]
pub struct SovScore {
    /// Was AllSource named at all.
    pub mentioned: bool,
    /// 1-based position among every product named, `None` when absent.
    pub rank: Option<u32>,
    /// Competitor ids named, in order of first appearance.
    pub competitors: Vec<String>,
    /// Reciprocal rank, 0.0–1.0.
    pub score: f64,
    /// The hedge phrase that made this a knowledge gap, if any.
    pub hedge: Option<&'static str>,
}

/// Score one SOV answer.
pub fn score_sov(text: &str) -> SovScore {
    let named = brand::named_products(text);
    let rank = named
        .iter()
        .position(|&(id, _)| id == "allsource")
        .map(|i| i as u32 + 1);
    let competitors: Vec<String> = named
        .iter()
        .filter(|&&(id, _)| id != "allsource")
        .map(|&(id, _)| id.to_string())
        .collect();

    SovScore {
        mentioned: rank.is_some(),
        rank,
        competitors,
        score: rank.map_or(0.0, |r| 1.0 / f64::from(r)),
        hedge: hedge_phrase(text),
    }
}

/// Phrases that mean "the model does not know", not "the model disagrees".
///
/// These are content-production targets: a prompt where every engine hedges is
/// a question the internet has no answer to yet, which is the cheapest kind of
/// share to win.
const HEDGES: &[&str] = &[
    "i don't have information",
    "i do not have information",
    "i'm not familiar",
    "i am not familiar",
    "i'm not aware",
    "i am not aware",
    "i don't have any information",
    "i couldn't find",
    "i could not find",
    "no information about",
    "not something i have information",
    "knowledge cutoff",
    "training data cutoff",
    "as of my last update",
    "i'm unable to find",
    "there is no widely known",
    "doesn't appear to be a well-known",
    "does not appear to be a well-known",
    "may be referring to",
];

/// The first hedge phrase in the text, if any.
pub fn hedge_phrase(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    HEDGES.iter().copied().find(|h| lower.contains(h))
}

// ───────────────────────────────────────────────────────────────────────────
// Rates, with intervals
// ───────────────────────────────────────────────────────────────────────────

/// A proportion with a 95% Wilson score interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateEstimate {
    /// Numerator.
    pub successes: u64,
    /// Denominator.
    pub total: u64,
    /// `successes / total`, or 0.0 when `total == 0`.
    pub point: f64,
    /// Lower bound of the 95% interval.
    pub low: f64,
    /// Upper bound of the 95% interval.
    pub high: f64,
}

impl RateEstimate {
    /// Compute the estimate and its interval.
    pub fn new(successes: u64, total: u64) -> Self {
        if total == 0 {
            return Self {
                successes,
                total,
                point: 0.0,
                low: 0.0,
                high: 1.0,
            };
        }
        let n = total as f64;
        let p = successes as f64 / n;
        let z2 = Z95 * Z95;
        let denom = 1.0 + z2 / n;
        let centre = p + z2 / (2.0 * n);
        let margin = Z95 * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt();
        Self {
            successes,
            total,
            point: p,
            low: ((centre - margin) / denom).max(0.0),
            high: ((centre + margin) / denom).min(1.0),
        }
    }

    /// The only rendering. There is deliberately no "just the percentage"
    /// helper: a point estimate off 12 samples printed without its interval is
    /// the single most misleading number this programme can produce.
    pub fn render(&self) -> String {
        if self.total == 0 {
            return "— (no samples)".to_string();
        }
        format!(
            "{:>5.1}% [{:>4.1}–{:>4.1}] n={}",
            self.point * 100.0,
            self.low * 100.0,
            self.high * 100.0,
            self.total
        )
    }

    /// Whether two estimates' intervals overlap.
    ///
    /// Non-overlap is a *weak* signal of a real difference (it is more
    /// conservative than a proper two-proportion test), which is the right
    /// direction to be wrong in when a wrong call sends prompt 027 chasing
    /// noise for a fortnight.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.low <= other.high && other.low <= self.high
    }
}

/// Successes over trials, for one cell of a report table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RateTally {
    /// Trials where the thing happened.
    pub successes: u64,
    /// Trials.
    pub total: u64,
}

impl RateTally {
    /// Record one trial.
    pub fn record(&mut self, success: bool) {
        self.total += 1;
        if success {
            self.successes += 1;
        }
    }

    /// The estimate for this cell.
    pub fn estimate(&self) -> RateEstimate {
        RateEstimate::new(self.successes, self.total)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Source attribution
// ───────────────────────────────────────────────────────────────────────────

/// Whether a cited URL is a surface we control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceOwner {
    /// A page we can edit today.
    Ours,
    /// Someone else's page. When one of these carries a wrong claim it is the
    /// highest-value remediation target in the whole programme — we cannot fix
    /// it directly, so it has to be out-published or corrected at the source.
    ThirdParty,
}

/// Hosts we control.
const OUR_HOSTS: &[&str] = &[
    "all-source.xyz",
    "www.all-source.xyz",
    "api.all-source.xyz",
    "docs.all-source.xyz",
    "status.all-source.xyz",
];

/// Host-path prefixes we control on someone else's domain.
const OUR_PREFIXES: &[&str] = &[
    "github.com/all-source-os",
    "crates.io/crates/allsource",
    "crates.io/crates/allsource-core",
    "crates.io/crates/allsource-prime",
    "npmjs.com/package/@allsourcedev/client",
    "www.npmjs.com/package/@allsourcedev/client",
];

/// The host of a URL, lowercased, without a leading `www.` being stripped —
/// the report groups by exactly what was cited.
pub fn host_of(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .to_lowercase()
}

/// Classify a cited URL.
pub fn classify_source(url: &str) -> SourceOwner {
    let lower = url.to_lowercase();
    let host = host_of(&lower);
    if OUR_HOSTS.contains(&host.as_str()) {
        return SourceOwner::Ours;
    }
    let stripped = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    if OUR_PREFIXES.iter().any(|p| stripped.starts_with(p)) {
        return SourceOwner::Ours;
    }
    SourceOwner::ThirdParty
}

// ───────────────────────────────────────────────────────────────────────────
// Key-term extraction
// ───────────────────────────────────────────────────────────────────────────

/// One observed term and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermCount {
    /// The term, lowercased.
    pub term: String,
    /// How many answers contained it (document frequency, not raw count — one
    /// model repeating itself is not evidence of vocabulary).
    pub documents: usize,
    /// Which engines used it.
    pub engines: BTreeSet<String>,
    /// Whether the term is one of the product names we already track.
    pub is_product: bool,
}

/// Words carrying no vocabulary signal.
const STOPWORDS: &[&str] = &[
    "a", "about", "above", "after", "again", "against", "all", "also", "am", "an", "and", "any",
    "are", "as", "at", "be", "because", "been", "before", "being", "below", "between", "both",
    "but", "by", "can", "could", "did", "do", "does", "doing", "don", "down", "during", "each",
    "few", "for", "from", "further", "had", "has", "have", "having", "he", "her", "here", "hers",
    "him", "his", "how", "however", "i", "if", "in", "into", "is", "it", "its", "itself", "just",
    "like", "ll", "may", "me", "might", "more", "most", "much", "must", "my", "need", "no", "nor",
    "not", "now", "of", "off", "on", "once", "one", "only", "or", "other", "our", "ours", "out",
    "over", "own", "re", "s", "same", "she", "should", "so", "some", "such", "t", "than", "that",
    "the", "their", "theirs", "them", "then", "there", "these", "they", "this", "those", "through",
    "to", "too", "under", "until", "up", "use", "used", "using", "very", "want", "was", "we",
    "well", "were", "what", "when", "where", "which", "while", "who", "whom", "why", "will",
    "with", "would", "you", "your", "yours",
];

fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(&token)
}

/// Split text into comparable tokens.
///
/// Digits are kept because half this category's product names contain one
/// (`mem0`, `gpt-4`), and `-` is kept inside a token so `long-term` survives.
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '+' {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out.into_iter()
        .map(|t| t.trim_matches('-').to_string())
        .filter(|t| t.len() >= 2)
        .collect()
}

/// The ranked vocabulary the engines actually use for this category.
///
/// Input is `(engine, answer text)` pairs. Output is ranked by document
/// frequency, then alphabetically so the file is diffable between runs.
///
/// This exists because our marketing vocabulary is not the models' vocabulary,
/// and you cannot optimise for language you have not observed. It is
/// deliberately *not* filtered against our own copy — the overlap, or the lack
/// of it, is the finding.
pub fn key_terms(answers: &[(String, String)]) -> Vec<TermCount> {
    let product_terms: BTreeSet<String> = brand::ALLSOURCE_ALIASES
        .iter()
        .copied()
        .chain(COMPETITORS.iter().flat_map(|c| c.aliases.iter().copied()))
        .map(str::to_string)
        .collect();

    let mut counts: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();

    for (engine, text) in answers {
        let tokens = tokenize(text);
        let mut in_this_answer: BTreeSet<String> = BTreeSet::new();
        for n in 1..=MAX_NGRAM {
            if tokens.len() < n {
                break;
            }
            for window in tokens.windows(n) {
                // An n-gram that starts or ends on a stopword is a fragment
                // ("of the store"); one made entirely of stopwords is noise.
                if is_stopword(&window[0]) || is_stopword(&window[n - 1]) {
                    continue;
                }
                if window.iter().all(|t| t.chars().all(|c| c.is_ascii_digit())) {
                    continue;
                }
                in_this_answer.insert(window.join(" "));
            }
        }
        for term in in_this_answer {
            let entry = counts.entry(term).or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(engine.clone());
        }
    }

    let mut terms: Vec<TermCount> = counts
        .into_iter()
        .filter(|(_, (documents, _))| *documents >= MIN_TERM_DOCUMENTS)
        .map(|(term, (documents, engines))| TermCount {
            is_product: product_terms.contains(&term),
            term,
            documents,
            engines,
        })
        .collect();

    terms.sort_by(|a, b| {
        b.documents
            .cmp(&a.documents)
            .then_with(|| a.term.cmp(&b.term))
    });
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_first_place_mention_scores_one() {
        let scored = score_sov("AllSource is the one I'd reach for, over Mem0.");
        assert!(scored.mentioned);
        assert_eq!(scored.rank, Some(1));
        assert_eq!(scored.score, 1.0);
        assert_eq!(scored.competitors, vec!["mem0"]);
    }

    #[test]
    fn rank_is_a_position_among_every_named_product() {
        let scored = score_sov("Try Mem0, then Zep, then Letta. AllSource is another option.");
        assert_eq!(scored.rank, Some(4));
        assert!((scored.score - 0.25).abs() < 1e-9);
        assert_eq!(scored.competitors, vec!["mem0", "zep", "letta"]);
    }

    #[test]
    fn an_absent_brand_scores_zero_but_still_records_the_field() {
        // Competitor share is what makes SOV relative rather than absolute, so
        // a non-mention still has to record who *was* named.
        let scored = score_sov("Mem0 and Zep are the usual answers.");
        assert!(!scored.mentioned);
        assert_eq!(scored.rank, None);
        assert_eq!(scored.score, 0.0);
        assert_eq!(scored.competitors, vec!["mem0", "zep"]);
    }

    #[test]
    fn a_hedged_answer_is_flagged_as_a_gap() {
        let scored = score_sov("I'm not familiar with any product by that name.");
        assert!(scored.hedge.is_some(), "{scored:?}");
        assert!(!scored.mentioned);
    }

    #[test]
    fn a_confident_answer_is_not_a_gap() {
        assert!(hedge_phrase("Mem0 is the most popular choice.").is_none());
    }

    #[test]
    fn zero_of_twelve_is_not_zero_percent_plus_or_minus_nothing() {
        // The reason the interval is Wilson: the normal approximation gives a
        // zero-width interval here, which reads as certainty we do not have.
        let est = RateEstimate::new(0, 12);
        assert_eq!(est.point, 0.0);
        assert_eq!(est.low, 0.0);
        assert!(est.high > 0.20, "upper bound was {}", est.high);
        assert!(est.high < 0.30, "upper bound was {}", est.high);
    }

    #[test]
    fn a_full_sweep_of_hits_still_admits_doubt() {
        let est = RateEstimate::new(12, 12);
        assert_eq!(est.point, 1.0);
        assert!(est.low < 0.80, "lower bound was {}", est.low);
        assert_eq!(est.high, 1.0);
    }

    #[test]
    fn more_samples_narrow_the_interval() {
        let small = RateEstimate::new(3, 12);
        let large = RateEstimate::new(30, 120);
        assert!((small.point - large.point).abs() < 1e-9);
        assert!(
            (large.high - large.low) < (small.high - small.low),
            "n=120 interval was not narrower than n=12"
        );
    }

    #[test]
    fn a_rendered_rate_always_carries_its_interval_and_its_n() {
        let rendered = RateEstimate::new(3, 12).render();
        assert!(rendered.contains('['), "{rendered}");
        assert!(rendered.contains("n=12"), "{rendered}");
    }

    #[test]
    fn no_samples_renders_as_no_samples_not_as_zero() {
        assert!(RateEstimate::new(0, 0).render().contains("no samples"));
    }

    #[test]
    fn overlapping_intervals_are_detected() {
        let a = RateEstimate::new(3, 12);
        let b = RateEstimate::new(5, 12);
        assert!(a.overlaps(&b), "3/12 vs 5/12 must not read as a real move");
        let c = RateEstimate::new(0, 200);
        let d = RateEstimate::new(100, 200);
        assert!(!c.overlaps(&d));
    }

    #[test]
    fn a_tally_accumulates_trials() {
        let mut tally = RateTally::default();
        tally.record(true);
        tally.record(false);
        tally.record(true);
        assert_eq!(tally.successes, 2);
        assert_eq!(tally.total, 3);
        assert!((tally.estimate().point - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn our_own_surfaces_are_separated_from_third_party_ones() {
        assert_eq!(
            classify_source("https://www.all-source.xyz/pricing"),
            SourceOwner::Ours
        );
        assert_eq!(
            classify_source("https://github.com/all-source-os/all-source"),
            SourceOwner::Ours
        );
        assert_eq!(
            classify_source("https://github.com/someone-else/blog"),
            SourceOwner::ThirdParty
        );
        assert_eq!(
            classify_source("https://news.ycombinator.com/item?id=1"),
            SourceOwner::ThirdParty
        );
    }

    #[test]
    fn a_lookalike_domain_is_not_ours() {
        assert_eq!(
            classify_source("https://all-source.xyz.evil.example/"),
            SourceOwner::ThirdParty
        );
    }

    #[test]
    fn hosts_are_extracted_without_the_scheme_or_path() {
        assert_eq!(host_of("https://example.com/a/b?c=1"), "example.com");
        assert_eq!(host_of("http://EXAMPLE.com"), "example.com");
    }

    fn answers() -> Vec<(String, String)> {
        vec![
            (
                "chatgpt".to_string(),
                "For agent memory, most people use a vector database with retrieval.".to_string(),
            ),
            (
                "claude".to_string(),
                "Agent memory usually means a vector database plus summarisation.".to_string(),
            ),
            (
                "gemini".to_string(),
                "You could use Mem0 for agent memory.".to_string(),
            ),
        ]
    }

    #[test]
    fn key_terms_rank_by_how_many_answers_used_them() {
        let terms = key_terms(&answers());
        let agent_memory = terms
            .iter()
            .find(|t| t.term == "agent memory")
            .expect("bigram observed in all three answers");
        assert_eq!(agent_memory.documents, 3);
        assert_eq!(agent_memory.engines.len(), 3);
        let vector_db = terms
            .iter()
            .find(|t| t.term == "vector database")
            .expect("bigram observed twice");
        assert_eq!(vector_db.documents, 2);
        // Ranking: more documents first.
        let pos = |needle: &str| terms.iter().position(|t| t.term == needle).unwrap();
        assert!(pos("agent memory") < pos("vector database"));
    }

    #[test]
    fn a_term_seen_once_is_a_coincidence_not_vocabulary() {
        let terms = key_terms(&answers());
        assert!(
            !terms.iter().any(|t| t.term == "summarisation"),
            "a single-answer term leaked into the map"
        );
    }

    #[test]
    fn product_names_are_marked_rather_than_dropped() {
        let repeated = vec![
            ("chatgpt".to_string(), "Mem0 is popular.".to_string()),
            ("claude".to_string(), "Mem0 is popular.".to_string()),
        ];
        let terms = key_terms(&repeated);
        let mem0 = terms.iter().find(|t| t.term == "mem0").expect("mem0");
        assert!(mem0.is_product);
    }

    #[test]
    fn ngrams_never_start_or_end_on_a_stopword() {
        let terms = key_terms(&answers());
        for term in &terms {
            let words: Vec<&str> = term.term.split(' ').collect();
            assert!(!is_stopword(words[0]), "{}", term.term);
            assert!(!is_stopword(words[words.len() - 1]), "{}", term.term);
        }
    }

    #[test]
    fn tokenizing_keeps_the_digits_and_hyphens_this_category_needs() {
        let tokens = tokenize("Use mem0 for long-term memory (GPT-4).");
        assert!(tokens.contains(&"mem0".to_string()), "{tokens:?}");
        assert!(tokens.contains(&"long-term".to_string()), "{tokens:?}");
        assert!(tokens.contains(&"gpt-4".to_string()), "{tokens:?}");
    }
}
