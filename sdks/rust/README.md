# AllSource Rust Client

Typed Rust client for the [AllSource](https://github.com/all-source-os/allsource-monorepo) event store.

## Install

```toml
[dependencies]
allsource = { path = "../packages/rust-client" }  # or from crates.io when published
```

## Usage

```rust
use allsource::{Client, IngestEventInput, QueryEventsParams};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), allsource::Error> {
    let client = Client::new("http://localhost:3902", "your-api-key")?;

    // Check health
    let health = client.health().await?;
    println!("Status: {}", health.status);

    // Ingest an event
    let event = client.ingest_event(IngestEventInput {
        event_type: "user.signup".into(),
        entity_id: "user-123".into(),
        payload: json!({"email": "alice@example.com", "plan": "growth"}),
        metadata: Some(json!({"source": "api", "ip": "1.2.3.4"})),
    }).await?;
    println!("Ingested: {} at {}", event.id, event.timestamp);

    // Query events for an entity
    let result = client.query_events(
        QueryEventsParams::new()
            .entity_id("user-123")
            .limit(10),
    ).await?;
    println!("Found {} events", result.count);

    // Get all events for an entity (shorthand)
    let events = client.get_entity_events("user-123").await?;
    for e in &events.data {
        println!("  {} — {}", e.event_type, e.timestamp);
    }

    // Time-range query
    let recent = client.query_events(
        QueryEventsParams::new()
            .event_type("order.placed")
            .since("2026-02-01T00:00:00Z")
            .until("2026-02-28T23:59:59Z")
            .limit(100),
    ).await?;
    println!("Orders in Feb: {}", recent.count);

    Ok(())
}
```

## API

| Method | Description |
|--------|-------------|
| `Client::new(base_url, api_key)` | Create client with defaults (30s timeout, rustls) |
| `Client::with_config(config)` | Create client with custom config |
| `client.ingest_event(input)` | Ingest a single event |
| `client.query_events(params)` | Query events with filters |
| `client.get_entity_events(id)` | Get all events for an entity |
| `client.get_events_by_type(type)` | Get all events of a type |
| `client.list_streams()` | List distinct entity IDs |
| `client.list_event_types()` | List distinct event types |
| `client.health()` | Health check |

## Features

- `rustls` (default) — Use rustls for TLS
- `native-tls` — Use platform-native TLS

## License

[MIT](../../LICENSE)
