//! `geo` — GEO (Generative Engine Optimization) measurement for
//! `www.all-source.xyz`.
//!
//! Shipped so far: the `geo.*` event contract and gateway emitter (prompt
//! 023), and layers 1 and 2 — referral attribution and crawl-log diagnostics
//! (prompt 024). `geo probe` and `geo research` are declared here so the later
//! slices have a home to land in, and fail loudly until they do.

mod crawl;
mod logs;
mod ranges;
mod report;

use std::path::PathBuf;

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};
use geo_core::{Aggregation, EmitMode};

use crate::{logs::LogFormat, report::Window};

/// Default reporting window, in days. GEO is trend analysis; a month is the
/// shortest window in which a change to a static surface is visible.
const DEFAULT_DAYS: i64 = 30;

/// Default crawl-ingest window, in days. Log retention is short and a crawl
/// ingest is meant to run often; a week is the natural catch-up window.
const DEFAULT_CRAWL_DAYS: i64 = 7;

#[derive(Parser)]
#[command(
    name = "geo",
    version,
    about = "GEO measurement for www.all-source.xyz",
    long_about = "Measures how AllSource shows up in generative engines: who arrives from an AI \
                  surface, which AI crawlers read the site, what the engines say about us, and \
                  whether a change moved any of it.\n\nAll telemetry is written to AllSource \
                  through the Control Plane gateway as durable geo.* events — see \
                  docs/contracts/geo-events/README.md."
)]
struct Cli {
    /// Print what would be emitted or queried instead of touching the gateway.
    /// Needs no API key.
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Ingest AI-crawler hits from the edge logs (layer 2).
    Crawl(Box<CrawlArgs>),
    /// Run share-of-voice and interrogation probes against the engines
    /// (layers 3a/3b).
    Probe(NotImplementedArgs),
    /// Count geo.* events per measurement layer over a window.
    Report(ReportArgs),
    /// Drive the optimization loop: open an experiment, score it, decide
    /// (layer 5).
    Research(NotImplementedArgs),
}

/// Stub subcommands take (and ignore) trailing arguments so a later slice can
/// define its own flags without breaking anyone's muscle memory today.
#[derive(clap::Args)]
struct NotImplementedArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    args: Vec<String>,
}

/// Window flags, shared by every subcommand that has one so `--since 7d` means
/// the same thing everywhere.
#[derive(clap::Args, Clone)]
struct WindowArgs {
    /// Start of the window: a duration back from --until (`7d`, `12h`, `4w`)
    /// or an RFC 3339 instant (`2026-07-01T00:00:00Z`). UTC always.
    #[arg(long)]
    since: Option<String>,

    /// End of the window (RFC 3339). Defaults to now.
    #[arg(long)]
    until: Option<String>,
}

#[derive(clap::Args)]
struct ReportArgs {
    /// Window length in days, counted back from --until. Ignored when --since
    /// is given.
    #[arg(long, default_value_t = DEFAULT_DAYS, value_parser = clap::value_parser!(i64).range(1..))]
    days: i64,

    #[command(flatten)]
    window: WindowArgs,

    /// Stop after this many events. Guards against a runaway scan.
    #[arg(long, default_value_t = 100_000)]
    max_events: u64,

    /// Events fetched per gateway request.
    #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u32).range(1..=1000))]
    page_size: u32,
}

#[derive(clap::Args)]
struct CrawlArgs {
    /// Window length in days, counted back from --until. Ignored when --since
    /// is given.
    #[arg(long, default_value_t = DEFAULT_CRAWL_DAYS, value_parser = clap::value_parser!(i64).range(1..))]
    days: i64,

    #[command(flatten)]
    window: WindowArgs,

    /// An access-log file: a Vercel log-drain dump (NDJSON) or a Combined Log
    /// Format stream. Repeatable.
    #[arg(long, value_name = "PATH")]
    file: Vec<PathBuf>,

    /// Pull logs by shelling out to `vercel logs <project> --json`.
    #[arg(long, value_name = "PROJECT")]
    vercel_project: Option<String>,

    /// Pull logs by shelling out to `fly logs -a <app> --no-tail --json`.
    #[arg(long, value_name = "APP")]
    fly_app: Option<String>,

    /// Log format. `auto` sniffs the first non-blank line.
    #[arg(long, value_enum, default_value_t = LogFormat::Auto)]
    format: LogFormat,

    /// How much one emitted event stands for. `hit` keeps full fidelity;
    /// `hourly`/`daily` collapse a wide backfill.
    #[arg(long, value_enum, default_value_t = AggregationArg::Hit)]
    aggregate: AggregationArg,

    /// Directory holding cached vendor IP-range lists. Read first, and written
    /// to after a successful fetch. With --offline this is the only source.
    #[arg(long, value_name = "DIR")]
    ranges_dir: Option<PathBuf>,

    /// Never touch the network for vendor range lists. Bots whose list is not
    /// cached become `unverifiable` — never `rejected`.
    #[arg(long)]
    offline: bool,

    /// Also emit hits whose identity could not be checked, stamped
    /// `verified: false`. Spoofed hits (IP outside the vendor's published
    /// range) are never emitted either way.
    #[arg(long)]
    include_unverified: bool,
}

/// clap mirror of [`Aggregation`]. Kept here so `geo-core` does not grow a
/// clap dependency for a CLI concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum AggregationArg {
    Hit,
    Hourly,
    Daily,
}

impl From<AggregationArg> for Aggregation {
    fn from(arg: AggregationArg) -> Self {
        match arg {
            AggregationArg::Hit => Self::Hit,
            AggregationArg::Hourly => Self::Hourly,
            AggregationArg::Daily => Self::Daily,
        }
    }
}

/// Parse a duration shorthand: `12h`, `7d`, `4w`. Returns `None` for anything
/// else so the caller can fall through to RFC 3339.
fn parse_duration(raw: &str) -> Option<Duration> {
    let raw = raw.trim();
    let (digits, unit) = raw.split_at(raw.len().checked_sub(1)?);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: i64 = digits.parse().ok()?;
    if n == 0 {
        return None;
    }
    match unit {
        "h" | "H" => Some(Duration::hours(n)),
        "d" | "D" => Some(Duration::days(n)),
        "w" | "W" => Some(Duration::weeks(n)),
        _ => None,
    }
}

/// Parse an RFC 3339 instant into UTC. GEO is trend analysis over multi-week
/// windows; a timezone slip silently ruins one, so nothing here is local time.
fn parse_utc(label: &str, raw: &str) -> Result<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(e) => bail!(
            "--{label} must be RFC 3339 (e.g. 2026-07-01T00:00:00Z) or a duration \
             (7d, 12h, 4w), got {raw:?}: {e}"
        ),
    }
}

/// Resolve a window from `--since`/`--until`/`--days`.
fn resolve_window(args: &WindowArgs, default_days: i64) -> Result<Window> {
    let until = match &args.until {
        Some(raw) => parse_utc("until", raw)?,
        None => Utc::now(),
    };
    let since = match &args.since {
        Some(raw) => match parse_duration(raw) {
            Some(d) => until - d,
            None => parse_utc("since", raw)?,
        },
        None => until - Duration::days(default_days),
    };
    if since >= until {
        bail!(
            "empty window: --since ({since}) is not before --until ({until})",
            since = since.to_rfc3339(),
            until = until.to_rfc3339()
        );
    }
    Ok(Window { since, until })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mode = if cli.dry_run {
        EmitMode::DryRun
    } else {
        EmitMode::Live
    };

    match &cli.command {
        Command::Report(args) => {
            let window = resolve_window(&args.window, args.days)?;
            if cli.dry_run {
                report::run_dry_run(window)
            } else {
                report::run_live(window, args.max_events, args.page_size).await
            }
        }
        Command::Crawl(args) => {
            let window = resolve_window(&args.window, args.days)?;
            crawl::run(crawl::CrawlOptions {
                window,
                inputs: crawl::inputs_from(
                    &args.file,
                    args.vercel_project.as_deref(),
                    args.fly_app.as_deref(),
                ),
                format: args.format,
                aggregation: args.aggregate.into(),
                ranges_dir: args.ranges_dir.clone(),
                offline: args.offline,
                include_unverified: args.include_unverified,
                mode,
            })
            .await
        }
        Command::Probe(_) => {
            not_implemented("geo probe", "025", "share-of-voice + interrogation probes")
        }
        Command::Research(_) => not_implemented("geo research", "027", "the optimization loop"),
    }
}

/// Exits non-zero with a pointer at the slice that will implement it.
fn not_implemented(command: &str, prompt: &str, what: &str) -> Result<()> {
    bail!(
        "{command} is not implemented — {what} lands in .prompts/{prompt}.\n\
         The geo.* event contract it will write is already defined: see \
         docs/contracts/geo-events/README.md.\n\
         Try `geo report --dry-run` to see the shape of the events it will emit."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn window_args(since: Option<&str>, until: Option<&str>) -> WindowArgs {
        WindowArgs {
            since: since.map(str::to_string),
            until: until.map(str::to_string),
        }
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn default_window_is_thirty_days_back_from_until() {
        let window = resolve_window(
            &window_args(None, Some("2026-08-11T00:00:00Z")),
            DEFAULT_DAYS,
        )
        .expect("window resolves");
        assert_eq!(window.until - window.since, Duration::days(DEFAULT_DAYS));
    }

    #[test]
    fn explicit_since_wins_over_days() {
        let window = resolve_window(
            &window_args(Some("2026-01-01T00:00:00Z"), Some("2026-02-01T00:00:00Z")),
            7,
        )
        .expect("window resolves");
        assert_eq!(window.until - window.since, Duration::days(31));
    }

    #[test]
    fn a_duration_shorthand_is_a_valid_since() {
        for (raw, expected) in [
            ("7d", Duration::days(7)),
            ("12h", Duration::hours(12)),
            ("4w", Duration::weeks(4)),
        ] {
            let window = resolve_window(&window_args(Some(raw), Some("2026-08-11T00:00:00Z")), 30)
                .unwrap_or_else(|e| panic!("{raw} should resolve: {e}"));
            assert_eq!(window.until - window.since, expected, "{raw}");
        }
    }

    #[test]
    fn a_bare_number_is_not_a_duration() {
        assert_eq!(parse_duration("7"), None);
        assert_eq!(parse_duration("d"), None);
        assert_eq!(parse_duration("0d"), None);
        assert_eq!(parse_duration("7y"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn non_utc_input_is_normalised_not_rejected() {
        // +02:00 is accepted and converted; what we refuse is ambiguity.
        let dt = parse_utc("since", "2026-08-11T02:00:00+02:00").expect("parses");
        assert_eq!(dt.to_rfc3339(), "2026-08-11T00:00:00+00:00");
    }

    #[test]
    fn a_local_timestamp_without_an_offset_is_refused() {
        assert!(parse_utc("since", "2026-08-11 09:00:00").is_err());
        assert!(resolve_window(&window_args(Some("2026-08-11 09:00:00"), None), 30).is_err());
    }

    #[test]
    fn an_inverted_window_is_refused() {
        assert!(
            resolve_window(
                &window_args(Some("2026-03-01T00:00:00Z"), Some("2026-02-01T00:00:00Z")),
                7
            )
            .is_err()
        );
    }

    #[test]
    fn stubs_point_at_the_slice_that_implements_them() {
        let err = not_implemented("geo probe", "025", "probes").unwrap_err();
        assert!(err.to_string().contains(".prompts/025"), "{err}");
    }

    #[test]
    fn the_aggregation_flag_maps_onto_the_contract_enum() {
        assert_eq!(Aggregation::from(AggregationArg::Hit), Aggregation::Hit);
        assert_eq!(
            Aggregation::from(AggregationArg::Hourly),
            Aggregation::Hourly
        );
        assert_eq!(Aggregation::from(AggregationArg::Daily), Aggregation::Daily);
    }
}
