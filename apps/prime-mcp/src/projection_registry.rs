//! **In-memory cache** of declarative `ProjectionDef`s keyed by `entity_type`.
//!
//! The source of truth is the Core event log — every successful
//! [`crate::tools::call_define_projection`] writes a
//! `prime.projection.defined` event via [`allsource_core::prime::Prime::define_projection`].
//! This module is the read-side cache that the MCP dispatch path queries
//! synchronously; it's hydrated from the event log at server startup by
//! [`hydrate`] and kept consistent with the log by [`upsert`] (which is
//! always called *after* the persistence write succeeds).
//!
//! Why this design:
//! - Definitions survive process restarts — that's the `AllSource` pitch
//! - Definitions are queryable, replayable, time-travellable like any
//!   other event in Core
//! - The cache layer keeps tool-call latency low (no per-call query)
//!
//! Concurrency: `RwLock<HashMap>` is overkill for the access pattern (MCP
//! tools are dispatched sequentially per session) but matches Rust's
//! idiomatic shared-state shape and lets multiple session futures read
//! definitions concurrently if dispatch ever becomes parallel.
//!
//! Single-tenant for now — matches the current prime-mcp architecture
//! where each process is one tenant. When hosted multi-tenant Prime ships,
//! either (a) one cache per tenant keyed by `tenant_id`, or (b) the cache
//! is folded into a per-tenant Prime instance. Either works because the
//! durable side is already tenant-scoped via Core's per-event `tenant_id`.

use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use allsource_core::prime::projections::ProjectionDef;

fn registry() -> &'static RwLock<HashMap<String, ProjectionDef>> {
    static CELL: OnceLock<RwLock<HashMap<String, ProjectionDef>>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register or replace a definition for an entity type.
///
/// Replacement is intentional — agents iterate on projection definitions,
/// and refusing to replace would make iteration awkward. The MCP tool
/// surface logs at WARN when a replacement happens (see `call_define_projection`).
///
/// Returns `true` if a previous definition was replaced, `false` if this is
/// the first definition for the `entity_type`.
pub fn upsert(def: ProjectionDef) -> bool {
    let mut guard = registry().write().expect("projection registry poisoned");
    guard.insert(def.entity_type.clone(), def).is_some()
}

/// Look up the definition for an entity type. Returns `None` if no definition
/// has been registered for the type.
pub fn get(entity_type: &str) -> Option<ProjectionDef> {
    let guard = registry().read().expect("projection registry poisoned");
    guard.get(entity_type).cloned()
}

/// List every registered definition. Used by the `prime_list_projections`
/// MCP tool so agents can introspect what's defined without keeping the
/// state in their conversation context.
pub fn list() -> Vec<ProjectionDef> {
    let guard = registry().read().expect("projection registry poisoned");
    guard.values().cloned().collect()
}

/// Bulk-load definitions into the cache. Called once at process startup
/// after Prime is initialized and has replayed its event log.
///
/// Replaces the cache contents wholesale — the input `defs` is treated
/// as the authoritative current state derived from the event log. If
/// the cache already had entries (e.g. a process running tests calls
/// hydrate twice), the second call wins.
pub fn hydrate(defs: Vec<ProjectionDef>) {
    let mut guard = registry().write().expect("projection registry poisoned");
    guard.clear();
    for def in defs {
        guard.insert(def.entity_type.clone(), def);
    }
}

/// Test-only: wipe the registry. Lives behind `#[cfg(test)]` so production
/// callers can't reach it.
#[cfg(test)]
pub fn clear_for_test() {
    let mut guard = registry().write().expect("projection registry poisoned");
    guard.clear();
}

/// Serializes tests that touch the process-global registry. Both this module's
/// tests and `tools.rs`'s projection tests mutate the same static cache, so
/// under `cargo test`'s parallel runner a `clear`/`upsert`/`list` in one test
/// would otherwise corrupt another's exact-count assertions. Hold the returned
/// guard for the whole test body.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::projections::MergePolicy;
    use std::collections::BTreeMap;

    fn def(entity_type: &str) -> ProjectionDef {
        let mut fields = BTreeMap::new();
        fields.insert("status".to_string(), MergePolicy::LastWrite);
        ProjectionDef {
            entity_type: entity_type.to_string(),
            field_policies: fields,
        }
    }

    #[test]
    fn upsert_returns_false_on_first_definition() {
        let _guard = test_guard();
        clear_for_test();
        assert!(!upsert(def("contact-unique-1")));
    }

    #[test]
    fn upsert_returns_true_on_replacement() {
        let _guard = test_guard();
        clear_for_test();
        upsert(def("contact-unique-2"));
        assert!(upsert(def("contact-unique-2")));
    }

    #[test]
    fn get_returns_registered_definition() {
        let _guard = test_guard();
        clear_for_test();
        upsert(def("contact-unique-3"));
        let fetched = get("contact-unique-3").expect("definition missing");
        assert_eq!(fetched.entity_type, "contact-unique-3");
        assert_eq!(
            fetched.field_policies.get("status"),
            Some(&MergePolicy::LastWrite)
        );
    }

    #[test]
    fn get_for_unknown_type_returns_none() {
        let _guard = test_guard();
        clear_for_test();
        assert!(get("does-not-exist").is_none());
    }

    #[test]
    fn list_contains_every_registered_definition() {
        let _guard = test_guard();
        clear_for_test();
        upsert(def("type-a"));
        upsert(def("type-b"));
        let listed = list();
        assert_eq!(listed.len(), 2);
        let types: Vec<_> = listed.iter().map(|d| d.entity_type.as_str()).collect();
        assert!(types.contains(&"type-a"));
        assert!(types.contains(&"type-b"));
    }

    #[test]
    fn hydrate_replaces_cache_with_provided_defs() {
        let _guard = test_guard();
        clear_for_test();
        upsert(def("stale-1"));
        upsert(def("stale-2"));
        // Simulates startup: Prime::load_projection_defs() returned [fresh-1]
        hydrate(vec![def("fresh-1")]);
        let listed = list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].entity_type, "fresh-1");
    }

    #[test]
    fn hydrate_with_empty_input_clears_the_cache() {
        let _guard = test_guard();
        clear_for_test();
        upsert(def("will-be-gone"));
        hydrate(vec![]);
        assert!(list().is_empty());
    }
}
