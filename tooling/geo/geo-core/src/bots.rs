//! The versioned AI-bot taxonomy, and identity verification against the
//! vendors' own published IP ranges.
//!
//! # Why three categories and never a blended number
//!
//! The single most important fact about AI bot traffic is that the three kinds
//! of bot mean completely different things, and averaging them together
//! destroys the signal:
//!
//! - [`BotCategory::TrainingCrawler`] — corpus collection. Answers *"is the
//!   site readable by the thing that will one day answer questions about us?"*
//!   Moves on the vendor's training schedule, not on ours.
//! - [`BotCategory::SearchIndexer`] — the retrieval index an assistant searches
//!   at answer time. Answers *"are we eligible to be cited?"*
//! - [`BotCategory::UserFetcher`] — a human, right now, asked an assistant
//!   something and the assistant went and fetched one of our pages. This is
//!   real-time demand and it is the closest thing layer 2 has to a lead.
//!
//! Cloudflare's June 2025 crawl-to-referral figures (OpenAI ~1,700:1,
//! Anthropic ~73,000:1, versus Google ~14:1) are why layer 2 exists at all:
//! crawl volume moves months before referral volume does. A blended
//! "AI bot hits" number would hide exactly the movement we are watching for.
//!
//! # Why verification is not optional
//!
//! A `User-Agent` header is a string the client chose. Scrapers routinely
//! claim to be `GPTBot` to get past a permissive robots.txt. An unverified
//! count is not a measurement, so every entry here carries a
//! [`Verification`] telling you whether the vendor publishes the IP ranges
//! needed to check the claim, and where.
//!
//! Four vendors (OpenAI, Anthropic, Perplexity, Google) publish the *same*
//! JSON shape — `{"creationTime": ..., "prefixes": [{"ipv4Prefix": ...}]}` —
//! so one parser ([`PublishedPrefixes`]) covers all of them. Vendors that
//! publish nothing are represented honestly as
//! [`Verification::NoPublishedRanges`]: their hits can be counted but can
//! never be marked `verified`.
//!
//! # Versioning
//!
//! [`TAXONOMY_VERSION`] is stamped alongside emitted crawl events by the
//! ingest. Bump it whenever a bot moves category or a verification source
//! changes, so a historical count can be read against the taxonomy that
//! produced it.

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use serde::Deserialize;

/// Version of the taxonomy below. Bump on any category move, addition, removal
/// or verification-source change.
pub const TAXONOMY_VERSION: u32 = 1;

// ───────────────────────────────────────────────────────────────────────────
// Categories
// ───────────────────────────────────────────────────────────────────────────

/// What a bot's visit *means*. Never aggregate across these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BotCategory {
    /// Collects pages into a training corpus.
    TrainingCrawler,
    /// Builds the retrieval index an assistant searches at answer time.
    SearchIndexer,
    /// Fetches a page because a human asked an assistant something just now.
    UserFetcher,
}

impl BotCategory {
    /// Every category, in "furthest from revenue" → "closest to revenue" order.
    pub const ALL: [Self; 3] = [
        Self::TrainingCrawler,
        Self::SearchIndexer,
        Self::UserFetcher,
    ];

    /// The wire string stored in `geo.crawl.observed.category`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TrainingCrawler => "training_crawler",
            Self::SearchIndexer => "search_indexer",
            Self::UserFetcher => "user_fetcher",
        }
    }

    /// Parse a wire string. Unknown categories return `None` rather than being
    /// folded into a neighbour — a misfiled bot is worse than a visible gap.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    /// Short human label for report headings.
    pub fn label(self) -> &'static str {
        match self {
            Self::TrainingCrawler => "Training crawlers",
            Self::SearchIndexer => "Search indexers",
            Self::UserFetcher => "User-triggered fetchers",
        }
    }

    /// What a rise in this category actually tells you.
    pub fn means(self) -> &'static str {
        match self {
            Self::TrainingCrawler => "infrastructure readiness — we are reachable by the corpus",
            Self::SearchIndexer => "citation eligibility — we are in the answer-time index",
            Self::UserFetcher => "real-time human demand — someone is asking about us now",
        }
    }
}

impl std::fmt::Display for BotCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The taxonomy
// ───────────────────────────────────────────────────────────────────────────

/// How a claimed bot identity can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verification {
    /// The vendor publishes its crawler IP ranges as JSON at this URL, in the
    /// shared `{creationTime, prefixes:[{ipv4Prefix|ipv6Prefix}]}` shape.
    PublishedRanges { url: &'static str },
    /// The vendor publishes nothing usable. Hits are still countable, but can
    /// never be marked `verified` — and we say so rather than pretending.
    NoPublishedRanges { reason: &'static str },
}

impl Verification {
    /// The range-list URL, when there is one.
    pub fn ranges_url(self) -> Option<&'static str> {
        match self {
            Self::PublishedRanges { url } => Some(url),
            Self::NoPublishedRanges { .. } => None,
        }
    }
}

/// One bot in the taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BotSpec {
    /// Normalised id stored in `geo.crawl.observed.bot` (lowercase, stable).
    pub id: &'static str,
    /// The distinguishing token to look for in a `User-Agent`, matched
    /// case-insensitively.
    pub ua_token: &'static str,
    /// Which of the three meanings this bot's visit carries.
    pub category: BotCategory,
    /// Who operates it.
    pub owner: &'static str,
    /// The vendor's own documentation for this bot. Cited so a future reader
    /// can re-check a categorisation against the source rather than trusting
    /// this table.
    pub docs_url: &'static str,
    /// How a claimed identity is checked.
    pub verification: Verification,
}

/// OpenAI's bot documentation, shared by its three agents.
const OPENAI_DOCS: &str = "https://platform.openai.com/docs/bots";
/// Anthropic's crawler documentation.
const ANTHROPIC_DOCS: &str = "https://claude.com/crawling";
/// One JSON file covers ClaudeBot, Claude-SearchBot and Claude-User.
const ANTHROPIC_RANGES: &str = "https://claude.com/crawling/bots.json";
/// Perplexity's bot documentation.
const PERPLEXITY_DOCS: &str = "https://docs.perplexity.ai/guides/bots";

/// The taxonomy.
///
/// Every entry cites the vendor's own documentation next to it. Entries with
/// [`Verification::NoPublishedRanges`] are deliberately kept: a hit from an
/// unverifiable bot is real data, it is just data we refuse to promote to
/// `verified`.
///
/// Deliberately **absent**: `Google-Extended` and `Applebot-Extended`. Those
/// are robots.txt opt-out *tokens*, not user agents — they never appear in an
/// access log, so putting them here would create a category that is
/// permanently zero and read as "Google never crawls us". They belong in
/// `robots.txt`, and that is where they are.
pub const BOTS: &[BotSpec] = &[
    // ── Training crawlers ────────────────────────────────────────────────
    BotSpec {
        id: "gptbot",
        ua_token: "GPTBot",
        category: BotCategory::TrainingCrawler,
        owner: "OpenAI",
        docs_url: OPENAI_DOCS,
        verification: Verification::PublishedRanges {
            url: "https://openai.com/gptbot.json",
        },
    },
    BotSpec {
        id: "claudebot",
        ua_token: "ClaudeBot",
        category: BotCategory::TrainingCrawler,
        owner: "Anthropic",
        docs_url: ANTHROPIC_DOCS,
        verification: Verification::PublishedRanges {
            url: ANTHROPIC_RANGES,
        },
    },
    BotSpec {
        id: "anthropic-ai",
        ua_token: "anthropic-ai",
        category: BotCategory::TrainingCrawler,
        owner: "Anthropic",
        docs_url: ANTHROPIC_DOCS,
        verification: Verification::PublishedRanges {
            url: ANTHROPIC_RANGES,
        },
    },
    BotSpec {
        id: "ccbot",
        ua_token: "CCBot",
        category: BotCategory::TrainingCrawler,
        owner: "Common Crawl",
        docs_url: "https://commoncrawl.org/ccbot",
        verification: Verification::NoPublishedRanges {
            reason: "Common Crawl documents CCBot's user agent but publishes no IP range list",
        },
    },
    BotSpec {
        id: "bytespider",
        ua_token: "Bytespider",
        category: BotCategory::TrainingCrawler,
        owner: "ByteDance",
        docs_url: "https://commoncrawl.org/ccbot",
        verification: Verification::NoPublishedRanges {
            reason: "ByteDance publishes no crawler IP range list",
        },
    },
    BotSpec {
        id: "meta-externalagent",
        ua_token: "meta-externalagent",
        category: BotCategory::TrainingCrawler,
        owner: "Meta",
        docs_url: "https://developers.facebook.com/docs/sharing/webmasters/web-crawlers",
        verification: Verification::NoPublishedRanges {
            reason: "Meta documents the agent but publishes no IP range list for it",
        },
    },
    // ── Search indexers ──────────────────────────────────────────────────
    BotSpec {
        id: "oai-searchbot",
        ua_token: "OAI-SearchBot",
        category: BotCategory::SearchIndexer,
        owner: "OpenAI",
        docs_url: OPENAI_DOCS,
        verification: Verification::PublishedRanges {
            url: "https://openai.com/searchbot.json",
        },
    },
    BotSpec {
        id: "claude-searchbot",
        ua_token: "Claude-SearchBot",
        category: BotCategory::SearchIndexer,
        owner: "Anthropic",
        docs_url: ANTHROPIC_DOCS,
        verification: Verification::PublishedRanges {
            url: ANTHROPIC_RANGES,
        },
    },
    BotSpec {
        id: "perplexitybot",
        ua_token: "PerplexityBot",
        category: BotCategory::SearchIndexer,
        owner: "Perplexity",
        docs_url: PERPLEXITY_DOCS,
        verification: Verification::PublishedRanges {
            url: "https://www.perplexity.ai/perplexitybot.json",
        },
    },
    BotSpec {
        id: "google-cloudvertexbot",
        ua_token: "Google-CloudVertexBot",
        category: BotCategory::SearchIndexer,
        owner: "Google",
        docs_url: "https://developers.google.com/search/docs/crawling-indexing/google-common-crawlers",
        verification: Verification::PublishedRanges {
            url: "https://developers.google.com/static/crawling/ipranges/special-crawlers.json",
        },
    },
    BotSpec {
        id: "duckassistbot",
        ua_token: "DuckAssistBot",
        category: BotCategory::SearchIndexer,
        owner: "DuckDuckGo",
        docs_url: "https://duckduckgo.com/duckduckgo-help-pages/results/duckassistbot/",
        verification: Verification::NoPublishedRanges {
            reason: "DuckDuckGo documents verification by reverse DNS, not by a published range list",
        },
    },
    // ── User-triggered fetchers ──────────────────────────────────────────
    BotSpec {
        id: "chatgpt-user",
        ua_token: "ChatGPT-User",
        category: BotCategory::UserFetcher,
        owner: "OpenAI",
        docs_url: OPENAI_DOCS,
        verification: Verification::PublishedRanges {
            url: "https://openai.com/chatgpt-user.json",
        },
    },
    BotSpec {
        id: "claude-user",
        ua_token: "Claude-User",
        category: BotCategory::UserFetcher,
        owner: "Anthropic",
        docs_url: ANTHROPIC_DOCS,
        verification: Verification::PublishedRanges {
            url: ANTHROPIC_RANGES,
        },
    },
    BotSpec {
        id: "perplexity-user",
        ua_token: "Perplexity-User",
        category: BotCategory::UserFetcher,
        owner: "Perplexity",
        docs_url: PERPLEXITY_DOCS,
        verification: Verification::PublishedRanges {
            url: "https://www.perplexity.ai/perplexity-user.json",
        },
    },
    BotSpec {
        id: "meta-externalfetcher",
        ua_token: "meta-externalfetcher",
        category: BotCategory::UserFetcher,
        owner: "Meta",
        docs_url: "https://developers.facebook.com/docs/sharing/webmasters/web-crawlers",
        verification: Verification::NoPublishedRanges {
            reason: "Meta documents the fetcher but publishes no IP range list for it",
        },
    },
];

/// Identify the bot behind a `User-Agent`, if any.
///
/// Returns the **longest** matching token so a future short token can never
/// shadow a longer one. (A test also asserts no token is a substring of
/// another, so this is belt and braces.)
pub fn identify(user_agent: &str) -> Option<&'static BotSpec> {
    let haystack = user_agent.to_ascii_lowercase();
    BOTS.iter()
        .filter(|spec| haystack.contains(&spec.ua_token.to_ascii_lowercase()))
        .max_by_key(|spec| spec.ua_token.len())
}

/// Look a bot up by its normalised id (as stored in an emitted event).
pub fn by_id(id: &str) -> Option<&'static BotSpec> {
    BOTS.iter().find(|spec| spec.id == id)
}

/// Every distinct published-range URL, mapped to the bot ids it verifies.
///
/// One Anthropic file covers three bots, so the ingest fetches per *URL*, not
/// per bot.
pub fn range_sources() -> BTreeMap<&'static str, Vec<&'static str>> {
    let mut out: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    for spec in BOTS {
        if let Some(url) = spec.verification.ranges_url() {
            out.entry(url).or_default().push(spec.id);
        }
    }
    out
}

// ───────────────────────────────────────────────────────────────────────────
// CIDR matching
// ───────────────────────────────────────────────────────────────────────────

/// One published prefix.
///
/// Hand-rolled rather than pulled from a crate: the whole of CIDR containment
/// is a masked byte comparison, and a verification bug that silently rejects
/// everything looks exactly like "no AI traffic" — which is precisely the
/// misread that would derail this program. Code we can unit-test line by line
/// is worth more here than a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    base: IpAddr,
    bits: u8,
}

impl Cidr {
    /// Parse `"1.2.3.0/24"` or `"2001:db8::/32"`. A bare address is treated as
    /// a full-length prefix.
    pub fn parse(s: &str) -> Result<Self, String> {
        let (addr_part, bits_part) = match s.split_once('/') {
            Some((a, b)) => (a, Some(b)),
            None => (s, None),
        };
        let base: IpAddr = addr_part
            .trim()
            .parse()
            .map_err(|e| format!("{s:?} is not an IP prefix: {e}"))?;
        let max_bits = if base.is_ipv4() { 32 } else { 128 };
        let bits = match bits_part {
            None => max_bits,
            Some(b) => {
                let parsed: u8 = b
                    .trim()
                    .parse()
                    .map_err(|e| format!("{s:?} has a bad prefix length: {e}"))?;
                if parsed > max_bits {
                    return Err(format!("{s:?} prefix length exceeds {max_bits}"));
                }
                parsed
            }
        };
        Ok(Self { base, bits })
    }

    /// Does this prefix contain `ip`?
    ///
    /// A v4 address is never matched by a v6 prefix or vice versa — no
    /// v4-mapped-v6 leniency, because a mismatch there would silently widen a
    /// vendor's range.
    pub fn contains(&self, ip: &IpAddr) -> bool {
        match (self.base, ip) {
            (IpAddr::V4(base), IpAddr::V4(probe)) => {
                masked_eq(&base.octets(), &probe.octets(), self.bits)
            }
            (IpAddr::V6(base), IpAddr::V6(probe)) => {
                masked_eq(&base.octets(), &probe.octets(), self.bits)
            }
            _ => false,
        }
    }

    /// Prefix length in bits.
    pub fn bits(&self) -> u8 {
        self.bits
    }
}

impl std::fmt::Display for Cidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.bits)
    }
}

/// Compare the first `bits` bits of two equal-length octet arrays.
fn masked_eq(a: &[u8], b: &[u8], bits: u8) -> bool {
    let whole = usize::from(bits / 8);
    let rest = bits % 8;
    if a[..whole] != b[..whole] {
        return false;
    }
    if rest == 0 {
        return true;
    }
    let mask = 0xFFu8 << (8 - rest);
    (a[whole] & mask) == (b[whole] & mask)
}

// ───────────────────────────────────────────────────────────────────────────
// Published range files
// ───────────────────────────────────────────────────────────────────────────

/// One entry in a vendor's published prefix list.
#[derive(Debug, Clone, Deserialize)]
pub struct PublishedPrefix {
    #[serde(rename = "ipv4Prefix")]
    pub ipv4_prefix: Option<String>,
    #[serde(rename = "ipv6Prefix")]
    pub ipv6_prefix: Option<String>,
}

/// A vendor's published crawler prefix list.
///
/// OpenAI, Anthropic, Perplexity and Google all publish this exact shape, so
/// this one type parses all four.
#[derive(Debug, Clone, Deserialize)]
pub struct PublishedPrefixes {
    /// When the vendor last regenerated the list. Reported so a stale file is
    /// visible rather than silently trusted.
    #[serde(rename = "creationTime")]
    pub creation_time: Option<String>,
    pub prefixes: Vec<PublishedPrefix>,
}

/// The verdict on one claimed bot identity.
///
/// Note the three different shades of "not verified". Collapsing them into a
/// bool would make a spoofing attempt indistinguishable from a vendor that
/// publishes nothing, and from our own fetch having failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Claimed identity confirmed against the vendor's published ranges.
    Verified,
    /// The vendor publishes ranges and this IP is not in them. Someone is
    /// wearing the bot's user agent. Never emitted as a crawl event.
    Rejected,
    /// The vendor publishes no ranges, so the claim is uncheckable.
    UnverifiableNoRanges,
    /// The log line carried no client IP, so there is nothing to check.
    UnverifiableNoClientIp,
    /// We could not load the vendor's range list this run (offline, or the
    /// fetch failed). Distinct from `Rejected` on purpose: a fetch failure
    /// must never be reported as a spoof.
    UnverifiableRangesUnavailable,
}

impl Verdict {
    /// Only [`Verdict::Verified`] is a verified hit.
    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }

    /// Short label for report output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Rejected => "rejected (IP outside vendor range)",
            Self::UnverifiableNoRanges => "unverifiable (vendor publishes no ranges)",
            Self::UnverifiableNoClientIp => "unverifiable (log line had no client IP)",
            Self::UnverifiableRangesUnavailable => "unverifiable (range list unavailable)",
        }
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Published prefixes, keyed by the URL they came from.
#[derive(Debug, Clone, Default)]
pub struct RangeCatalog {
    by_url: BTreeMap<String, Vec<Cidr>>,
    creation_times: BTreeMap<String, String>,
}

impl RangeCatalog {
    /// An empty catalog. Every check against it returns
    /// [`Verdict::UnverifiableRangesUnavailable`] — never `Rejected`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load one vendor file. Returns how many prefixes were understood.
    ///
    /// Unparseable prefixes are skipped rather than failing the whole file: a
    /// vendor adding a format we do not know must not blind us to the ranges
    /// we do know.
    pub fn insert_json(&mut self, url: &str, json: &str) -> Result<usize, String> {
        let parsed: PublishedPrefixes =
            serde_json::from_str(json).map_err(|e| format!("{url}: not a prefix list: {e}"))?;
        let mut cidrs = Vec::with_capacity(parsed.prefixes.len());
        for prefix in &parsed.prefixes {
            for raw in [prefix.ipv4_prefix.as_deref(), prefix.ipv6_prefix.as_deref()]
                .into_iter()
                .flatten()
            {
                if let Ok(cidr) = Cidr::parse(raw) {
                    cidrs.push(cidr);
                }
            }
        }
        if cidrs.is_empty() {
            return Err(format!("{url}: prefix list parsed but held no usable prefix"));
        }
        let count = cidrs.len();
        self.by_url.insert(url.to_string(), cidrs);
        if let Some(created) = parsed.creation_time {
            self.creation_times.insert(url.to_string(), created);
        }
        Ok(count)
    }

    /// Whether a URL's ranges are loaded.
    pub fn has(&self, url: &str) -> bool {
        self.by_url.contains_key(url)
    }

    /// The vendor's own `creationTime` for a loaded URL.
    pub fn creation_time(&self, url: &str) -> Option<&str> {
        self.creation_times.get(url).map(String::as_str)
    }

    /// Number of prefixes loaded for a URL.
    pub fn prefix_count(&self, url: &str) -> usize {
        self.by_url.get(url).map_or(0, Vec::len)
    }

    /// Check one claimed identity.
    pub fn verify(&self, spec: &BotSpec, client_ip: Option<IpAddr>) -> Verdict {
        let Some(url) = spec.verification.ranges_url() else {
            return Verdict::UnverifiableNoRanges;
        };
        let Some(ranges) = self.by_url.get(url) else {
            return Verdict::UnverifiableRangesUnavailable;
        };
        let Some(ip) = client_ip else {
            return Verdict::UnverifiableNoClientIp;
        };
        if ranges.iter().any(|cidr| cidr.contains(&ip)) {
            Verdict::Verified
        } else {
            Verdict::Rejected
        }
    }
}

/// Cache/fixture file name for a range URL: stable, filesystem-safe, and
/// readable enough that a human can tell which vendor a cached file came from.
pub fn range_cache_file_name(url: &str) -> String {
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let mut out = String::with_capacity(stripped.len());
    for ch in stripped.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

/// Convenience for constructing a v4 [`IpAddr`] in tests and fixtures.
pub fn ipv4(a: u8, b: u8, c: u8, d: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(a, b, c, d))
}

/// Convenience for constructing a v6 [`IpAddr`].
pub fn ipv6(segments: [u16; 8]) -> IpAddr {
    IpAddr::V6(Ipv6Addr::new(
        segments[0],
        segments[1],
        segments[2],
        segments[3],
        segments[4],
        segments[5],
        segments[6],
        segments[7],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_ua_token_shadows_another() {
        // If one token were a substring of another, identification would
        // depend on iteration order and a fetcher could be counted as a
        // training crawler — the exact conflation this module exists to stop.
        for a in BOTS {
            for b in BOTS {
                if a.id == b.id {
                    continue;
                }
                assert!(
                    !a.ua_token
                        .to_ascii_lowercase()
                        .contains(&b.ua_token.to_ascii_lowercase()),
                    "{} contains {} — categorisation would be order-dependent",
                    a.ua_token,
                    b.ua_token
                );
            }
        }
    }

    #[test]
    fn ids_are_unique_lowercase_and_documented() {
        let mut seen = std::collections::BTreeSet::new();
        for spec in BOTS {
            assert!(seen.insert(spec.id), "duplicate bot id {}", spec.id);
            assert_eq!(spec.id, spec.id.to_ascii_lowercase(), "{}", spec.id);
            assert!(
                spec.docs_url.starts_with("https://"),
                "{} must cite the vendor's own documentation",
                spec.id
            );
        }
    }

    #[test]
    fn all_three_categories_are_populated() {
        for category in BotCategory::ALL {
            assert!(
                BOTS.iter().any(|b| b.category == category),
                "{category} has no bots — a permanently-zero category reads as 'no traffic'"
            );
        }
    }

    #[test]
    fn the_three_openai_agents_land_in_three_different_categories() {
        // This is the framework's central claim; if it ever stops holding,
        // the report is lying.
        assert_eq!(
            by_id("gptbot").unwrap().category,
            BotCategory::TrainingCrawler
        );
        assert_eq!(
            by_id("oai-searchbot").unwrap().category,
            BotCategory::SearchIndexer
        );
        assert_eq!(
            by_id("chatgpt-user").unwrap().category,
            BotCategory::UserFetcher
        );
    }

    #[test]
    fn identifies_real_user_agent_strings() {
        let cases = [
            (
                "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; \
                 GPTBot/1.2; +https://openai.com/gptbot",
                "gptbot",
            ),
            (
                "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; \
                 OAI-SearchBot/1.0; +https://openai.com/searchbot",
                "oai-searchbot",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/120 Safari/537.36; compatible; \
                 ChatGPT-User/1.0; +https://openai.com/bot",
                "chatgpt-user",
            ),
            (
                "Mozilla/5.0 (compatible; ClaudeBot/1.0; \
                 +claudebot@anthropic.com)",
                "claudebot",
            ),
            (
                "Mozilla/5.0 (compatible; Claude-SearchBot/1.0; \
                 +claudebot@anthropic.com)",
                "claude-searchbot",
            ),
            (
                "Mozilla/5.0 (compatible; Claude-User/1.0; \
                 +claudebot@anthropic.com)",
                "claude-user",
            ),
            (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
                 AppleWebKit/537.36 (KHTML, like Gecko) PerplexityBot/1.0; \
                 +https://perplexity.ai/perplexitybot",
                "perplexitybot",
            ),
            ("CCBot/2.0 (https://commoncrawl.org/faq/)", "ccbot"),
        ];
        for (ua, expected) in cases {
            assert_eq!(
                identify(ua).map(|s| s.id),
                Some(expected),
                "failed to identify {ua}"
            );
        }
    }

    #[test]
    fn a_human_browser_is_not_a_bot() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
        assert!(identify(ua).is_none());
    }

    #[test]
    fn cidr_containment_is_exact_at_the_boundaries() {
        let net = Cidr::parse("172.182.202.0/25").expect("parses");
        assert!(net.contains(&ipv4(172, 182, 202, 0)));
        assert!(net.contains(&ipv4(172, 182, 202, 127)));
        assert!(!net.contains(&ipv4(172, 182, 202, 128)));
        assert!(!net.contains(&ipv4(172, 182, 203, 0)));
    }

    #[test]
    fn cidr_handles_a_single_host_and_a_whole_v4_space() {
        let host = Cidr::parse("3.224.62.45/32").expect("parses");
        assert!(host.contains(&ipv4(3, 224, 62, 45)));
        assert!(!host.contains(&ipv4(3, 224, 62, 46)));

        let all = Cidr::parse("0.0.0.0/0").expect("parses");
        assert!(all.contains(&ipv4(198, 51, 100, 7)));
    }

    #[test]
    fn cidr_never_matches_across_address_families() {
        let v4 = Cidr::parse("0.0.0.0/0").expect("parses");
        assert!(!v4.contains(&ipv6([0x2001, 0x4860, 0, 0, 0, 0, 0, 1])));

        let v6 = Cidr::parse("2001:4860:4801:2008::/64").expect("parses");
        assert!(v6.contains(&ipv6([0x2001, 0x4860, 0x4801, 0x2008, 0, 0, 0, 5])));
        assert!(!v6.contains(&ipv6([0x2001, 0x4860, 0x4801, 0x2009, 0, 0, 0, 5])));
        assert!(!v6.contains(&ipv4(1, 2, 3, 4)));
    }

    #[test]
    fn a_bare_address_is_a_full_length_prefix() {
        let net = Cidr::parse("192.0.2.10").expect("parses");
        assert_eq!(net.bits(), 32);
        assert!(net.contains(&ipv4(192, 0, 2, 10)));
        assert!(!net.contains(&ipv4(192, 0, 2, 11)));
    }

    #[test]
    fn a_bad_prefix_is_an_error_not_a_silent_match_all() {
        assert!(Cidr::parse("not-an-ip/24").is_err());
        assert!(Cidr::parse("1.2.3.4/33").is_err());
    }

    const OPENAI_SAMPLE: &str = r#"{
      "creationTime": "2025-10-30T11:00:00.000000",
      "prefixes": [
        {"ipv4Prefix": "132.196.86.0/24"},
        {"ipv4Prefix": "20.125.66.80/28"},
        {"ipv6Prefix": "2600:1f18::/32"}
      ]
    }"#;

    fn loaded_catalog() -> RangeCatalog {
        let mut catalog = RangeCatalog::new();
        let url = by_id("gptbot").unwrap().verification.ranges_url().unwrap();
        let n = catalog.insert_json(url, OPENAI_SAMPLE).expect("loads");
        assert_eq!(n, 3);
        catalog
    }

    #[test]
    fn the_shared_vendor_format_parses() {
        let catalog = loaded_catalog();
        let url = by_id("gptbot").unwrap().verification.ranges_url().unwrap();
        assert!(catalog.has(url));
        assert_eq!(catalog.prefix_count(url), 3);
        assert_eq!(catalog.creation_time(url), Some("2025-10-30T11:00:00.000000"));
    }

    #[test]
    fn an_in_range_ip_verifies_and_an_out_of_range_one_is_rejected() {
        let catalog = loaded_catalog();
        let gptbot = by_id("gptbot").unwrap();
        assert_eq!(
            catalog.verify(gptbot, Some(ipv4(132, 196, 86, 9))),
            Verdict::Verified
        );
        assert_eq!(
            catalog.verify(gptbot, Some(ipv4(203, 0, 113, 9))),
            Verdict::Rejected
        );
    }

    #[test]
    fn a_missing_range_list_is_never_reported_as_a_spoof() {
        // The failure that would silently zero the whole programme: an empty
        // catalog must produce "unverifiable", not "rejected".
        let empty = RangeCatalog::new();
        let gptbot = by_id("gptbot").unwrap();
        assert_eq!(
            empty.verify(gptbot, Some(ipv4(132, 196, 86, 9))),
            Verdict::UnverifiableRangesUnavailable
        );
    }

    #[test]
    fn a_vendor_with_no_published_ranges_is_unverifiable_not_rejected() {
        let catalog = loaded_catalog();
        let ccbot = by_id("ccbot").unwrap();
        assert_eq!(
            catalog.verify(ccbot, Some(ipv4(1, 2, 3, 4))),
            Verdict::UnverifiableNoRanges
        );
    }

    #[test]
    fn a_log_line_with_no_client_ip_is_unverifiable() {
        let catalog = loaded_catalog();
        let gptbot = by_id("gptbot").unwrap();
        assert_eq!(
            catalog.verify(gptbot, None),
            Verdict::UnverifiableNoClientIp
        );
    }

    #[test]
    fn one_anthropic_file_serves_every_anthropic_bot() {
        // The ingest must fetch per range-URL, not per bot, or it would pull
        // the same file four times.
        let sources = range_sources();
        let anthropic = sources
            .get(ANTHROPIC_RANGES)
            .expect("Anthropic range source is registered");
        let expected: Vec<&str> = BOTS
            .iter()
            .filter(|b| b.owner == "Anthropic")
            .map(|b| b.id)
            .collect();
        assert_eq!(anthropic, &expected, "{anthropic:?}");
        assert!(anthropic.len() > 1);
    }

    #[test]
    fn range_cache_names_are_distinct_and_filesystem_safe() {
        let mut names = std::collections::BTreeSet::new();
        for url in range_sources().keys() {
            let name = range_cache_file_name(url);
            assert!(!name.contains('/'), "{name}");
            assert!(names.insert(name.clone()), "duplicate cache name {name}");
        }
    }

    #[test]
    fn categories_round_trip_through_their_wire_strings() {
        for category in BotCategory::ALL {
            assert_eq!(BotCategory::parse(category.as_str()), Some(category));
        }
        assert_eq!(BotCategory::parse("all_bots"), None);
    }
}
