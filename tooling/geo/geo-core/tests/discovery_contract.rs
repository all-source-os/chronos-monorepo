//! The anti-drift gate for the layer-4 discovery-source vocabulary.
//!
//! Three sides speak this vocabulary — Rust (the report), TypeScript (the web
//! form and its route handler) and Go (`/api/v1/onboard/start` validation).
//! They cannot import each other (the monorepo isolation rule, and they are
//! three languages), so they agree through one committed file:
//! `docs/contracts/geo-events/discovery-sources.json`, generated from
//! `geo_core::discovery` here and asserted against by the other two.
//!
//! Drift here has a specific, quiet cost: if one side writes `"ChatGPT"` and
//! another writes `"chatgpt"`, the report shows two channels where there is
//! one and the AI-sourced share — the headline number of the whole layer — is
//! silently halved.

use std::{fs, path::PathBuf};

use geo_core::{
    DISCOVERY_SOURCES, GeoEvent, GeoEventType, capture_path, discovery_source, samples,
};

fn contract_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/geo-events/discovery-sources.json")
}

/// The exact JSON the contract file holds. One renderer, used by both the
/// generator and the check, so they cannot disagree about formatting.
fn render() -> String {
    let doc = serde_json::json!({
        "$comment": "GENERATED from tooling/geo/geo-core/src/discovery.rs. \
                     Do not edit by hand — run: cargo test -p geo-core -- --ignored regenerate_discovery_contract",
        "capture_paths": capture_path::ALL,
        "sources": DISCOVERY_SOURCES,
    });
    let mut json = serde_json::to_string_pretty(&doc).expect("contract renders");
    json.push('\n');
    json
}

/// Rewrites `docs/contracts/geo-events/discovery-sources.json`.
///
/// ```text
/// cargo test -p geo-core -- --ignored regenerate_discovery_contract
/// ```
#[test]
#[ignore = "generator, not a check — run explicitly after changing the vocabulary"]
fn regenerate_discovery_contract() {
    let path = contract_path();
    fs::write(&path, render()).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    println!("wrote {}", path.display());
}

/// The canonical example is what the contract doc shows and what every reader
/// of this contract copies. If it carried a display label (`"ChatGPT"`) where
/// the wire carries an id (`"chatgpt"`), the doc would teach the drift.
#[test]
fn the_canonical_selfreport_sample_speaks_the_vocabulary() {
    let GeoEvent::SelfReport(p) = samples::sample(GeoEventType::SelfReportCaptured) else {
        panic!("sample() returned the wrong variant");
    };
    assert!(
        discovery_source(&p.surface).is_some(),
        "sample surface {:?} is not a discovery-source id",
        p.surface
    );
    assert!(
        capture_path::ALL.contains(&p.source.as_str()),
        "sample source {:?} is not a capture path",
        p.source
    );
}

#[test]
fn the_committed_vocabulary_matches_geo_core() {
    let path = contract_path();
    let committed = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing {}: {e}", path.display()));
    assert_eq!(
        committed,
        render(),
        "docs/contracts/geo-events/discovery-sources.json has drifted from \
         geo_core::discovery. Regenerate it:\n  \
         cargo test -p geo-core -- --ignored regenerate_discovery_contract\n\
         then update apps/web/src/lib/geo-discovery-sources.ts and \
         apps/control-plane/geo_selfreport.go to match."
    );
}
