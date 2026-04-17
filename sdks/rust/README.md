# AllSource Rust Client

Typed Rust client for the [AllSource](https://github.com/all-source-os/all-source) event store — query, ingest, fold events, and build custom projections.

## Install

```toml
[dependencies]
allsource = "0.19"
```

## Two clients

- [`QueryClient`] — reads from the Query Service (port 3902): events, projections, streams
- [`CoreClient`] — writes directly to Core (port 3900): event ingestion, projection state, durable consumers

```rust
use allsource::{QueryClient, CoreClient, IngestEventInput, QueryEventsParams};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), allsource::Error> {
    let core = CoreClient::new("http://localhost:3900", "your-api-key")?;
    let query = QueryClient::new("http://localhost:3902", "your-api-key")?;

    core.ingest_event(IngestEventInput {
        event_type: "user.signup".into(),
        entity_id: "user-123".into(),
        payload: json!({"email": "alice@example.com"}),
        metadata: None,
    }).await?;

    let events = query.query_events(
        QueryEventsParams::new().entity_id("user-123").limit(10),
    ).await?;
    println!("Found {} events", events.count);

    Ok(())
}
```

## Building custom projections

`ProjectionWorker` is a first-party worker for reducing events into domain state without reimplementing WebSocket subscription, checkpointing, dedup, and reconnection boilerplate.

```rust,no_run
use std::collections::HashMap;
use allsource::{CoreClient, Error, Event, ProjectionWorker};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct AssetState {
    symbol: String,
    altname: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let core = CoreClient::new("http://localhost:3900", "your-api-key")?;

    let worker = ProjectionWorker::<HashMap<String, AssetState>>::builder(core)
        .name("assets")                                  // durable consumer id
        .event_types(&["asset.registered", "asset.updated"])
        .reducer(|state, event: &Event| {
            if event.event_type == "asset.registered" {
                let symbol = event.payload.get("symbol")
                    .and_then(|v| v.as_str()).unwrap_or("").to_string();
                state.insert(symbol.clone(), AssetState { symbol, ..Default::default() });
            }
            Ok(())
        })
        .checkpoint_interval(100)
        .build()?;

    let handle = worker.start().await?;

    // Wait for catch-up replay to finish.
    while !handle.is_caught_up() {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Read reduced state directly.
    if let Some(asset) = handle.state().read().await.get("BTC") {
        println!("BTC: {asset:?}");
    }

    handle.stop().await?;
    Ok(())
}
```

See the runnable [`examples/asset_projection.rs`](examples/asset_projection.rs) for a full worker covering `asset.registered`, `asset.updated`, and `exchange_mapping.set` — the shape originally requested in [issue #155](https://github.com/all-source-os/all-source/issues/155).

### When to reach for what

| You want... | Use |
|---|---|
| Ad-hoc queries, one-shot reads | `QueryClient::query_events` |
| Fold events into state, one call | `QueryClient::query_and_fold` with `EventFolder` |
| A long-lived read model with live updates | **`ProjectionWorker`** |
| Server-managed projections (Entity Snapshot, Event Counter) | Query Service built-in projections |

`ProjectionWorker` is the right answer when the read model should stay warm and caught-up, when replay-on-cold-start would be O(n) expensive, or when you want live updates without polling.

### Lifecycle notes

- **Checkpointing**: the worker registers a durable consumer with Core. Core tracks the cursor server-side; the SDK `save_checkpoint` call is an `ack_consumer` under the hood. Restart → Core replays only what was added after the last ack.
- **Reconnection**: exponential backoff (100ms → 30s) on WS or HTTP error. Core's consumer replay fills any gaps when the connection returns.
- **Dedup**: per-entity version tracking in the worker. Events with `version ≤ last_applied` are skipped in case of replay overlap.
- **State push-back** (optional, off by default): enable via `.state_flush_entities(...)` to periodically bulk-write reduced state to Core's projection KV for other consumers to read.

### Cargo features

- `projection-worker` (default) — enables `ProjectionWorker` + WebSocket support
- `ws` — WebSocket client only (without the worker)
- `rustls` (default) — rustls TLS backend
- `native-tls` — platform-native TLS

To build without WebSocket support (smaller dep tree):

```toml
allsource = { version = "0.19", default-features = false, features = ["rustls"] }
```

For a deeper guide on projection design, operational concerns, and migration from polling, see [`docs/use-cases/custom-projections.md`](../../docs/use-cases/custom-projections.md).

## API surface

| Client | Method | Purpose |
|--------|--------|---------|
| `CoreClient` | `ingest_event` / `ingest_batch` | Write events |
| `CoreClient` | `get/put_projection_state`, `bulk_put_projection_state` | Projection KV |
| `CoreClient` | `register/get/ack_consumer` | Durable consumer registry |
| `CoreClient` | `save_checkpoint` / `load_checkpoint` | Worker checkpoint helpers |
| `QueryClient` | `query_events` | Filter events |
| `QueryClient` | `query_and_fold::<F: EventFolder>` | Fold → state in one call |
| `QueryClient` | `list_entities`, `detect_duplicates` | Entity discovery |
| `ProjectionWorker` | `builder(core)` | Start configuring a worker |
| `ProjectionHandle` | `state()`, `get_state(id)`, `is_caught_up()`, `stop()` | Control running worker |

## License

[MIT](../../LICENSE)
