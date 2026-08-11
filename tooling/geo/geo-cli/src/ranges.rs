//! Loading the vendors' published crawler IP ranges.
//!
//! Verification is only as good as the range lists behind it, so this module
//! is loud about where each list came from and how old it is. Two rules:
//!
//! 1. **A fetch failure is never a rejection.** If a list cannot be loaded,
//!    every bot behind it comes back `unverifiable`, not `rejected` — see
//!    [`geo_core::Verdict`]. Reporting a network blip as a spoofing attempt
//!    would be worse than reporting nothing.
//! 2. **Caching is explicit.** `--ranges-dir` doubles as an offline fixture
//!    directory and as a cache, so a run can be replayed byte-for-byte with
//!    `--offline` — which is what makes the ingest's tests deterministic.

use std::{
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result};
use geo_core::bots::{self, RangeCatalog, range_cache_file_name};

/// Per-file fetch budget. The lists are a few KB; anything slower is a
/// problem we want to see rather than wait out.
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

/// What happened to one vendor range list.
#[derive(Debug, Clone)]
pub struct RangeSource {
    /// Where the list is published.
    pub url: String,
    /// Which bots it verifies.
    pub bots: Vec<&'static str>,
    /// How it was obtained, or why it was not.
    pub outcome: RangeOutcome,
}

/// How a range list was obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeOutcome {
    /// Read from `--ranges-dir`.
    Cached { prefixes: usize, created: String },
    /// Fetched over HTTPS (and written to `--ranges-dir` when one was given).
    Fetched { prefixes: usize, created: String },
    /// Not loaded. Every bot behind this URL becomes `unverifiable`.
    Unavailable { reason: String },
}

impl RangeOutcome {
    pub fn is_loaded(&self) -> bool {
        !matches!(self, Self::Unavailable { .. })
    }

    /// One-line description for the run summary.
    pub fn describe(&self) -> String {
        match self {
            Self::Cached { prefixes, created } => {
                format!("cached, {prefixes} prefixes, vendor creationTime {created}")
            }
            Self::Fetched { prefixes, created } => {
                format!("fetched, {prefixes} prefixes, vendor creationTime {created}")
            }
            Self::Unavailable { reason } => format!("UNAVAILABLE — {reason}"),
        }
    }
}

/// Load every published range list the taxonomy references.
///
/// `ranges_dir` is read first when present. `offline` forbids the network
/// entirely, so a run against a fixture directory is fully hermetic.
pub async fn load(ranges_dir: Option<&Path>, offline: bool) -> Result<(RangeCatalog, Vec<RangeSource>)> {
    let mut catalog = RangeCatalog::new();
    let mut sources = Vec::new();

    let client = if offline {
        None
    } else {
        Some(
            reqwest::Client::builder()
                .timeout(FETCH_TIMEOUT)
                .user_agent(concat!("allsource-geo/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("could not build an HTTP client for vendor range lists")?,
        )
    };

    for (url, bot_ids) in bots::range_sources() {
        let cache_path = ranges_dir.map(|dir| dir.join(range_cache_file_name(url)));
        let outcome =
            load_one(&mut catalog, url, cache_path.as_deref(), client.as_ref(), offline).await;
        sources.push(RangeSource {
            url: url.to_string(),
            bots: bot_ids,
            outcome,
        });
    }

    Ok((catalog, sources))
}

async fn load_one(
    catalog: &mut RangeCatalog,
    url: &str,
    cache_path: Option<&Path>,
    client: Option<&reqwest::Client>,
    offline: bool,
) -> RangeOutcome {
    if let Some(path) = cache_path
        && path.is_file()
    {
        match std::fs::read_to_string(path) {
            Ok(body) => match catalog.insert_json(url, &body) {
                Ok(prefixes) => {
                    return RangeOutcome::Cached {
                        prefixes,
                        created: catalog.creation_time(url).unwrap_or("unknown").to_string(),
                    };
                }
                Err(e) => {
                    return RangeOutcome::Unavailable {
                        reason: format!("cached {} is unusable: {e}", path.display()),
                    };
                }
            },
            Err(e) => {
                return RangeOutcome::Unavailable {
                    reason: format!("cached {} unreadable: {e}", path.display()),
                };
            }
        }
    }

    let Some(client) = client else {
        return RangeOutcome::Unavailable {
            reason: if offline {
                format!("--offline and no cached copy in --ranges-dir ({url})")
            } else {
                "no HTTP client".to_string()
            },
        };
    };

    let body = match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(body) => body,
            Err(e) => {
                return RangeOutcome::Unavailable {
                    reason: format!("body read failed: {e}"),
                };
            }
        },
        Ok(resp) => {
            return RangeOutcome::Unavailable {
                reason: format!("HTTP {}", resp.status()),
            };
        }
        Err(e) => {
            return RangeOutcome::Unavailable {
                reason: format!("request failed: {e}"),
            };
        }
    };

    match catalog.insert_json(url, &body) {
        Ok(prefixes) => {
            if let Some(path) = cache_path {
                write_cache(path, &body);
            }
            RangeOutcome::Fetched {
                prefixes,
                created: catalog.creation_time(url).unwrap_or("unknown").to_string(),
            }
        }
        Err(e) => RangeOutcome::Unavailable {
            reason: format!("response is not a prefix list: {e}"),
        },
    }
}

/// Best-effort cache write. A cache we could not write is a slower next run,
/// not a failed one.
fn write_cache(path: &Path, body: &str) {
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let _ = std::fs::write(path, body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use geo_core::bots::{by_id, ipv4};

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("geo-ranges-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[tokio::test]
    async fn offline_with_no_cache_leaves_everything_unverifiable_never_rejected() {
        let (catalog, sources) = load(None, true).await.expect("load");
        assert!(!sources.is_empty());
        assert!(sources.iter().all(|s| !s.outcome.is_loaded()));

        // The critical property: an unloaded catalog must not manufacture
        // rejections, or a network outage would read as a site-wide spoof.
        let gptbot = by_id("gptbot").expect("gptbot is in the taxonomy");
        assert_eq!(
            catalog.verify(gptbot, Some(ipv4(132, 196, 86, 9))),
            geo_core::Verdict::UnverifiableRangesUnavailable
        );
    }

    #[tokio::test]
    async fn a_cached_fixture_is_used_without_touching_the_network() {
        let dir = tmp_dir("cached");
        let gptbot = by_id("gptbot").expect("gptbot is in the taxonomy");
        let url = gptbot
            .verification
            .ranges_url()
            .expect("gptbot ranges are published");
        std::fs::write(
            dir.join(range_cache_file_name(url)),
            r#"{"creationTime":"2025-10-30T11:00:00.000000","prefixes":[{"ipv4Prefix":"132.196.86.0/24"}]}"#,
        )
        .expect("fixture write");

        let (catalog, sources) = load(Some(&dir), true).await.expect("load");
        let loaded: Vec<_> = sources.iter().filter(|s| s.outcome.is_loaded()).collect();
        assert_eq!(loaded.len(), 1, "{sources:#?}");
        assert!(matches!(
            loaded[0].outcome,
            RangeOutcome::Cached { prefixes: 1, .. }
        ));
        assert_eq!(
            catalog.verify(gptbot, Some(ipv4(132, 196, 86, 9))),
            geo_core::Verdict::Verified
        );
    }

    #[tokio::test]
    async fn a_corrupt_cache_file_is_unavailable_not_a_panic() {
        let dir = tmp_dir("corrupt");
        let url = by_id("gptbot")
            .expect("gptbot is in the taxonomy")
            .verification
            .ranges_url()
            .expect("published");
        std::fs::write(dir.join(range_cache_file_name(url)), "not json").expect("fixture write");

        let (_, sources) = load(Some(&dir), true).await.expect("load");
        let entry = sources.iter().find(|s| s.url == url).expect("source");
        assert!(matches!(entry.outcome, RangeOutcome::Unavailable { .. }));
    }

}
