//! The anti-drift gate between `docs/contracts/geo-events/` and these types.
//!
//! This repo has been burned by tests that asserted against hand-written
//! fixtures honouring a contract the server did not implement (gh#250). So
//! nothing here is hand-written twice: the assertions compare the *real*
//! serialisation produced by the emitter against the *committed* example files
//! that the contract doc links to. Change one without the other and this
//! fails.

use std::{fs, path::PathBuf};

use geo_core::{GeoEvent, GeoEventType, IngestEnvelope, samples};

/// `docs/contracts/geo-events/examples/`, resolved from this crate.
fn examples_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/geo-events/examples");
    assert!(
        dir.is_dir(),
        "{} must exist — the contract doc is the authority for the geo.* family",
        dir.display()
    );
    dir
}

fn committed_example(event_type: GeoEventType) -> String {
    let path = examples_dir().join(samples::example_file_name(event_type));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing contract example {}: {e}", path.display()))
}

/// Round-trip a payload through JSON and back, asserting the value survives.
fn assert_payload_round_trips(event: &GeoEvent) {
    let json = event.payload().expect("payload serialises");
    let text = serde_json::to_string(&json).expect("payload renders");

    macro_rules! round_trip {
        ($variant:path, $ty:ty, $inner:expr) => {{
            let back: $ty = serde_json::from_str(&text).unwrap_or_else(|e| {
                panic!(
                    "{} payload did not deserialise: {e}\n{text}",
                    event.event_type()
                )
            });
            assert_eq!(
                &back,
                $inner,
                "{} payload changed across a round trip",
                event.event_type()
            );
            // And the re-serialised form is byte-identical, so a replay
            // through this type cannot rewrite the stored shape.
            let again = $variant(back);
            assert_eq!(again.payload().expect("re-serialises"), json);
        }};
    }

    match event {
        GeoEvent::Referral(p) => round_trip!(GeoEvent::Referral, geo_core::ReferralObserved, p),
        GeoEvent::Crawl(p) => round_trip!(GeoEvent::Crawl, geo_core::CrawlObserved, p),
        GeoEvent::Sov(p) => round_trip!(GeoEvent::Sov, geo_core::SovProbed, p),
        GeoEvent::Interrogation(p) => {
            round_trip!(GeoEvent::Interrogation, geo_core::InterrogationProbed, p);
        }
        GeoEvent::SelfReport(p) => {
            round_trip!(GeoEvent::SelfReport, geo_core::SelfReportCaptured, p);
        }
        GeoEvent::ExperimentStarted(p) => {
            round_trip!(GeoEvent::ExperimentStarted, geo_core::ExperimentStarted, p);
        }
        GeoEvent::ExperimentScored(p) => {
            round_trip!(GeoEvent::ExperimentScored, geo_core::ExperimentScored, p);
        }
    }
}

/// Rewrites `docs/contracts/geo-events/examples/` from the emitter.
///
/// Not a check — a generator, so the committed examples are always the real
/// serialisation and never a hand-typed approximation of it. Run it after
/// changing a payload type:
///
/// ```text
/// cargo test -p geo-core -- --ignored regenerate_contract_examples
/// ```
#[test]
#[ignore = "generator, not a check — run explicitly after changing a payload type"]
fn regenerate_contract_examples() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/geo-events/examples");
    fs::create_dir_all(&dir).expect("examples dir is creatable");

    for event in samples::canonical_events() {
        let path = dir.join(samples::example_file_name(event.event_type()));
        let mut json = IngestEnvelope::build(&event)
            .expect("envelope builds")
            .to_pretty_json()
            .expect("envelope renders");
        json.push('\n');
        fs::write(&path, json).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        println!("wrote {}", path.display());
    }
}

#[test]
fn every_payload_round_trips_through_json() {
    for event in samples::canonical_events() {
        assert_payload_round_trips(&event);
    }
}

#[test]
fn every_event_type_has_a_committed_example() {
    for event_type in GeoEventType::ALL {
        let text = committed_example(event_type);
        assert!(
            !text.trim().is_empty(),
            "{event_type} example file is empty"
        );
    }
}

#[test]
fn dry_run_output_matches_the_committed_contract_examples() {
    for event in samples::canonical_events() {
        let event_type = event.event_type();
        let rendered = IngestEnvelope::build(&event)
            .expect("envelope builds")
            .to_pretty_json()
            .expect("envelope renders");
        let committed = committed_example(event_type);

        assert_eq!(
            rendered.trim_end(),
            committed.trim_end(),
            "docs/contracts/geo-events/examples/{} has drifted from geo-core.\n\
             Regenerate it from the emitter's dry-run output rather than editing by hand.",
            samples::example_file_name(event_type)
        );
    }
}

#[test]
fn committed_examples_deserialise_as_ingest_envelopes() {
    for event_type in GeoEventType::ALL {
        let text = committed_example(event_type);
        let value: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("{event_type} example: {e}"));
        let obj = value.as_object().expect("example is an object");

        assert_eq!(
            obj.get("event_type").and_then(serde_json::Value::as_str),
            Some(event_type.as_str())
        );
        for required in ["event_type", "entity_id", "payload", "metadata"] {
            assert!(
                obj.contains_key(required),
                "{event_type} example is missing {required}"
            );
        }
        assert!(
            obj["entity_id"]
                .as_str()
                .expect("entity_id is a string")
                .starts_with("geo:"),
            "{event_type} example entity_id is outside the geo: namespace"
        );
        assert!(
            obj["metadata"].get("idempotency_key").is_some(),
            "{event_type} example is missing metadata.idempotency_key"
        );
    }
}

#[test]
fn the_examples_directory_holds_exactly_the_contract() {
    let expected: Vec<String> = GeoEventType::ALL
        .into_iter()
        .map(samples::example_file_name)
        .collect();

    let mut found: Vec<String> = fs::read_dir(examples_dir())
        .expect("examples dir is readable")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect();
    found.sort();

    let mut expected_sorted = expected;
    expected_sorted.sort();

    assert_eq!(
        found, expected_sorted,
        "examples/ and GeoEventType::ALL disagree — an event was added or removed on one side only"
    );
}
