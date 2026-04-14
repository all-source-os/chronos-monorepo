# allsource-client

Python SDK for the [AllSource Event Store](https://all-source.xyz).

## Install

```bash
pip install allsource-client
```

## Authentication

AllSource uses API keys. Get one from your dashboard at [all-source.xyz](https://all-source.xyz) (Settings → API Keys) or mint one via `POST /api/v1/auth/api-keys`. The SDK passes the key in the `X-API-Key` header automatically.

```python
import os
from allsource_client import AllSourceClient

client = AllSourceClient(
    api_key=os.environ["ALLSOURCE_API_KEY"],
    base_url="https://api.all-source.xyz",
)
```

## Quickstart

```python
from allsource_client import AllSourceClient

client = AllSourceClient(api_key="as_your_key", base_url="https://api.all-source.xyz")

# Ingest an event
event = client.ingest("user.signup", "user-123", {"plan": "pro", "source": "web"})
print(f"Created event {event.id}")

# Query events
result = client.query(event_type="user.signup", limit=10)
for e in result.events:
    print(f"{e.timestamp} {e.entity_id}: {e.payload}")

# Projections
projections = client.get_projections()
projection = client.get_projection("user-count")

# Webhooks
webhook = client.create_webhook("https://example.com/hook", event_types=["user.*"])
client.delete_webhook(webhook.id)
```

## Async Usage

```python
import asyncio
from allsource_client import AllSourceAsyncClient

async def main():
    async with AllSourceAsyncClient(api_key="as_your_key") as client:
        event = await client.ingest("order.placed", "order-456", {"total": 99.99})
        events = await client.query(entity_id="order-456")
        print(f"Found {events.count} events")

asyncio.run(main())
```

## API Reference

### `AllSourceClient(api_key, base_url, timeout)`

| Method | Description |
|--------|-------------|
| `ingest(event_type, entity_id, data)` | Ingest a single event |
| `ingest_batch(events)` | Ingest multiple events |
| `query(event_type, entity_id, start, end, limit, offset)` | Query events |
| `get_events_by_entity(entity_id)` | Get events for an entity |
| `get_events_by_type(event_type)` | Get events by type |
| `get_projections()` | List projections |
| `get_projection(name)` | Get a projection |
| `create_projection(name, definition, version, initial_state)` | Create a projection |
| `create_webhook(url, event_types, entity_id)` | Register a webhook |
| `list_webhooks()` | List webhooks |
| `delete_webhook(webhook_id)` | Delete a webhook |
| `get_webhook_deliveries(webhook_id)` | Get delivery history |
| `health()` | Check API health |

`AllSourceAsyncClient` has the same methods, all returning coroutines.

## Development

```bash
pip install -e ".[dev]"
pytest
mypy src/
```
