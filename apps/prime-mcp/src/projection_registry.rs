//! In-memory registry of declarative `ProjectionDef`s keyed by entity_type.
//!
//! Closes the MCP-facing half of neotoma-gaps bead t-cdd2 alongside
//! [`crate::tools::call_define_projection`] / `call_project_node` /
//! `call_node_provenance`.
//!
//! Scope of this v1:
//! - Single-tenant (matches the current prime-mcp single-tenant architecture)
//! - Definitions are in-memory only — survive the process lifetime, not a
//!   restart. Event-sourced persistence is the next step under the
//!   `prime.projection.defined` event type; not in this first cut so the
//!   blast radius stays small.
//! - Definitions are shared across all sessions inside the process — the
//!   single MCP server process IS the tenant, so this is correct given the
//!   architecture but worth re-checking once hosted multi-tenant Prime ships.
//!
//! Concurrency: `RwLock<HashMap>` is overkill for the access pattern (MCP
//! tools are dispatched sequentially per session) but matches Rust's
//! idiomatic shared-state shape and lets multiple session futures read
//! definitions concurrently if dispatch ever becomes parallel.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

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
/// the first definition for the entity_type.
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

/// Test-only: wipe the registry. Lives behind `#[cfg(test)]` so production
/// callers can't reach it.
#[cfg(test)]
pub fn clear_for_test() {
    let mut guard = registry().write().expect("projection registry poisoned");
    guard.clear();
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
        clear_for_test();
        assert!(!upsert(def("contact-unique-1")));
    }

    #[test]
    fn upsert_returns_true_on_replacement() {
        clear_for_test();
        upsert(def("contact-unique-2"));
        assert!(upsert(def("contact-unique-2")));
    }

    #[test]
    fn get_returns_registered_definition() {
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
        clear_for_test();
        assert!(get("does-not-exist").is_none());
    }

    #[test]
    fn list_contains_every_registered_definition() {
        clear_for_test();
        upsert(def("type-a"));
        upsert(def("type-b"));
        let listed = list();
        assert_eq!(listed.len(), 2);
        let types: Vec<_> = listed.iter().map(|d| d.entity_type.as_str()).collect();
        assert!(types.contains(&"type-a"));
        assert!(types.contains(&"type-b"));
    }
}
