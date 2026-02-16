# allsource-client

Python SDK for the [AllSource](https://all-source.xyz) event store.

## Installation

```bash
pip install allsource-client
```

## Quick Start

```python
from allsource import AllSourceClient, IngestEventInput

client = AllSourceClient(
    base_url="https://allsource-query.fly.dev",
    api_key="your-api-key",
)

# Ingest an event
event = client.ingest_event(IngestEventInput(
    event_type="user.signup",
    entity_id="user-123",
    payload={"email": "alice@example.com", "plan": "pro"},
    metadata={"source": "python-sdk"},
))
print(f"Created event: {event.id}")

# Query events
from allsource import QueryEventsParams

result = client.query_events(QueryEventsParams(
    entity_id="user-123",
    event_type="user.signup",
    limit=10,
))
for ev in result.events:
    print(f"{ev.timestamp} {ev.event_type} {ev.entity_id}")

# Health check
health = client.get_health()
print(f"Status: {health.status}")
```

## API Reference

### `AllSourceClient(base_url, api_key, *, timeout=30.0)`

| Parameter  | Type    | Description                            |
|-----------|---------|----------------------------------------|
| `base_url` | `str`   | Base URL of the AllSource Query Service |
| `api_key`  | `str`   | API key for authentication              |
| `timeout`  | `float` | Request timeout in seconds (default 30) |

Supports use as a context manager:

```python
with AllSourceClient(base_url="...", api_key="...") as client:
    health = client.get_health()
```

### `client.ingest_event(event) -> Event`

Ingest a single event.

```python
event = client.ingest_event(IngestEventInput(
    event_type="order.placed",
    entity_id="order-456",
    payload={"total": 99.99, "currency": "USD"},
))
```

### `client.query_events(params=None) -> QueryEventsResponse`

Query events with optional filters.

```python
result = client.query_events(QueryEventsParams(
    event_type="order.placed",
    limit=50,
    start_time="2026-01-01T00:00:00Z",
))
print(f"Found {result.count} events")
```

### `client.get_health() -> HealthResponse`

Check service health.

### Error Handling

All API errors raise `AllSourceError` with `status` and `body` attributes:

```python
from allsource import AllSourceError

try:
    client.get_health()
except AllSourceError as e:
    print(f"HTTP {e.status}: {e.body}")
```

## Type Hints

This package ships with a `py.typed` marker and full type annotations. It works with mypy, pyright, and other type checkers out of the box.
