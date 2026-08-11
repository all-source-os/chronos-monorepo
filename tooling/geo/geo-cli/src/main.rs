//! `geo` — GEO (Generative Engine Optimization) measurement for
//! `www.all-source.xyz`.
//!
//! This slice ships the foundation: the `geo.*` event contract, the gateway
//! emitter, and `geo report`. The crawl, probe and research subcommands are
//! declared here so the later slices have a home to land in, and fail loudly
//! until they do.

mod report;

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};
use clap::{Parser, Subcommand};

use crate::report::Window;

/// Default reporting window, in days. GEO is trend analysis; a month is the
/// shortest window in which a change to a static surface is visible.
const DEFAULT_DAYS: i64 = 30;

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
    Crawl(NotImplementedArgs),
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

#[derive(clap::Args)]
struct ReportArgs {
    /// Window length in days, counted back from --until. Ignored when --since
    /// is given.
    #[arg(long, default_value_t = DEFAULT_DAYS, value_parser = clap::value_parser!(i64).range(1..))]
    days: i64,

    /// Start of the window (RFC 3339, e.g. 2026-07-01T00:00:00Z). UTC always.
    #[arg(long)]
    since: Option<String>,

    /// End of the window (RFC 3339). Defaults to now.
    #[arg(long)]
    until: Option<String>,

    /// Stop after this many events. Guards against a runaway scan.
    #[arg(long, default_value_t = 100_000)]
    max_events: u64,

    /// Events fetched per gateway request.
    #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u32).range(1..=1000))]
    page_size: u32,
}

/// Parse an RFC 3339 instant into UTC. GEO is trend analysis over multi-week
/// windows; a timezone slip silently ruins one, so nothing here is local time.
fn parse_utc(label: &str, raw: &str) -> Result<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(raw) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(e) => bail!("--{label} must be RFC 3339 (e.g. 2026-07-01T00:00:00Z), got {raw:?}: {e}"),
    }
}

fn window_from(args: &ReportArgs) -> Result<Window> {
    let until = match &args.until {
        Some(raw) => parse_utc("until", raw)?,
        None => Utc::now(),
    };
    let since = match &args.since {
        Some(raw) => parse_utc("since", raw)?,
        None => until - Duration::days(args.days),
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

    match &cli.command {
        Command::Report(args) => {
            let window = window_from(args)?;
            if cli.dry_run {
                report::run_dry_run(window)
            } else {
                report::run_live(window, args.max_events, args.page_size).await
            }
        }
        Command::Crawl(_) => not_implemented("geo crawl", "024", "AI-crawler log ingest"),
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

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn default_window_is_thirty_days_back_from_until() {
        let args = ReportArgs {
            days: DEFAULT_DAYS,
            since: None,
            until: Some("2026-08-11T00:00:00Z".to_string()),
            max_events: 100,
            page_size: 100,
        };
        let window = window_from(&args).expect("window resolves");
        assert_eq!(window.until - window.since, Duration::days(DEFAULT_DAYS));
    }

    #[test]
    fn explicit_since_wins_over_days() {
        let args = ReportArgs {
            days: 7,
            since: Some("2026-01-01T00:00:00Z".to_string()),
            until: Some("2026-02-01T00:00:00Z".to_string()),
            max_events: 100,
            page_size: 100,
        };
        let window = window_from(&args).expect("window resolves");
        assert_eq!(window.until - window.since, Duration::days(31));
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
    }

    #[test]
    fn an_inverted_window_is_refused() {
        let args = ReportArgs {
            days: 7,
            since: Some("2026-03-01T00:00:00Z".to_string()),
            until: Some("2026-02-01T00:00:00Z".to_string()),
            max_events: 100,
            page_size: 100,
        };
        assert!(window_from(&args).is_err());
    }

    #[test]
    fn stubs_point_at_the_slice_that_implements_them() {
        let err = not_implemented("geo crawl", "024", "log ingest").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(".prompts/024"), "{msg}");
    }
}
