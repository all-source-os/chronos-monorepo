//! Guards the rustdoc on the projection-state read paths against drifting back
//! out of sync with Core.
//!
//! Core ≥ 0.19.1 resolves `GET /api/v1/projections/:name/:entity_id/state` from
//! the registered projection first and falls back to the projection state cache
//! (see `apps/core/src/infrastructure/web/api.rs::get_projection_state` and its
//! `get_projection_state_falls_back_to_cache_when_unregistered` test). The SDK
//! docs claimed the opposite for three releases, which told adopters to build
//! fan-outs Core makes unnecessary — so the claim is asserted here instead of
//! only being fixed once.

/// Collapse a doc block into one whitespace-normalized line so assertions don't
/// depend on where rustfmt happens to wrap a sentence.
fn doc_block_before(source: &str, signature: &str) -> String {
    let idx = source
        .find(signature)
        .unwrap_or_else(|| panic!("signature not found in source: {signature}"));
    let mut lines: Vec<&str> = Vec::new();
    for line in source[..idx].lines().rev() {
        let trimmed = line.trim();
        if let Some(doc) = trimmed.strip_prefix("///") {
            lines.push(doc.trim());
        } else if trimmed.starts_with("#[") || trimmed.is_empty() {
            // attributes / blank separators are not doc text but don't end it
            if trimmed.is_empty() && !lines.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
    lines.reverse();
    lines
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Claims that contradict Core's cache fallback. Any of these in a projection
/// state read doc means the doc is stale again.
const STALE_CLAIMS: &[&str] = &[
    "requires the projection to be registered",
    "requires a projection to be registered",
    "is not readable through this endpoint",
    "it lives in a separate cache",
    "the GET path returns 404",
];

fn assert_describes_cache_fallback(what: &str, doc: &str) {
    for claim in STALE_CLAIMS {
        assert!(
            !doc.contains(claim),
            "{what} rustdoc still claims `{claim}`, but Core ≥ 0.19.1 falls back \
             to the projection state cache. Doc was: {doc}"
        );
    }
    assert!(
        doc.contains("0.19.1"),
        "{what} rustdoc must name the Core version floor (0.19.1) for the cache \
         fallback. Doc was: {doc}"
    );
    assert!(
        doc.contains("falls back") || doc.contains("fall back"),
        "{what} rustdoc must describe Core's resolution order (registered \
         projection first, state cache as fallback). Doc was: {doc}"
    );
}

#[test]
fn get_projection_state_doc_matches_core_resolution_order() {
    let source = include_str!("../src/projection_api.rs");
    let doc = doc_block_before(
        source,
        "pub async fn get_projection_state<T: DeserializeOwned>",
    );
    assert_describes_cache_fallback("CoreClient::get_projection_state", &doc);
}

#[test]
fn projection_handle_get_state_doc_matches_core_resolution_order() {
    let source = include_str!("../src/projection_worker.rs");
    let doc = doc_block_before(source, "pub async fn get_state<T: DeserializeOwned>");
    assert_describes_cache_fallback("ProjectionHandle::get_state", &doc);
}
