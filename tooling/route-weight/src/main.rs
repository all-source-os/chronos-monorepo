//! Measures how much client JavaScript a prerendered Next.js route actually pulls in.
//!
//! Why this exists: Next 16 on Turbopack no longer prints the "First Load JS" column
//! that the webpack builder did, so there is no build-output number to optimise
//! against. This reads the prerendered HTML for a route, collects every
//! `/_next/static/**.js` it references, and sums those files on disk.
//!
//! The number is deterministic for a given build, which is the property that makes
//! it usable as an autoresearch scalar: no noise band, no repeated sampling.
//!
//! It counts each chunk ONCE even when the HTML references it more than once — the
//! browser does the same. It is raw bytes, not gzip: raw is what the bundler
//! controls and what moves when you remove a dependency.
//!
//! Usage:
//!   route-weight <next-dir> <route-html-path>...
//!   route-weight apps/web/.next dashboard.html dashboard/billing.html

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Pulls every `/_next/static/...js` URL out of a blob of HTML.
///
/// Deliberately a hand-rolled scan rather than a regex dependency: the pattern is
/// fixed, and a measurement tool that drifts because a dep bumped is worse than a
/// few lines of matching.
fn extract_static_js(html: &str) -> BTreeSet<String> {
    const PREFIX: &str = "/_next/static/";
    let mut found = BTreeSet::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(rel) = html[i..].find(PREFIX) {
        let start = i + rel;
        // Walk to the closing quote of the attribute value.
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'"' && bytes[end] != b'\'' {
            end += 1;
        }
        let url = &html[start..end];
        if url.ends_with(".js") {
            found.insert(url.to_string());
        }
        i = end.max(start + 1);
    }
    found
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: route-weight <next-dir> <route-html-relative-path>...");
        eprintln!("  e.g. route-weight apps/web/.next dashboard.html");
        return ExitCode::from(2);
    }

    let next_dir = PathBuf::from(&args[0]);
    let mut grand_total: u64 = 0;
    let mut failed = false;

    for route in &args[1..] {
        let html_path = next_dir.join("server/app").join(route);
        let html = match std::fs::read_to_string(&html_path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", html_path.display());
                failed = true;
                continue;
            }
        };

        let urls = extract_static_js(&html);
        let mut total: u64 = 0;
        let mut missing = 0usize;

        for url in &urls {
            // "/_next/static/x.js" -> "<next_dir>/static/x.js"
            let rel = url.trim_start_matches("/_next/");
            let path: &Path = &next_dir.join(rel);
            match std::fs::metadata(path) {
                Ok(m) => total += m.len(),
                Err(_) => missing += 1,
            }
        }

        // A chunk referenced but absent means the measurement is incomplete, and a
        // silently-low number would read as an improvement. Refuse to report it.
        if missing > 0 {
            eprintln!("error: {route}: {missing} referenced chunk(s) missing from disk");
            failed = true;
        }

        println!("{route}\t{total}\t{} chunks", urls.len());
        grand_total += total;
    }

    if args.len() > 2 {
        println!("TOTAL\t{grand_total}");
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::extract_static_js;

    #[test]
    fn finds_script_and_preload_urls() {
        let html = r#"<link rel="preload" href="/_next/static/chunks/a.js"/>
                      <script src="/_next/static/chunks/b.js"></script>"#;
        let found = extract_static_js(html);
        assert_eq!(found.len(), 2);
        assert!(found.contains("/_next/static/chunks/a.js"));
        assert!(found.contains("/_next/static/chunks/b.js"));
    }

    #[test]
    fn counts_a_repeated_chunk_once() {
        let html = r#"<script src="/_next/static/chunks/a.js"></script>
                      <script src="/_next/static/chunks/a.js"></script>"#;
        assert_eq!(extract_static_js(html).len(), 1);
    }

    #[test]
    fn ignores_non_js_assets() {
        let html = r#"<link href="/_next/static/css/x.css"/>
                      <script src="/_next/static/chunks/a.js"></script>"#;
        let found = extract_static_js(html);
        assert_eq!(found.len(), 1);
        assert!(found.contains("/_next/static/chunks/a.js"));
    }

    #[test]
    fn handles_single_quoted_attributes() {
        let html = "<script src='/_next/static/chunks/a.js'></script>";
        assert_eq!(extract_static_js(html).len(), 1);
    }
}
