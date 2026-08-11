//! Who counts as "us" and who counts as a competitor, and how to find either
//! in a wall of model prose.
//!
//! This is the reference table layer 3 shares between the probe scorer and the
//! report, for the same reason [`crate::bots`] exists for layer 2: two copies
//! of a taxonomy drift, and a drifted taxonomy silently rewrites history.
//!
//! ## Matching is boundary-aware, and that is load-bearing
//!
//! A naive `contains()` counts `redis` inside `redisearch`, `postgres` inside
//! `postgresql`, and — worst — `allsource` inside `callsource`. Every one of
//! those inflates a metric the whole program is optimised against. So a match
//! requires a non-alphanumeric character (or a string edge) on both sides, and
//! any spelling that *is* a real product gets its own alias.

/// Version of the competitor table below. Bump it when the set changes, so a
/// historical share number can be read against the set it was computed with.
pub const COMPETITOR_SET_VERSION: u32 = 1;

/// Every spelling of our own name a model might use.
///
/// `chronos` is deliberately absent: it is the monorepo's name, not the
/// product's, and it collides with a dozen unrelated projects.
pub const ALLSOURCE_ALIASES: &[&str] = &[
    "allsource",
    "all-source",
    "all source",
    "all-source.xyz",
    "allsource chronos",
    "allsource core",
    "allsource prime",
];

/// A product the engines might name instead of (or alongside) us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetitorSpec {
    /// Stable id, used in payloads and report rows.
    pub id: &'static str,
    /// Human label for report tables.
    pub label: &'static str,
    /// Lowercase spellings to look for.
    pub aliases: &'static [&'static str],
    /// What kind of answer naming it represents.
    pub kind: CompetitorKind,
}

/// Why a named product matters — not every "competitor" is a company.
///
/// The two most common answers to "how do I give my agent memory" are a
/// managed memory product and *a file*. Counting only the products would make
/// the category look far more contested, and far more commercial, than it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompetitorKind {
    /// A funded agent-memory product.
    MemoryProduct,
    /// A general datastore pressed into the role.
    Datastore,
    /// A vector database.
    VectorStore,
    /// A framework that ships a memory abstraction.
    Framework,
    /// "Just write a JSON file" — the real default.
    RollYourOwn,
}

impl CompetitorKind {
    /// Every kind, in report order.
    pub const ALL: [Self; 5] = [
        Self::MemoryProduct,
        Self::Datastore,
        Self::VectorStore,
        Self::Framework,
        Self::RollYourOwn,
    ];

    /// Wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemoryProduct => "memory_product",
            Self::Datastore => "datastore",
            Self::VectorStore => "vector_store",
            Self::Framework => "framework",
            Self::RollYourOwn => "roll_your_own",
        }
    }
}

/// The competitor table.
///
/// The first four rows are the set `apps/web/src/app/(marketing)/vs/` already
/// commits to; the rest are what actually turns up when you ask an engine this
/// category's questions.
pub const COMPETITORS: &[CompetitorSpec] = &[
    CompetitorSpec {
        id: "mem0",
        label: "Mem0",
        aliases: &["mem0", "mem zero", "mem0ai"],
        kind: CompetitorKind::MemoryProduct,
    },
    CompetitorSpec {
        id: "letta",
        label: "Letta (MemGPT)",
        aliases: &["letta", "memgpt"],
        kind: CompetitorKind::MemoryProduct,
    },
    CompetitorSpec {
        id: "zep",
        label: "Zep",
        aliases: &["zep", "graphiti"],
        kind: CompetitorKind::MemoryProduct,
    },
    CompetitorSpec {
        id: "stoolap",
        label: "Stoolap",
        aliases: &["stoolap"],
        kind: CompetitorKind::Datastore,
    },
    CompetitorSpec {
        id: "eventstoredb",
        label: "EventStoreDB / Kurrent",
        aliases: &["eventstoredb", "event store db", "eventstore", "kurrentdb", "kurrent"],
        kind: CompetitorKind::Datastore,
    },
    CompetitorSpec {
        id: "redis",
        label: "Redis",
        aliases: &["redis", "redis stack", "valkey"],
        kind: CompetitorKind::Datastore,
    },
    CompetitorSpec {
        id: "postgres",
        label: "Postgres / pgvector",
        aliases: &["postgres", "postgresql", "pgvector", "supabase"],
        kind: CompetitorKind::Datastore,
    },
    CompetitorSpec {
        id: "sqlite",
        label: "SQLite",
        aliases: &["sqlite", "sqlite3", "libsql", "turso"],
        kind: CompetitorKind::Datastore,
    },
    CompetitorSpec {
        id: "pinecone",
        label: "Pinecone",
        aliases: &["pinecone"],
        kind: CompetitorKind::VectorStore,
    },
    CompetitorSpec {
        id: "chroma",
        label: "Chroma",
        aliases: &["chroma", "chromadb"],
        kind: CompetitorKind::VectorStore,
    },
    CompetitorSpec {
        id: "qdrant",
        label: "Qdrant",
        aliases: &["qdrant"],
        kind: CompetitorKind::VectorStore,
    },
    CompetitorSpec {
        id: "weaviate",
        label: "Weaviate",
        aliases: &["weaviate"],
        kind: CompetitorKind::VectorStore,
    },
    CompetitorSpec {
        id: "langchain",
        label: "LangChain / LangGraph",
        aliases: &["langchain", "langgraph", "langmem", "langsmith"],
        kind: CompetitorKind::Framework,
    },
    CompetitorSpec {
        id: "llamaindex",
        label: "LlamaIndex",
        aliases: &["llamaindex", "llama index"],
        kind: CompetitorKind::Framework,
    },
    CompetitorSpec {
        id: "files",
        label: "Plain files",
        aliases: &[
            "plain file",
            "plain files",
            "flat file",
            "flat files",
            "json file",
            "json files",
            "markdown file",
            "markdown files",
            "a text file",
            "text files",
            "claude.md",
            "agents.md",
        ],
        kind: CompetitorKind::RollYourOwn,
    },
];

impl CompetitorSpec {
    /// Look a competitor up by id.
    pub fn by_id(id: &str) -> Option<&'static Self> {
        COMPETITORS.iter().find(|c| c.id == id)
    }
}

/// Whether a match at `start..end` in `haystack` stands alone as a word.
///
/// Both edges must be a non-alphanumeric byte or the string boundary. `_` is
/// treated as a word character so `mem0_store` does not count as `mem0`.
fn is_standalone(haystack: &str, start: usize, end: usize) -> bool {
    let bytes = haystack.as_bytes();
    let before_ok = start == 0 || {
        let b = bytes[start - 1];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    let after_ok = end >= bytes.len() || {
        let b = bytes[end];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    before_ok && after_ok
}

/// Byte offset of the first standalone occurrence of any alias, if present.
///
/// Case-insensitive. Returns the offset into the *lowercased* haystack, which
/// is only ever used for ordering, never for slicing the original.
pub fn first_mention(text: &str, aliases: &[&str]) -> Option<usize> {
    let hay = text.to_lowercase();
    let mut best: Option<usize> = None;
    for alias in aliases {
        let mut from = 0;
        while let Some(rel) = hay[from..].find(alias) {
            let start = from + rel;
            let end = start + alias.len();
            if is_standalone(&hay, start, end) {
                best = Some(best.map_or(start, |b: usize| b.min(start)));
                break;
            }
            from = start + 1;
        }
    }
    best
}

/// Whether the text names AllSource at all.
pub fn names_allsource(text: &str) -> bool {
    first_mention(text, ALLSOURCE_ALIASES).is_some()
}

/// Every named product in the text, in order of first appearance.
///
/// Returns `(id, offset)`. AllSource itself is included under the id
/// `"allsource"` so a caller can compute a rank directly from this list —
/// rank is a position among *all* named products, ours included.
pub fn named_products(text: &str) -> Vec<(&'static str, usize)> {
    let mut found: Vec<(&'static str, usize)> = Vec::new();
    if let Some(at) = first_mention(text, ALLSOURCE_ALIASES) {
        found.push(("allsource", at));
    }
    for spec in COMPETITORS {
        if let Some(at) = first_mention(text, spec.aliases) {
            found.push((spec.id, at));
        }
    }
    found.sort_by_key(|&(id, at)| (at, id));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alias_is_lowercase_and_non_empty() {
        // `first_mention` lowercases the haystack, so an uppercase alias could
        // never match — silently reporting zero share for that product.
        for alias in ALLSOURCE_ALIASES {
            assert_eq!(*alias, alias.to_lowercase(), "{alias}");
            assert!(!alias.is_empty());
        }
        for spec in COMPETITORS {
            assert!(!spec.aliases.is_empty(), "{}", spec.id);
            for alias in spec.aliases {
                assert_eq!(*alias, alias.to_lowercase(), "{alias}");
            }
        }
    }

    #[test]
    fn competitor_ids_are_unique() {
        let mut ids: Vec<&str> = COMPETITORS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len());
    }

    #[test]
    fn the_marketing_competitor_set_is_present() {
        // apps/web/src/app/(marketing)/vs/ commits to these four; the scorer
        // must not quietly stop counting one.
        for id in ["mem0", "letta", "zep", "stoolap"] {
            assert!(CompetitorSpec::by_id(id).is_some(), "{id} missing");
        }
    }

    #[test]
    fn a_mention_is_case_insensitive() {
        assert!(names_allsource("Have you tried AllSource?"));
        assert!(names_allsource("allsource is an event store"));
        assert!(names_allsource("see all-source.xyz for docs"));
    }

    #[test]
    fn a_substring_is_not_a_mention() {
        // The bug this whole module exists to prevent.
        assert!(!names_allsource("CallSource is a call-tracking product"));
        assert!(!names_allsource("recallsourcemap"));
        assert_eq!(first_mention("redisearch is fast", &["redis"]), None);
        assert_eq!(first_mention("postgresql rocks", &["postgres"]), None);
        // ...but the real spelling still matches.
        assert!(first_mention("postgresql rocks", &["postgresql"]).is_some());
    }

    #[test]
    fn an_underscore_does_not_open_a_word_boundary() {
        assert_eq!(first_mention("mem0_adapter", &["mem0"]), None);
    }

    #[test]
    fn products_come_back_in_order_of_first_appearance() {
        let text = "I'd look at Mem0 first, then Zep, and AllSource is also an option.";
        let found = named_products(text);
        let ids: Vec<&str> = found.iter().map(|&(id, _)| id).collect();
        assert_eq!(ids, vec!["mem0", "zep", "allsource"]);
    }

    #[test]
    fn a_text_naming_nobody_yields_nothing() {
        assert!(named_products("Just use whatever your framework ships with.").is_empty());
    }

    #[test]
    fn plain_files_count_as_an_answer() {
        let found = named_products("Honestly, a JSON file on disk is enough for most agents.");
        assert_eq!(found.first().map(|&(id, _)| id), Some("files"));
    }
}
