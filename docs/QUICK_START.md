# Quick Start

Get started with AllSource in under 5 minutes using curl.

## Prerequisites

You need an API key. Get one by signing up at [allsource-query.fly.dev](https://allsource-query.fly.dev) or via the onboarding endpoint:

```bash
curl -s https://allsource-query.fly.dev/api/v1/onboard/start \
  -H "Content-Type: application/json" \
  -d '{"email": "you@example.com"}' | jq
```

Set your API key for the examples below:

```bash
export ALLSOURCE_API_KEY="your-api-key-here"
export ALLSOURCE_URL="https://allsource-query.fly.dev"
```

> **Local development?** If running locally with Docker, use `http://localhost:3902` for the Query Service or `http://localhost:3900` for Core directly.

---

## 1. Ingest an Event

Store your first event. Every event has an `event_type`, an `entity_id` (the thing it happened to), and a `payload` with your data:

```bash
curl -s "$ALLSOURCE_URL/api/events" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" \
  -d '{
    "event_type": "user.signup",
    "entity_id": "user-123",
    "payload": {
      "email": "alice@example.com",
      "plan": "pro",
      "source": "landing_page"
    },
    "metadata": {
      "ip": "203.0.113.42",
      "user_agent": "Mozilla/5.0"
    }
  }' | jq
```

Response:

```json
{
  "event_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "timestamp": "2026-02-15T10:30:00.000Z"
}
```

Add a few more events to have data to query:

```bash
# Order placed
curl -s "$ALLSOURCE_URL/api/events" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" \
  -d '{
    "event_type": "order.placed",
    "entity_id": "order-456",
    "payload": {
      "user_id": "user-123",
      "total": 99.99,
      "items": ["widget-a", "widget-b"]
    }
  }' | jq

# Payment processed
curl -s "$ALLSOURCE_URL/api/events" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" \
  -d '{
    "event_type": "payment.processed",
    "entity_id": "order-456",
    "payload": {
      "amount": 99.99,
      "currency": "USD",
      "method": "card",
      "stripe_id": "pi_abc123"
    }
  }' | jq
```

---

## 2. Query Events

### List all events

```bash
curl -s "$ALLSOURCE_URL/api/events" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

### Filter by entity

Get the full history of a specific entity:

```bash
curl -s "$ALLSOURCE_URL/api/events?entity_id=order-456" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

### Filter by event type

Find all events of a specific type:

```bash
curl -s "$ALLSOURCE_URL/api/events?event_type=user.signup" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

### Paginate results

```bash
curl -s "$ALLSOURCE_URL/api/events?limit=10&offset=0" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

Response:

```json
{
  "events": [
    {
      "id": "a1b2c3d4-...",
      "event_type": "user.signup",
      "entity_id": "user-123",
      "payload": { "email": "alice@example.com", "plan": "pro", "source": "landing_page" },
      "metadata": { "ip": "203.0.113.42", "user_agent": "Mozilla/5.0" },
      "timestamp": "2026-02-15T10:30:00.000Z"
    }
  ],
  "count": 1
}
```

---

## 3. Create a Projection

Projections are materialized views that automatically update as new events arrive. Save and retrieve per-entity state:

```bash
# Save projection state for an entity
curl -s "$ALLSOURCE_URL/api/projections" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" \
  -d '{
    "name": "order_summary",
    "entity_id": "order-456",
    "state": {
      "status": "paid",
      "total": 99.99,
      "items_count": 2,
      "events_processed": 2
    }
  }' | jq
```

### List projections

```bash
curl -s "$ALLSOURCE_URL/api/projections" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

### Get a specific projection

```bash
curl -s "$ALLSOURCE_URL/api/projections/order_summary" \
  -H "X-API-Key: $ALLSOURCE_API_KEY" | jq
```

---

## Next Steps

- **Batch ingest**: POST to `/api/events/batch` with `{"events": [...]}` for bulk loading
- **Schemas**: Register JSON schemas at `/api/schemas` to validate event payloads
- **Analytics**: Use `/api/analytics/frequency`, `/api/analytics/summary`, and `/api/analytics/correlation`
- **WebSocket streaming**: Connect to `/api/v1/events/stream` for real-time event feeds
- **API docs**: Visit `$ALLSOURCE_URL/api/docs` for the interactive Swagger UI
- **Full API reference**: See [docs/current/API_REFERENCE.md](current/API_REFERENCE.md)
