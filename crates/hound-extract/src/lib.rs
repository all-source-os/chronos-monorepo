//! Prime Hound — code extractor (Phase 1).
//!
//! Walks a source tree and turns it into a language-agnostic intermediate
//! representation of graph **symbols** (definitions) and **references** (calls)
//! using [Tree-sitter]. Everything here is pure and on-device: no LLM, no
//! network, no AllSource dependency — the caller (`apps/prime-mcp`) is what maps
//! this IR onto Prime `add_node` / `add_edge` events.
//!
//! Phase 1 supports **Rust** only. The shape (`SymbolKind`, `Reference`,
//! per-file [`FileGraph`]) is deliberately language-neutral so adding a grammar
//! is a matter of a new parser + node-kind mapping, not a new data model.

use std::path::Path;

use anyhow::Result;
use ignore::WalkBuilder;
use tree_sitter::{Node, Parser};

/// A defined entity in the source — becomes a Prime node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Type,
    Trait,
    Module,
}

impl SymbolKind {
    /// The Prime `node_type` string for this symbol.
    #[must_use]
    pub fn as_node_type(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Type => "type",
            SymbolKind::Trait => "trait",
            SymbolKind::Module => "module",
        }
    }
}

/// A definition found in a file (function, type, trait, module).
#[derive(Debug, Clone)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: String,
    /// 1-based line number of the definition.
    pub line: usize,
}

/// The kind of a reference between symbols. Only calls in Phase 1; `use`/import
/// edges are a deliberate follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Call,
}

/// A reference from within a file to a named symbol — becomes a Prime edge once
/// the caller resolves `name` against the extracted symbol set.
#[derive(Debug, Clone)]
pub struct Reference {
    pub kind: RefKind,
    /// The referenced name (e.g. the called function's identifier).
    pub name: String,
    /// The enclosing function this reference appears in, if any. `None` means
    /// the reference is at module/const scope.
    pub from_fn: Option<String>,
    /// 1-based line number of the reference site.
    pub line: usize,
}

/// All symbols and references extracted from a single file.
#[derive(Debug, Clone, Default)]
pub struct FileGraph {
    /// Path relative to the scanned root, using `/` separators.
    pub path: String,
    pub language: String,
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
}

/// The result of scanning a directory tree.
#[derive(Debug, Default)]
pub struct ExtractResult {
    pub files: Vec<FileGraph>,
    /// Source files matched by extension and handed to the parser.
    pub files_scanned: usize,
    /// Files that parsed (a tree was produced).
    pub files_parsed: usize,
    /// Files matched but skipped (unreadable).
    pub files_skipped: usize,
}

fn line_of(node: Node) -> usize {
    node.start_position().row + 1
}

fn field_text(node: Node, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

/// Resolve the callee name of a `call_expression`'s `function` child across the
/// common Rust call shapes: bare `foo()`, path `a::b::foo()`, method
/// `x.foo()`, and turbofish `foo::<T>()`.
fn callee_name(node: Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => node.utf8_text(src).ok().map(str::to_string),
        "scoped_identifier" => field_text(node, "name", src),
        "field_expression" => field_text(node, "field", src),
        "generic_function" => node
            .child_by_field_name("function")
            .and_then(|f| callee_name(f, src)),
        _ => None,
    }
}

fn visit(node: Node, src: &[u8], enclosing: Option<&str>, fg: &mut FileGraph) {
    // `next_enclosing` is what children see: entering a function sets it to that
    // function's name so calls in the body attribute to the right caller.
    let mut next_enclosing: Option<String> = enclosing.map(str::to_string);

    match node.kind() {
        "function_item" => {
            if let Some(name) = field_text(node, "name", src) {
                fg.symbols.push(Symbol {
                    kind: SymbolKind::Function,
                    name: name.clone(),
                    line: line_of(node),
                });
                next_enclosing = Some(name);
            }
        }
        "struct_item" | "enum_item" | "union_item" | "type_item" => {
            if let Some(name) = field_text(node, "name", src) {
                fg.symbols.push(Symbol {
                    kind: SymbolKind::Type,
                    name,
                    line: line_of(node),
                });
            }
        }
        "trait_item" => {
            if let Some(name) = field_text(node, "name", src) {
                fg.symbols.push(Symbol {
                    kind: SymbolKind::Trait,
                    name,
                    line: line_of(node),
                });
            }
        }
        "mod_item" => {
            if let Some(name) = field_text(node, "name", src) {
                fg.symbols.push(Symbol {
                    kind: SymbolKind::Module,
                    name,
                    line: line_of(node),
                });
            }
        }
        "call_expression" => {
            if let Some(func) = node.child_by_field_name("function") {
                if let Some(name) = callee_name(func, src) {
                    fg.references.push(Reference {
                        kind: RefKind::Call,
                        name,
                        from_fn: enclosing.map(str::to_string),
                        line: line_of(node),
                    });
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, src, next_enclosing.as_deref(), fg);
    }
}

/// Extract symbols and references from a single Rust source string. Infallible:
/// a grammar load or parse failure yields an empty [`FileGraph`] rather than an
/// error, so one unparseable file never aborts a whole-tree scan.
#[must_use]
pub fn extract_rust_source(src: &str, path: &str) -> FileGraph {
    let mut fg = FileGraph {
        path: path.to_string(),
        language: "rust".to_string(),
        symbols: Vec::new(),
        references: Vec::new(),
    };
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return fg;
    }
    let Some(tree) = parser.parse(src.as_bytes(), None) else {
        return fg;
    };
    visit(tree.root_node(), src.as_bytes(), None, &mut fg);
    fg
}

/// Walk `root` (respecting `.gitignore` and skipping hidden dirs like `.git`),
/// extract every `.rs` file, and collect the per-file graphs.
pub fn extract(root: &Path) -> Result<ExtractResult> {
    let mut result = ExtractResult::default();
    for entry in WalkBuilder::new(root).build() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        result.files_scanned += 1;
        let Ok(src) = std::fs::read_to_string(path) else {
            result.files_skipped += 1;
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        result.files.push(extract_rust_source(&src, &rel));
        result.files_parsed += 1;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
mod a {
    pub fn foo() { bar(); }
    fn bar() {}
    struct S;
}

trait Greet { fn hello(&self); }

fn main() {
    a::foo();
    let _ = compute();
}

fn compute() -> i32 { 42 }
"#;

    #[test]
    fn extracts_symbols() {
        let fg = extract_rust_source(SAMPLE, "sample.rs");
        let syms: Vec<(SymbolKind, &str)> =
            fg.symbols.iter().map(|s| (s.kind, s.name.as_str())).collect();
        assert!(syms.contains(&(SymbolKind::Function, "foo")));
        assert!(syms.contains(&(SymbolKind::Function, "bar")));
        assert!(syms.contains(&(SymbolKind::Function, "main")));
        assert!(syms.contains(&(SymbolKind::Function, "compute")));
        assert!(syms.contains(&(SymbolKind::Type, "S")));
        assert!(syms.contains(&(SymbolKind::Module, "a")));
        assert!(syms.contains(&(SymbolKind::Trait, "Greet")));
    }

    #[test]
    fn attributes_calls_to_enclosing_function() {
        let fg = extract_rust_source(SAMPLE, "sample.rs");

        // bar() is called from inside foo()
        let bar = fg.references.iter().find(|r| r.name == "bar").expect("bar call");
        assert_eq!(bar.from_fn.as_deref(), Some("foo"));

        // a::foo() resolves to callee "foo", called from main()
        let foo = fg.references.iter().find(|r| r.name == "foo").expect("foo call");
        assert_eq!(foo.from_fn.as_deref(), Some("main"));

        // compute() called from main()
        let compute = fg
            .references
            .iter()
            .find(|r| r.name == "compute")
            .expect("compute call");
        assert_eq!(compute.from_fn.as_deref(), Some("main"));
    }

    #[test]
    fn empty_source_is_empty_graph() {
        let fg = extract_rust_source("", "empty.rs");
        assert!(fg.symbols.is_empty());
        assert!(fg.references.is_empty());
        assert_eq!(fg.language, "rust");
    }
}
