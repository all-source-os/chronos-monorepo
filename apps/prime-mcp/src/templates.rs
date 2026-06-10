//! Pre-built Prime entity-type templates.
//!
//! Closes the "schema-agnostic out of the box" gap in
//! `docs/articles/neotoma-comparison.md` §"Gap 4". Each template defines a
//! common entity type (person, contact, organization, task, decision,
//! transaction, meeting, document) with its property shape and the typical
//! relations it participates in. Agents call `prime_list_templates` to see
//! what's available, then `prime_load_template` to read a specific one and
//! use the shape when creating nodes via `prime_add_node`.
//!
//! Templates are **descriptive, not enforced** today — they're a paved-path
//! reference, not a write-time validator. The validator lives separately in
//! Core (see neotoma-gap #3, bead t-0795).
//!
//! Templates are embedded at compile time via `include_str!` so the binary
//! stays self-contained — no template files to ship alongside it.

use std::{collections::HashMap, sync::OnceLock};

use serde_json::Value;

/// All bundled templates, keyed by entity_type. Lazily initialised.
fn templates() -> &'static HashMap<&'static str, Value> {
    static CELL: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut m = HashMap::new();
        for (name, raw) in BUNDLED_TEMPLATES {
            let parsed: Value = serde_json::from_str(raw)
                .unwrap_or_else(|e| panic!("template {name} failed to parse at startup: {e}"));
            m.insert(*name, parsed);
        }
        m
    })
}

/// Raw JSON for every bundled template. The build fails if any of these
/// files are missing or unparseable (see the unit tests below).
const BUNDLED_TEMPLATES: &[(&str, &str)] = &[
    ("person", include_str!("../templates/person.json")),
    ("contact", include_str!("../templates/contact.json")),
    (
        "organization",
        include_str!("../templates/organization.json"),
    ),
    ("task", include_str!("../templates/task.json")),
    ("decision", include_str!("../templates/decision.json")),
    ("transaction", include_str!("../templates/transaction.json")),
    ("meeting", include_str!("../templates/meeting.json")),
    ("document", include_str!("../templates/document.json")),
];

/// Return a summary view of every template: entity_type + description.
/// Used by the `prime_list_templates` MCP tool so agents can decide which
/// one to fetch in full.
pub fn list_templates_summary() -> Value {
    let summaries: Vec<Value> = templates()
        .iter()
        .map(|(name, full)| {
            serde_json::json!({
                "entity_type": name,
                "description": full.get("description").cloned().unwrap_or(Value::Null),
            })
        })
        .collect();
    serde_json::json!({ "templates": summaries })
}

/// Load a single template by name. Returns `None` if unknown.
pub fn load_template(name: &str) -> Option<Value> {
    templates().get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every bundled template must parse as JSON at startup.
    #[test]
    fn all_templates_parse() {
        for (name, raw) in BUNDLED_TEMPLATES {
            let parsed: Result<Value, _> = serde_json::from_str(raw);
            assert!(
                parsed.is_ok(),
                "template `{name}` does not parse as JSON: {:?}",
                parsed.err()
            );
        }
    }

    /// Every template must declare `entity_type`, `description`, and a non-empty
    /// `properties` map. Catches shape regressions when a new template is added.
    #[test]
    fn templates_have_required_shape() {
        for (name, _raw) in BUNDLED_TEMPLATES {
            let t = load_template(name).expect("template missing from registry");
            assert_eq!(
                t.get("entity_type").and_then(Value::as_str),
                Some(*name),
                "template `{name}`: entity_type field must match filename"
            );
            assert!(
                t.get("description").and_then(Value::as_str).is_some(),
                "template `{name}`: missing description"
            );
            let props = t.get("properties").and_then(Value::as_object);
            assert!(
                matches!(props, Some(p) if !p.is_empty()),
                "template `{name}`: properties must be a non-empty object"
            );
            // suggested_relations is optional but, if present, must be an array.
            if let Some(rels) = t.get("suggested_relations") {
                assert!(
                    rels.is_array(),
                    "template `{name}`: suggested_relations must be an array"
                );
            }
        }
    }

    /// At least one template (`task`) must have a required property to prove
    /// the required/optional distinction is reaching the JSON.
    #[test]
    fn task_template_has_required_fields() {
        let task = load_template("task").expect("task template missing");
        let props = task.get("properties").and_then(Value::as_object).unwrap();
        let title = props.get("title").and_then(Value::as_object).unwrap();
        assert_eq!(title.get("required").and_then(Value::as_bool), Some(true));
    }

    /// `list_templates_summary` returns one entry per template.
    #[test]
    fn summary_includes_every_template() {
        let summary = list_templates_summary();
        let arr = summary
            .get("templates")
            .and_then(Value::as_array)
            .expect("templates array missing");
        assert_eq!(arr.len(), BUNDLED_TEMPLATES.len());
    }

    /// Unknown templates return `None`, not an empty value.
    #[test]
    fn unknown_template_returns_none() {
        assert!(load_template("nonexistent").is_none());
    }
}
