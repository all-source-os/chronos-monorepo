//! Minimal TOON (Token-Oriented Object Notation) encoder.
//!
//! TOON is a compact, indentation-based serialization that drops the
//! structural punctuation JSON repeats on every element. For uniform arrays
//! of flat objects it emits a single header row plus CSV-style data rows,
//! cutting token count ~40-60% versus pretty-printed JSON.
//!
//! This encoder is one-way (serialize only) and targets the shapes Prime
//! tool results actually produce: a root object whose values are scalars,
//! nested objects, or arrays of objects. It is spec-aligned with
//! <https://github.com/toon-format/toon> for those shapes.
//!
//! Encoding rules:
//! - Object → `key: value` lines, nested objects indented two spaces.
//! - Array of scalars → `key[N]: a,b,c` (inline CSV).
//! - Array of objects that share an identical, all-scalar key set → tabular
//!   `key[N]{f1,f2}:` header followed by one CSV row per element.
//! - Any other array (nested or non-uniform) → list form, one `- ` item each.

use serde_json::Value;
use std::fmt::Write;

/// Encode a JSON value as TOON. The root is expected to be an object;
/// a non-object root is emitted under a synthetic `value` key.
#[must_use]
pub fn encode(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                write_field(&mut out, 0, k, v);
            }
        }
        other => write_field(&mut out, 0, "value", other),
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

fn indent_str(level: usize) -> String {
    "  ".repeat(level)
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Object(_) | Value::Array(_))
}

/// Write a `key: ...` field, recursing into nested objects/arrays.
fn write_field(out: &mut String, level: usize, key: &str, value: &Value) {
    let pad = indent_str(level);
    match value {
        v if is_scalar(v) => {
            let _ = writeln!(out, "{pad}{}: {}", fmt_key(key), fmt_value(v));
        }
        Value::Object(map) => {
            let _ = writeln!(out, "{pad}{}:", fmt_key(key));
            for (k, v) in map {
                write_field(out, level + 1, k, v);
            }
        }
        Value::Array(arr) => write_array(out, level, key, arr),
        _ => unreachable!("scalar handled by guard above"),
    }
}

fn write_array(out: &mut String, level: usize, key: &str, arr: &[Value]) {
    let pad = indent_str(level);
    let n = arr.len();

    if arr.is_empty() {
        let _ = writeln!(out, "{pad}{}[0]:", fmt_key(key));
        return;
    }

    // All scalars → inline CSV row.
    if arr.iter().all(is_scalar) {
        let row: Vec<String> = arr.iter().map(fmt_value).collect();
        let _ = writeln!(out, "{pad}{}[{n}]: {}", fmt_key(key), row.join(","));
        return;
    }

    // Uniform objects with an all-scalar key set → tabular block.
    if let Some(fields) = tabular_fields(arr) {
        let header: Vec<String> = fields.iter().map(|f| fmt_key(f)).collect();
        let _ = writeln!(out, "{pad}{}[{n}]{{{}}}:", fmt_key(key), header.join(","));
        let rowpad = indent_str(level + 1);
        for el in arr {
            let obj = el.as_object().expect("tabular_fields verified objects");
            let row: Vec<String> = fields
                .iter()
                .map(|f| fmt_value(obj.get(*f).unwrap_or(&Value::Null)))
                .collect();
            let _ = writeln!(out, "{rowpad}{}", row.join(","));
        }
        return;
    }

    // Mixed / nested → list form.
    let _ = writeln!(out, "{pad}{}[{n}]:", fmt_key(key));
    for el in arr {
        write_list_item(out, level + 1, el);
    }
}

/// If every element is a non-empty object sharing the same key set, and
/// every value is a scalar, return the shared field order. Else `None`.
fn tabular_fields(arr: &[Value]) -> Option<Vec<&str>> {
    let first = arr.first()?.as_object()?;
    if first.is_empty() {
        return None;
    }
    let fields: Vec<&str> = first.keys().map(String::as_str).collect();
    for el in arr {
        let obj = el.as_object()?;
        if obj.len() != fields.len() {
            return None;
        }
        for f in &fields {
            match obj.get(*f) {
                Some(v) if is_scalar(v) => {}
                _ => return None,
            }
        }
    }
    Some(fields)
}

fn write_list_item(out: &mut String, level: usize, value: &Value) {
    let pad = indent_str(level);
    if is_scalar(value) {
        let _ = writeln!(out, "{pad}- {}", fmt_value(value));
        return;
    }
    let mut buf = String::new();
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                let _ = writeln!(out, "{pad}-");
                return;
            }
            for (k, v) in map {
                write_field(&mut buf, level + 1, k, v);
            }
        }
        Value::Array(arr) => write_array(&mut buf, level + 1, "items", arr),
        _ => unreachable!("scalar handled above"),
    }
    // Splice "- " over the leading indent of the child's first line so the
    // dash aligns with the indentation level and content stays aligned.
    let child_pad = indent_str(level + 1);
    if let Some(rest) = buf.strip_prefix(child_pad.as_str()) {
        out.push_str(&pad);
        out.push_str("- ");
        out.push_str(rest);
    } else {
        out.push_str(&buf);
    }
}

/// Format an object key, quoting only when it contains structural characters.
fn fmt_key(k: &str) -> String {
    if k.is_empty() || k.contains([',', ':', '{', '}', '[', ']', '"', ' ', '\n', '\t']) {
        serde_json::to_string(k).unwrap_or_else(|_| format!("{k:?}"))
    } else {
        k.to_string()
    }
}

/// Format a scalar value. Strings are quoted only when leaving them bare
/// would be ambiguous (structural chars, surrounding whitespace, or a
/// spelling that would otherwise parse as a number / bool / null).
fn fmt_value(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if string_needs_quote(s) {
                serde_json::to_string(s).unwrap_or_else(|_| format!("{s:?}"))
            } else {
                s.clone()
            }
        }
        // Containers should not reach here; compact-JSON fallback is safe.
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn string_needs_quote(s: &str) -> bool {
    // A colon / bracket / brace is unambiguous in value position (the value
    // runs to end of line), so it is *not* quoted — important because Prime
    // entity IDs like `node:person:1` are pervasive. Only the CSV-cell
    // delimiter (comma), quotes, whitespace edges, control characters, and
    // literal collisions with number / bool / null force quoting.
    s.is_empty()
        || s != s.trim()
        || s.contains([',', '"', '\n', '\r', '\t', '\\'])
        || matches!(s, "true" | "false" | "null")
        || s.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::encode;
    use serde_json::json;

    #[test]
    fn flat_object() {
        let v = json!({ "deleted": true, "id": "node:person:1" });
        assert_eq!(encode(&v), "deleted: true\nid: node:person:1");
    }

    #[test]
    fn tabular_array_of_uniform_objects() {
        let v = json!({
            "vectors": [
                { "id": "v1", "score": 0.92, "text": "alpha" },
                { "id": "v2", "score": 0.81, "text": "beta" },
            ]
        });
        assert_eq!(
            encode(&v),
            "vectors[2]{id,score,text}:\n  v1,0.92,alpha\n  v2,0.81,beta"
        );
    }

    #[test]
    fn scalar_array_inline() {
        let v = json!({ "tags": ["a", "b", "c"] });
        assert_eq!(encode(&v), "tags[3]: a,b,c");
    }

    #[test]
    fn empty_array() {
        assert_eq!(encode(&json!({ "nodes": [] })), "nodes[0]:");
    }

    #[test]
    fn non_uniform_array_falls_back_to_list() {
        // `properties` is a nested object, so rows are not tabular.
        let v = json!({
            "nodes": [
                { "id": "n1", "properties": { "name": "Alice" } },
                { "id": "n2", "properties": { "name": "Bob" } },
            ]
        });
        assert_eq!(
            encode(&v),
            "nodes[2]:\n  - id: n1\n    properties:\n      name: Alice\n  \
             - id: n2\n    properties:\n      name: Bob"
        );
    }

    #[test]
    fn ambiguous_strings_are_quoted() {
        let v = json!({ "a": "123", "b": "true", "c": "x,y", "d": "plain" });
        assert_eq!(encode(&v), "a: \"123\"\nb: \"true\"\nc: \"x,y\"\nd: plain");
    }

    #[test]
    fn nested_object() {
        // serde_json maps are ordered (no `preserve_order` feature), so the
        // encoder emits keys alphabetically — deterministic regardless of
        // json! literal order.
        let v = json!({ "stats": { "total_nodes": 3, "total_edges": 1 } });
        assert_eq!(encode(&v), "stats:\n  total_edges: 1\n  total_nodes: 3");
    }

    #[test]
    fn entity_ids_with_colons_are_not_quoted() {
        let v = json!({ "node_id": "node:person:42" });
        assert_eq!(encode(&v), "node_id: node:person:42");
    }
}
