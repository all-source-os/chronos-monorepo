//! Embedded Core quickstart — use AllSource Core as a library, no server needed.
//!
//! Run with:
//!   cargo run --no-default-features --features embedded --example embedded_quickstart

use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
use serde_json::json;

#[tokio::main]
async fn main() -> allsource_core::error::Result<()> {
    // 1. Open an in-memory core (or use .data_dir("path") for persistence)
    let core = EmbeddedCore::open(Config::builder().build()?).await?;

    // 2. Ingest events — plain strings, no value objects
    core.ingest(IngestEvent {
        entity_id: "order-1",
        event_type: "order.placed",
        payload: json!({"total": 99.99, "currency": "USD"}),
        metadata: None,
        tenant_id: None,
    })
    .await?;

    core.ingest(IngestEvent {
        entity_id: "order-1",
        event_type: "order.paid",
        payload: json!({"method": "credit_card"}),
        metadata: None,
        tenant_id: None,
    })
    .await?;

    core.ingest(IngestEvent {
        entity_id: "order-2",
        event_type: "order.placed",
        payload: json!({"total": 42.00, "currency": "EUR"}),
        metadata: None,
        tenant_id: None,
    })
    .await?;

    // 3. Query by entity ID
    let events = core.query(Query::new().entity_id("order-1")).await?;
    println!("Events for order-1: {} event(s)", events.len());
    for e in &events {
        println!("  {} — {}", e.event_type, e.payload);
    }

    // 4. Query by event type prefix
    let all_orders = core
        .query(Query::new().event_type_prefix("order."))
        .await?;
    println!("\nAll order events: {} event(s)", all_orders.len());

    // 5. Read projection state (entity_snapshots is built-in)
    if let Some(state) = core.projection("entity_snapshots", "order-1") {
        println!("\norder-1 snapshot: {state}");
    }

    // 6. Store stats
    let stats = core.stats();
    println!("\nStore stats: {} total events", stats.total_events);

    // 7. Graceful shutdown (flushes WAL + Parquet if persistence is enabled)
    core.shutdown().await?;
    println!("Shutdown complete.");

    Ok(())
}
