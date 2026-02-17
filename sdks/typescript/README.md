# @allsource/client

TypeScript/JavaScript client for the [AllSource](https://all-source.xyz) event store API.

## Installation

```bash
npm install @allsource/client
# or
bun add @allsource/client
```

## Quick Start

```typescript
import { AllSourceClient } from "@allsource/client";

const client = new AllSourceClient({
  baseUrl: "https://allsource-query.fly.dev",
  apiKey: "your-api-key",
});

// Ingest an event
const event = await client.ingestEvent({
  event_type: "user.signup",
  entity_id: "user-abc-123",
  payload: { email: "jane@example.com", plan: "pro" },
  metadata: { source: "web", ip: "1.2.3.4" },
});
console.log("Ingested:", event.id);

// Query events
const { events, count } = await client.queryEvents({
  entity_id: "user-abc-123",
  limit: 50,
});
console.log(`Found ${count} events`);

// Health check
const health = await client.getHealth();
console.log("Status:", health.status);
```

## API

### `new AllSourceClient(config)`

| Option    | Type     | Required | Default | Description                      |
|-----------|----------|----------|---------|----------------------------------|
| `baseUrl` | `string` | Yes      | —       | AllSource Query Service URL      |
| `apiKey`  | `string` | Yes      | —       | API key (sent as `X-API-Key`)    |
| `timeout` | `number` | No       | `30000` | Request timeout in milliseconds  |

### `client.ingestEvent(event)`

Ingest a single event.

```typescript
await client.ingestEvent({
  event_type: "order.placed",
  entity_id: "order-456",
  payload: { total: 99.99, currency: "USD" },
});
```

### `client.queryEvents(params?)`

Query events with optional filters. Returns `{ events, count }`.

| Param        | Type     | Description                |
|--------------|----------|----------------------------|
| `entity_id`  | `string` | Filter by entity ID        |
| `event_type` | `string` | Filter by event type       |
| `limit`      | `number` | Max events to return       |
| `offset`     | `number` | Number of events to skip   |
| `start_time` | `string` | Start time (ISO 8601)      |
| `end_time`   | `string` | End time (ISO 8601)        |

### `client.getHealth()`

Returns the service health status.

### Error Handling

All API errors throw `AllSourceError` with `status` and `body` properties:

```typescript
import { AllSourceClient, AllSourceError } from "@allsource/client";

try {
  await client.ingestEvent({ ... });
} catch (err) {
  if (err instanceof AllSourceError) {
    console.error(`API error ${err.status}:`, err.body);
  }
}
```

## License

MIT
