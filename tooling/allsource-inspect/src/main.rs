use allsource_core::{
    embedded::{Config, DurabilityStatus, EmbeddedCore, EventView, Query},
    store::StoreStats,
};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "allsource-inspect",
    about = "CLI tool for inspecting AllSource Core storage files",
    version
)]
struct Cli {
    /// Path to the AllSource data directory
    #[arg(long, env = "ALLSOURCE_DATA_DIR")]
    data_dir: PathBuf,

    /// Output format
    #[arg(long, default_value = "table")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Table,
}

#[derive(Subcommand)]
enum Command {
    /// Query events with filters
    Events {
        /// Filter by entity ID (exact match)
        #[arg(long)]
        entity_id: Option<String>,

        /// Filter by event type (exact match)
        #[arg(long)]
        event_type: Option<String>,

        /// Filter by event type prefix (e.g. "order." matches "order.placed")
        #[arg(long)]
        event_type_prefix: Option<String>,

        /// Filter events after this time (RFC 3339)
        #[arg(long)]
        since: Option<String>,

        /// Filter events before this time (RFC 3339)
        #[arg(long)]
        until: Option<String>,

        /// Maximum number of events to return
        #[arg(long, default_value = "100")]
        limit: usize,
    },
    /// Show storage summary and stats
    Summary,
    /// Show WAL status and recovered events
    Wal {
        /// Show only WAL events (skip events already in parquet)
        #[arg(long)]
        wal_only: bool,

        /// Maximum number of WAL events to display
        #[arg(long, default_value = "50")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = Config::builder().data_dir(&cli.data_dir).build()?;
    let core = EmbeddedCore::open(config).await?;

    match cli.command {
        Command::Events {
            entity_id,
            event_type,
            event_type_prefix,
            since,
            until,
            limit,
        } => {
            let since_dt = since
                .as_deref()
                .map(|s| s.parse::<DateTime<Utc>>())
                .transpose()?;
            let until_dt = until
                .as_deref()
                .map(|s| s.parse::<DateTime<Utc>>())
                .transpose()?;

            let mut query = Query::new();
            if let Some(ref id) = entity_id {
                query = query.entity_id(id);
            }
            if let Some(ref et) = event_type {
                query = query.event_type(et);
            }
            if let Some(ref prefix) = event_type_prefix {
                query = query.event_type_prefix(prefix);
            }
            if let Some(dt) = since_dt {
                query = query.since(dt);
            }
            if let Some(dt) = until_dt {
                query = query.until(dt);
            }
            query = query.limit(limit);

            let events = core.query(query).await?;
            print_events(&cli.format, &events);
        }
        Command::Summary => {
            cmd_summary(&core, &cli.format);
        }
        Command::Wal { wal_only, limit } => {
            cmd_wal(&core, &cli.format, wal_only, limit);
        }
    }

    core.shutdown().await?;
    Ok(())
}

fn print_events(format: &OutputFormat, events: &[EventView]) {
    match format {
        OutputFormat::Json => {
            for event in events {
                println!("{}", serde_json::to_string(event).unwrap());
            }
        }
        OutputFormat::Table => {
            if events.is_empty() {
                println!("No events found.");
                return;
            }
            let mut table = comfy_table::Table::new();
            table.set_header(vec![
                "ID",
                "Entity ID",
                "Event Type",
                "Timestamp",
                "Payload (truncated)",
            ]);
            for event in events {
                let payload_str = event.payload.to_string();
                let truncated = if payload_str.len() > 80 {
                    format!("{}…", &payload_str[..80])
                } else {
                    payload_str
                };
                table.add_row(vec![
                    event.id.to_string(),
                    event.entity_id.clone(),
                    event.event_type.clone(),
                    event.timestamp.to_rfc3339(),
                    truncated,
                ]);
            }
            println!("{table}");
            println!("\n{} event(s) returned", events.len());
        }
    }
}

fn cmd_summary(core: &EmbeddedCore, format: &OutputFormat) {
    let status = core.durability_status();
    let stats = core.stats();
    let store = core.inner();
    let streams = store.list_streams();
    let event_types = store.list_event_types();

    // Find date range from streams
    let all_dates: Vec<DateTime<Utc>> = streams.iter().filter_map(|s| s.last_event_at).collect();
    let earliest = all_dates.iter().min().copied();
    let latest = all_dates.iter().max().copied();

    // Top 10 event types by count
    let mut sorted_types = event_types.clone();
    sorted_types.sort_by(|a, b| b.event_count.cmp(&a.event_count));
    sorted_types.truncate(10);

    match format {
        OutputFormat::Json => {
            let top_types: Vec<serde_json::Value> = sorted_types
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "event_type": t.event_type,
                        "count": t.event_count,
                    })
                })
                .collect();
            let summary = serde_json::json!({
                "total_events": stats.total_events,
                "total_entities": stats.total_entities,
                "total_event_types": stats.total_event_types,
                "date_range": {
                    "earliest": earliest.map(|d| d.to_rfc3339()),
                    "latest": latest.map(|d| d.to_rfc3339()),
                },
                "top_event_types": top_types,
                "durability": {
                    "durable": status.durable,
                    "memory_events": status.memory_events,
                    "wal_enabled": status.wal_enabled,
                    "wal_entries": status.wal_entries,
                    "wal_bytes": status.wal_bytes,
                    "wal_sequence": status.wal_sequence,
                    "parquet_enabled": status.parquet_enabled,
                    "parquet_files": status.parquet_files,
                    "parquet_bytes": status.parquet_bytes,
                    "parquet_pending_batch": status.parquet_pending_batch,
                },
                "warnings": status.warnings,
            });
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        OutputFormat::Table => {
            print_summary_table(&stats, &status, earliest, latest, &sorted_types);
        }
    }
}

fn print_summary_table(
    stats: &StoreStats,
    status: &DurabilityStatus,
    earliest: Option<DateTime<Utc>>,
    latest: Option<DateTime<Utc>>,
    top_types: &[allsource_core::store::EventTypeInfo],
) {
    println!("AllSource Core Storage Summary");
    println!("==============================\n");

    println!("Events:      {}", stats.total_events);
    println!("Entities:    {}", stats.total_entities);
    println!("Event Types: {}", stats.total_event_types);

    match (earliest, latest) {
        (Some(e), Some(l)) => println!("Date Range:  {} → {}", e.to_rfc3339(), l.to_rfc3339()),
        _ => println!("Date Range:  (no events)"),
    }

    println!(
        "\nDurability:  {}",
        if status.durable { "YES" } else { "NO" }
    );
    println!("Memory:      {} events", status.memory_events);

    println!(
        "\nWAL:         {}",
        if status.wal_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    if status.wal_enabled {
        println!("  Entries:   {}", status.wal_entries);
        println!("  Bytes:     {}", format_bytes(status.wal_bytes));
        println!("  Sequence:  {}", status.wal_sequence);
    }

    println!(
        "\nParquet:     {}",
        if status.parquet_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    if status.parquet_enabled {
        println!("  Files:     {}", status.parquet_files);
        println!("  Bytes:     {}", format_bytes(status.parquet_bytes));
        println!("  Pending:   {} events", status.parquet_pending_batch);
    }

    if !top_types.is_empty() {
        println!("\nTop Event Types:");
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Event Type", "Count", "Last Seen"]);
        for t in top_types {
            table.add_row(vec![
                t.event_type.clone(),
                t.event_count.to_string(),
                t.last_event_at
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_else(|| "-".to_string()),
            ]);
        }
        println!("{table}");
    }

    if !status.warnings.is_empty() {
        println!("\nWarnings:");
        for w in &status.warnings {
            println!("  - {w}");
        }
    }
}

fn cmd_wal(core: &EmbeddedCore, format: &OutputFormat, _wal_only: bool, limit: usize) {
    let status = core.durability_status();
    let store = core.inner();

    match format {
        OutputFormat::Json => {
            let mut wal_info = serde_json::json!({
                "enabled": status.wal_enabled,
                "entries": status.wal_entries,
                "bytes": status.wal_bytes,
                "sequence": status.wal_sequence,
            });

            // Recover WAL events to show them
            if status.wal_enabled
                && let Some(wal) = store.wal()
            {
                match wal.recover() {
                    Ok(events) => {
                        let capped: Vec<&_> = events.iter().take(limit).collect();
                        let event_views: Vec<EventView> =
                            capped.iter().map(|e| EventView::from(*e)).collect();
                        wal_info["recovered_events"] = serde_json::json!(event_views.len());
                        for ev in &event_views {
                            println!("{}", serde_json::to_string(ev).unwrap());
                        }
                        return;
                    }
                    Err(e) => {
                        wal_info["recovery_error"] = serde_json::json!(e.to_string());
                    }
                }
            }

            println!("{}", serde_json::to_string_pretty(&wal_info).unwrap());
        }
        OutputFormat::Table => {
            if !status.wal_enabled {
                println!("WAL is not enabled for this data directory.");
                return;
            }
            println!("WAL Status");
            println!("==========\n");
            println!("Entries:   {}", status.wal_entries);
            println!("Bytes:     {}", format_bytes(status.wal_bytes));
            println!("Sequence:  {}", status.wal_sequence);

            // Recover and display WAL events
            if let Some(wal) = store.wal() {
                match wal.recover() {
                    Ok(events) => {
                        if events.is_empty() {
                            println!("\nNo events in WAL.");
                            return;
                        }
                        println!("\nRecovered {} event(s) from WAL:", events.len());
                        let capped: Vec<&_> = events.iter().take(limit).collect();
                        let event_views: Vec<EventView> =
                            capped.iter().map(|e| EventView::from(*e)).collect();
                        print_events(&OutputFormat::Table, &event_views);
                    }
                    Err(e) => {
                        println!("\nWAL recovery failed: {e}");
                    }
                }
            }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
