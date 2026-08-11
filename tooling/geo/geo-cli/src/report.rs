//! `geo report` — read the `geo.*` family back out of AllSource and print a
//! per-layer count table.
//!
//! The layer mapping lives here, not in `geo-core`: the emitter is
//! transport-thin and has no opinion about measurement layers. This is the
//! only place that groups event types into layers.

use std::collections::{BTreeMap, BTreeSet};

use allsource::{QueryClient, QueryEventsParams};
use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use geo_core::{EVENT_TYPE_PREFIX, GeoConfig, GeoEventType, IngestEnvelope, samples};

/// One measurement layer: a named group of event types and the slice that
/// produces them.
struct Layer {
    id: &'static str,
    name: &'static str,
    produced_by: &'static str,
    types: &'static [GeoEventType],
}

/// The layer map. Prompt numbers are the plan of record; if a slice lands
/// under a different number, fix it here and in
/// `docs/contracts/geo-events/README.md`.
const LAYERS: &[Layer] = &[
    Layer {
        id: "1",
        name: "Referral attribution",
        produced_by: "prompt 026",
        types: &[GeoEventType::ReferralObserved],
    },
    Layer {
        id: "2",
        name: "Crawl diagnostics",
        produced_by: "prompt 024",
        types: &[GeoEventType::CrawlObserved],
    },
    Layer {
        id: "3a",
        name: "Share of voice",
        produced_by: "prompt 025",
        types: &[GeoEventType::SovProbed],
    },
    Layer {
        id: "3b",
        name: "Interrogation",
        produced_by: "prompt 025",
        types: &[GeoEventType::InterrogationProbed],
    },
    Layer {
        id: "4",
        name: "Self-report",
        produced_by: "prompt 026",
        types: &[GeoEventType::SelfReportCaptured],
    },
    Layer {
        id: "5",
        name: "Experiments",
        produced_by: "prompt 027",
        types: &[
            GeoEventType::ExperimentStarted,
            GeoEventType::ExperimentScored,
        ],
    },
];

/// Events and distinct entities seen for one event type.
#[derive(Default)]
struct TypeTally {
    events: u64,
    entities: BTreeSet<String>,
}

/// Everything a report needs from the stream.
#[derive(Default)]
struct Tally {
    by_type: BTreeMap<&'static str, TypeTally>,
    unknown_types: BTreeMap<String, u64>,
    total_events: u64,
    truncated: bool,
}

impl Tally {
    fn record(&mut self, event_type: &str, entity_id: &str) {
        self.total_events += 1;
        if let Some(known) = GeoEventType::parse(event_type) {
            let slot = self.by_type.entry(known.as_str()).or_default();
            slot.events += 1;
            slot.entities.insert(entity_id.to_string());
        } else {
            // A `geo.*` type this build does not know about: report it rather
            // than silently dropping it, so a newer producer is visible.
            let seen = self
                .unknown_types
                .entry(event_type.to_string())
                .or_default();
            *seen += 1;
        }
    }

    fn events_for(&self, t: GeoEventType) -> u64 {
        self.by_type.get(t.as_str()).map_or(0, |s| s.events)
    }

    fn entities_for(&self, t: GeoEventType) -> u64 {
        self.by_type
            .get(t.as_str())
            .map_or(0, |s| s.entities.len() as u64)
    }
}

/// The window a report covers.
#[derive(Clone, Copy)]
pub struct Window {
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

impl Window {
    fn since_str(self) -> String {
        self.since.to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    fn until_str(self) -> String {
        self.until.to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}

/// Run a live report against the gateway.
pub async fn run_live(window: Window, max_events: u64, page_size: u32) -> Result<()> {
    let config = GeoConfig::from_env()?;
    let client = QueryClient::new(config.api_url(), config.api_key())
        .with_context(|| format!("could not build a gateway client for {}", config.api_url()))?;

    let params = QueryEventsParams::new()
        .event_type_prefix(EVENT_TYPE_PREFIX)
        .since(&window.since_str())
        .until(&window.until_str())
        .limit(page_size);

    let mut pages = client.query_events_paged(params);
    let mut tally = Tally::default();

    while let Some(page) = pages
        .next_page()
        .await
        .context("gateway query for geo.* events failed")?
    {
        for event in page {
            tally.record(&event.event_type, &event.entity_id);
        }
        if tally.total_events >= max_events {
            tally.truncated = true;
            break;
        }
    }

    println!(
        "GEO report — {} → {}",
        window.since_str(),
        window.until_str()
    );
    println!("source: {} (Control Plane gateway)", config.api_url());
    println!();
    print_table(&tally);
    Ok(())
}

/// Run a dry-run report: no gateway, no credential.
///
/// Prints the canonical example of every event type through the emitter's
/// dry-run path — the exact JSON a live emit would POST — then tallies those
/// same events. That makes this a self-contained smoke test of contract →
/// emitter → report with nothing to burn.
pub fn run_dry_run(window: Window) -> Result<()> {
    println!("GEO report — DRY RUN (no gateway call, no API key needed)");
    println!(
        "would query: GET /api/v1/events/query?event_type_prefix={EVENT_TYPE_PREFIX}&since={}&until={}",
        window.since_str(),
        window.until_str()
    );
    println!();
    println!("-- envelopes the emitter would POST ------------------------------");

    let events = samples::canonical_events();
    let mut tally = Tally::default();
    for event in &events {
        let envelope = IngestEnvelope::build(event)?;
        println!("{}", envelope.to_pretty_json()?);
        tally.record(&envelope.event_type, &envelope.entity_id);
    }

    println!();
    println!("-- tally over the sample above -----------------------------------");
    println!();
    print_table(&tally);
    Ok(())
}

fn print_table(tally: &Tally) {
    println!(
        "{:<5} {:<22} {:<28} {:>7} {:>9}  PRODUCED BY",
        "LAYER", "NAME", "EVENT TYPE", "EVENTS", "ENTITIES"
    );
    println!("{}", "-".repeat(92));

    for layer in LAYERS {
        for (i, &event_type) in layer.types.iter().enumerate() {
            let (id, name) = if i == 0 {
                (layer.id, layer.name)
            } else {
                ("", "")
            };
            println!(
                "{:<5} {:<22} {:<28} {:>7} {:>9}  {}",
                id,
                name,
                event_type.as_str(),
                tally.events_for(event_type),
                tally.entities_for(event_type),
                if i == 0 { layer.produced_by } else { "" },
            );
        }
    }

    println!("{}", "-".repeat(92));
    println!(
        "{:<5} {:<22} {:<28} {:>7} {:>9}",
        "", "TOTAL", "", tally.total_events, ""
    );

    if !tally.unknown_types.is_empty() {
        println!();
        println!("unrecognised geo.* types (a newer producer is writing to this stream):");
        for (name, count) in &tally.unknown_types {
            println!("  {name:<40} {count:>7}");
        }
    }

    if tally.truncated {
        println!();
        println!("NOTE: hit --max-events; counts above are a truncated prefix of the window.");
    }

    if tally.total_events == 0 {
        println!();
        println!(
            "No geo.* events in this window. Nothing has been instrumented yet — the layer \
             slices (prompts 024/025/026) are what start filling this table."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_type_belongs_to_exactly_one_layer() {
        for event_type in GeoEventType::ALL {
            let hits = LAYERS
                .iter()
                .filter(|l| l.types.contains(&event_type))
                .count();
            assert_eq!(hits, 1, "{event_type} is in {hits} layers, expected 1");
        }
    }

    #[test]
    fn the_layer_map_invents_no_event_types() {
        for layer in LAYERS {
            for t in layer.types {
                assert!(GeoEventType::ALL.contains(t), "{t} is not in the contract");
            }
        }
    }

    #[test]
    fn tally_counts_events_and_distinct_entities_separately() {
        let mut tally = Tally::default();
        // Same entity twice — a replay. Two events, one entity.
        tally.record("geo.crawl.observed", "geo:crawl:abc");
        tally.record("geo.crawl.observed", "geo:crawl:abc");
        tally.record("geo.crawl.observed", "geo:crawl:def");
        assert_eq!(tally.events_for(GeoEventType::CrawlObserved), 3);
        assert_eq!(tally.entities_for(GeoEventType::CrawlObserved), 2);
    }

    #[test]
    fn unknown_types_are_surfaced_not_dropped() {
        let mut tally = Tally::default();
        tally.record("geo.brand.new", "geo:brand:1");
        assert_eq!(tally.total_events, 1);
        assert_eq!(tally.unknown_types.get("geo.brand.new"), Some(&1));
    }
}
