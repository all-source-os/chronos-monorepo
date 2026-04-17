# AllSource Rust Client

Typed Rust client for the [AllSource](https://github.com/all-source-os/all-source) event store — query, ingest, fold events, and build custom projections.

## Install

```toml
[dependencies]
allsource = "0.19"  # pairs with Core ≥ 0.19.1 for full ProjectionHandle::get_state behaviour
```

## Which client do I want?

AllSource has **one public front door** (the gateway at `https://api.all-source.xyz` for our hosted offering, or your self-hosted equivalent) and **one internal fast path** (Core, reachable only on your network). The SDK exposes a client for each. Pick based on where your caller runs:

| Caller location | Use | URL shape | Why |
|---|---|---|---|
| Inside your network, same cluster/VPC as Core | `CoreClient` | `http://core.allsource.svc.cluster.local:3900` or internal DNS equivalent | One hop, ~μs key validation, no gateway overhead |
| Outside your network (public API, mobile, third-party) | `QueryClient` | `https://api.all-source.xyz` | Gateway enforces rate limits, quotas, billing |
| Mixed — internal workers + external customers | Both | internal + public URLs | Each for what it's good at |

**Core should never be on public DNS** — it's trust-the-caller by design. If you need internal services to reach Core from outside your network, use a VPN / Tailscale / bastion; don't publish `core.*` records.

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

See the [connection-path guide](https://all-source.xyz/blog/connection-path) for a deeper treatment of the direct-vs-gateway tradeoff, including latency numbers and when each pattern is the wrong default.

## Performance cheatsheet

The SDK defaults are tuned for typical use. A few things worth knowing:

- **`.clone()` is cheap.** Both `CoreClient` and `QueryClient` wrap a `reqwest::Client` in `Arc`; cloning shares the connection pool. Don't wrap them in your own `Arc<Mutex<...>>` — you'll serialize requests unnecessarily.
- **Batch writes.** `core.ingest_batch(Vec<IngestEventInput>)` is one round-trip for N events. Looping `ingest_event` is N round-trips.
- **Use `ProjectionWorker` for long-lived read models** instead of polling `QueryClient::query_events`. The worker holds a WebSocket + Core-tracked cursor, so cold starts are O(events-since-last-ack) rather than O(total-events).
- **Bootstrap the API key on Core** via `ALLSOURCE_BOOTSTRAP_API_KEY` if you go direct. Key validation becomes an in-memory DashMap hash lookup (~1μs) with zero QS roundtrip. See [AUTH_CHAIN.md](https://github.com/all-source-os/all-source/blob/main/docs/current/AUTH_CHAIN.md) for the full mechanics.
- **Tune retry / circuit breaker** via `CoreClient::with_config(ClientConfig { retry, circuit_breaker_threshold, ... })` when your upstream SLO differs from the 3-retry / 5-failure default.
- **Real-time reads** — subscribe to the WebSocket (`ProjectionWorker` or `EventStreamClient`) rather than polling. Sub-millisecond delivery after Core's broadcast vs. whatever interval you poll at.

## API keys

- **Provision** via `ALLSOURCE_BOOTSTRAP_API_KEY` on Core's first boot (idempotent — safe to leave set). Keys persist in Core's system WAL.
- **Authenticate** with `Authorization: Bearer ask_...` (preferred) or the legacy `X-API-Key` header. The SDK uses `Authorization: Bearer` automatically.
- **Validation** — Core does in-process hash lookup. Query Service adds a 120s ETS cache (SHA-256 keyed) before falling back to Core, so repeat calls through QS don't hammer Core.
- **Rotate** by provisioning a new key, updating clients, then revoking the old one. See the [AllSource API key patterns blog post](https://all-source.xyz/blog/api-keys-and-connections) for rotation without downtime.

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
