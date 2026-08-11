//! Access-log parsing.
//!
//! Two formats cover both edges we care about:
//!
//! - **JSON lines** — what a Vercel log drain delivers for `apps/web`, one
//!   JSON object per line. The fields we need live either at the top level or
//!   inside a `proxy` object; `proxy` wins because it is the edge's own view
//!   of the request. `fly logs --json` also lands here, but its objects carry
//!   the access line inside `message`, so we fall through to CLF on that.
//! - **Combined Log Format** — the classic `ip - - [ts] "GET /p HTTP/1.1" 200
//!   123 "ref" "ua"` line, which is what falls out of a plain-text log stream.
//!
//! Nothing here knows what a bot is. Parsing and identification are separate
//! so a parser bug and a taxonomy bug cannot be mistaken for each other.

use std::net::IpAddr;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

/// One request as read from a log, before anything decides what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessLine {
    /// When the edge served the request (UTC).
    pub timestamp: DateTime<Utc>,
    /// Client IP, when the log carried one. Without it no identity claim can
    /// be verified, so it is an `Option` rather than a silent `0.0.0.0`.
    pub client_ip: Option<IpAddr>,
    /// Raw `User-Agent`.
    pub user_agent: String,
    /// Path requested, site-relative, query string stripped.
    pub path: String,
    /// HTTP status served.
    pub status: u16,
    /// The edge's request id, when the log carried one.
    pub request_id: Option<String>,
}

/// Which parser to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LogFormat {
    /// Sniff the first non-blank line: `{` means JSON lines, anything else CLF.
    Auto,
    /// One JSON object per line (Vercel log drain, `fly logs --json`).
    Json,
    /// Combined Log Format.
    Clf,
}

/// What a parse run produced, including what it could not read.
///
/// Skips are counted and sampled rather than swallowed: a format change that
/// silently drops every line looks exactly like "no AI traffic", which is the
/// misread this whole layer exists to prevent.
#[derive(Debug, Default)]
pub struct ParseReport {
    pub lines: Vec<AccessLine>,
    pub blank: usize,
    pub unparseable: usize,
    /// Up to [`MAX_SAMPLES`] examples of lines we could not read.
    pub samples: Vec<String>,
}

/// How many unparseable lines to keep for the operator to look at.
pub const MAX_SAMPLES: usize = 3;

impl ParseReport {
    fn skip(&mut self, line: &str) {
        self.unparseable += 1;
        if self.samples.len() < MAX_SAMPLES {
            self.samples.push(line.chars().take(200).collect());
        }
    }
}

/// Parse a whole log body.
pub fn parse(input: &str, format: LogFormat) -> ParseReport {
    let mut report = ParseReport::default();
    let resolved = match format {
        LogFormat::Auto => sniff(input),
        other => other,
    };

    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() {
            report.blank += 1;
            continue;
        }
        let parsed = match resolved {
            LogFormat::Json => parse_json_line(line),
            // `Auto` is resolved above; treating a stray value as CLF is the
            // conservative choice because CLF parsing is strict.
            LogFormat::Clf | LogFormat::Auto => parse_clf_line(line),
        };
        match parsed {
            Some(access) => report.lines.push(access),
            None => report.skip(line),
        }
    }
    report
}

/// Decide the format from the first non-blank line.
fn sniff(input: &str) -> LogFormat {
    input
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .filter(|l| l.starts_with('{'))
        .map_or(LogFormat::Clf, |_| LogFormat::Json)
}

// ───────────────────────────────────────────────────────────────────────────
// JSON lines
// ───────────────────────────────────────────────────────────────────────────

/// Read a field from `proxy` first, then the top level.
///
/// `proxy` is the edge's own record of the request; the top level is the
/// function's. When both exist they agree, and when they disagree the edge is
/// the one that saw the client.
fn field<'a>(root: &'a Value, proxy: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    proxy
        .and_then(|p| p.get(key))
        .filter(|v| !v.is_null())
        .or_else(|| root.get(key).filter(|v| !v.is_null()))
}

fn parse_json_line(line: &str) -> Option<AccessLine> {
    let root: Value = serde_json::from_str(line).ok()?;
    let proxy = root.get("proxy");

    // `fly logs --json` and any other line-oriented shipper wraps a plain
    // access line in `message`. Try the structured fields first; if the shape
    // has no path at all, fall through to CLF on the message.
    let path_value = field(&root, proxy, "path");
    if path_value.is_none() {
        if let Some(message) = root.get("message").and_then(Value::as_str) {
            return parse_clf_line(message.trim());
        }
        return None;
    }

    let path = normalise_path(path_value?.as_str()?);
    let status = field(&root, proxy, "statusCode")
        .and_then(json_u16)
        .unwrap_or(0);
    let user_agent = field(&root, proxy, "userAgent")
        .and_then(json_user_agent)
        .unwrap_or_default();
    let client_ip = field(&root, proxy, "clientIp")
        .and_then(Value::as_str)
        .and_then(parse_ip);
    let request_id = field(&root, proxy, "requestId")
        .or_else(|| root.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let timestamp = field(&root, proxy, "timestamp")
        .or_else(|| root.get("timestampInMs"))
        .and_then(json_timestamp)?;

    Some(AccessLine {
        timestamp,
        client_ip,
        user_agent,
        path,
        status,
        request_id,
    })
}

/// `userAgent` is an array in a Vercel drain payload and a string elsewhere.
fn json_user_agent(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items
            .iter()
            .find_map(Value::as_str)
            .map(std::string::ToString::to_string),
        _ => None,
    }
}

fn json_u16(value: &Value) -> Option<u16> {
    match value {
        Value::Number(n) => u16::try_from(n.as_u64()?).ok(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Epoch milliseconds (number or numeric string) or an RFC 3339 string.
fn json_timestamp(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::Number(n) => from_millis(n.as_i64()?),
        Value::String(s) => {
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }
            from_millis(s.parse().ok()?)
        }
        _ => None,
    }
}

fn from_millis(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms).single()
}

// ───────────────────────────────────────────────────────────────────────────
// Combined Log Format
// ───────────────────────────────────────────────────────────────────────────

fn parse_clf_line(line: &str) -> Option<AccessLine> {
    let client_ip = line.split_whitespace().next().and_then(parse_ip);

    let open = line.find('[')?;
    let close = line[open..].find(']')? + open;
    let timestamp = DateTime::parse_from_str(&line[open + 1..close], "%d/%b/%Y:%H:%M:%S %z")
        .ok()?
        .with_timezone(&Utc);

    let quoted = quoted_fields(&line[close + 1..]);
    let request = quoted.first()?;
    let mut request_parts = request.split_whitespace();
    let _method = request_parts.next()?;
    let path = normalise_path(request_parts.next()?);

    // After the closing quote of the request come `status bytes`.
    let after_request = line[close + 1..]
        .split('"')
        .nth(2)
        .unwrap_or_default()
        .trim();
    let status = after_request
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Combined format is `"request" "referer" "user-agent"`.
    let user_agent = quoted.get(2).cloned().unwrap_or_default();

    Some(AccessLine {
        timestamp,
        client_ip,
        user_agent,
        path,
        status,
        request_id: None,
    })
}

/// Split out every `"..."` run, in order. No escape handling: neither a Vercel
/// nor an nginx access line escapes quotes inside these fields.
fn quoted_fields(rest: &str) -> Vec<String> {
    rest.split('"')
        .skip(1)
        .step_by(2)
        .map(std::string::ToString::to_string)
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
// Shared helpers
// ───────────────────────────────────────────────────────────────────────────

/// Strip a query string and any absolute-URL prefix, and guarantee a leading
/// slash — so `/docs?utm=x`, `https://host/docs` and `/docs` are one path in
/// the report rather than three.
fn normalise_path(raw: &str) -> String {
    let without_scheme = match raw.split_once("://") {
        Some((_, rest)) => rest.find('/').map_or("/", |i| &rest[i..]),
        None => raw,
    };
    let without_query = without_scheme
        .split(['?', '#'])
        .next()
        .unwrap_or(without_scheme);
    if without_query.starts_with('/') {
        without_query.to_string()
    } else {
        format!("/{without_query}")
    }
}

/// Parse a client IP, tolerating an `X-Forwarded-For`-style list (first entry
/// is the client) and a `host:port` v4 form.
fn parse_ip(raw: &str) -> Option<IpAddr> {
    let first = raw.split(',').next()?.trim();
    if let Ok(ip) = first.parse() {
        return Some(ip);
    }
    // `1.2.3.4:5678` — only meaningful for v4; a bare v6 also contains colons,
    // so only strip when what is left parses.
    first.rsplit_once(':').and_then(|(head, _)| head.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERCEL_LINE: &str = r#"{"id":"1786370400000-abc","timestampInMs":1786370400000,"type":"request","source":"static","requestId":"iad1::abcde-1786370400000-0f1e2d3c4b5a","proxy":{"timestamp":1786370400000,"method":"GET","host":"www.all-source.xyz","path":"/llms.txt","statusCode":200,"clientIp":"132.196.86.9","region":"iad1","userAgent":["Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko); compatible; GPTBot/1.2; +https://openai.com/gptbot"],"referer":""}}"#;

    #[test]
    fn a_vercel_drain_line_parses_from_the_proxy_object() {
        let line = parse_json_line(VERCEL_LINE).expect("parses");
        assert_eq!(line.path, "/llms.txt");
        assert_eq!(line.status, 200);
        assert_eq!(line.client_ip, Some("132.196.86.9".parse().unwrap()));
        assert!(line.user_agent.contains("GPTBot"));
        assert_eq!(
            line.request_id.as_deref(),
            Some("iad1::abcde-1786370400000-0f1e2d3c4b5a")
        );
        assert_eq!(line.timestamp.timestamp_millis(), 1_786_370_400_000);
    }

    #[test]
    fn the_proxy_object_wins_over_the_top_level() {
        let json = r#"{"timestampInMs":1786370400000,"path":"/wrong","statusCode":500,
            "proxy":{"timestamp":1786370400000,"path":"/right","statusCode":200,
                     "clientIp":"1.2.3.4","userAgent":"GPTBot/1.2"}}"#;
        let line = parse_json_line(json).expect("parses");
        assert_eq!(line.path, "/right");
        assert_eq!(line.status, 200);
    }

    #[test]
    fn a_flat_json_line_without_a_proxy_object_still_parses() {
        let json = r#"{"timestamp":"2026-08-10T12:00:00Z","path":"/pricing?utm_source=x",
            "statusCode":200,"clientIp":"1.2.3.4","userAgent":"ClaudeBot/1.0"}"#;
        let line = parse_json_line(json).expect("parses");
        assert_eq!(line.path, "/pricing");
        assert_eq!(line.user_agent, "ClaudeBot/1.0");
    }

    #[test]
    fn a_json_wrapper_around_a_clf_message_falls_through_to_clf() {
        // This is the `fly logs --json` shape.
        let json = r#"{"timestamp":"2026-08-10T12:00:00Z","message":"1.2.3.4 - - [10/Aug/2026:12:00:00 +0000] \"GET /docs HTTP/1.1\" 200 512 \"-\" \"CCBot/2.0 (https://commoncrawl.org/faq/)\""}"#;
        let line = parse_json_line(json).expect("parses");
        assert_eq!(line.path, "/docs");
        assert_eq!(line.status, 200);
        assert!(line.user_agent.starts_with("CCBot"));
    }

    #[test]
    fn a_combined_log_format_line_parses() {
        let raw = r#"20.171.206.7 - - [10/Aug/2026:12:34:56 +0000] "GET /blog/agent-memory?ref=x HTTP/1.1" 200 51234 "-" "Mozilla/5.0 (compatible; OAI-SearchBot/1.0; +https://openai.com/searchbot)""#;
        let line = parse_clf_line(raw).expect("parses");
        assert_eq!(line.path, "/blog/agent-memory");
        assert_eq!(line.status, 200);
        assert_eq!(line.client_ip, Some("20.171.206.7".parse().unwrap()));
        assert!(line.user_agent.contains("OAI-SearchBot"));
        assert_eq!(line.timestamp.to_rfc3339(), "2026-08-10T12:34:56+00:00");
        assert_eq!(line.request_id, None);
    }

    #[test]
    fn a_non_utc_clf_offset_is_converted_not_dropped() {
        let raw = r#"1.2.3.4 - - [10/Aug/2026:14:00:00 +0200] "GET / HTTP/1.1" 200 1 "-" "GPTBot/1.2""#;
        let line = parse_clf_line(raw).expect("parses");
        assert_eq!(line.timestamp.to_rfc3339(), "2026-08-10T12:00:00+00:00");
    }

    #[test]
    fn a_forwarded_for_list_yields_the_client_not_the_proxy() {
        assert_eq!(
            parse_ip("203.0.113.5, 10.0.0.1"),
            Some("203.0.113.5".parse().unwrap())
        );
    }

    #[test]
    fn an_ipv6_client_survives_the_port_stripping_heuristic() {
        assert_eq!(
            parse_ip("2600:1f18::1"),
            Some("2600:1f18::1".parse::<IpAddr>().unwrap())
        );
        assert_eq!(
            parse_ip("1.2.3.4:5678"),
            Some("1.2.3.4".parse::<IpAddr>().unwrap())
        );
        assert_eq!(parse_ip("not-an-ip"), None);
    }

    #[test]
    fn paths_normalise_to_one_form() {
        assert_eq!(normalise_path("/docs?a=1#x"), "/docs");
        assert_eq!(normalise_path("https://www.all-source.xyz/docs"), "/docs");
        assert_eq!(normalise_path("docs"), "/docs");
        assert_eq!(normalise_path("https://www.all-source.xyz"), "/");
    }

    #[test]
    fn format_is_sniffed_from_the_first_non_blank_line() {
        assert_eq!(sniff("\n\n{\"a\":1}\n"), LogFormat::Json);
        assert_eq!(sniff("\n1.2.3.4 - - [..]\n"), LogFormat::Clf);
        assert_eq!(sniff(""), LogFormat::Clf);
    }

    #[test]
    fn unparseable_lines_are_counted_and_sampled_not_swallowed() {
        let input = format!("{VERCEL_LINE}\n\nnot json at all\n{{\"broken\": \n");
        let report = parse(&input, LogFormat::Auto);
        assert_eq!(report.lines.len(), 1);
        assert_eq!(report.blank, 1);
        assert_eq!(report.unparseable, 2);
        assert_eq!(report.samples.len(), 2);
    }

    #[test]
    fn an_empty_log_parses_to_nothing_rather_than_failing() {
        let report = parse("\n\n", LogFormat::Auto);
        assert!(report.lines.is_empty());
        assert_eq!(report.blank, 2);
        assert_eq!(report.unparseable, 0);
    }
}
