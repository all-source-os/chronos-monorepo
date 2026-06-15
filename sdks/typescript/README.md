# @allsourcedev/client

TypeScript/JavaScript client for the [AllSource](https://all-source.xyz) event store API.

Talks to the AllSource **Query Service** (the public gateway), not Core directly.
Ships compiled ESM + CJS with `.d.ts` types. Runs on Node 18+ and Bun. Zero
runtime dependencies (uses the global `fetch`).

## Installation

```bash
npm install @allsourcedev/client
# or
bun add @allsourcedev/client
# or
pnpm add @allsourcedev/client
```

> **Why not a git install?** `@allsourcedev/client` lives in the `all-source`
> monorepo under `sdks/typescript`. Git installers can't pull a sub-directory of
> a monorepo — that's why `bun add github:…#workspace=@allsourcedev/client` and
> `#path:sdks/typescript` return 404. Install from the npm registry (above).

## Authentication

AllSource uses API keys (signed JWTs). Get one from your dashboard at
[all-source.xyz](https://all-source.xyz) → **Settings → API Keys**. The key is
sent in the `X-API-Key` header; the SDK does this for you when you pass `apiKey`
to the constructor.

Store the key in an env var rather than hard-coding it:

```typescript
import { AllSourceClient } from "@allsourcedev/client";

const client = new AllSourceClient({
  baseUrl: "https://allsource-query.fly.dev", // your Query Service URL
  apiKey: process.env.ALLSOURCE_API_KEY!,
});
```

The tenant is taken from the key — there is no separate tenant argument.

## Quick Start

```typescript
import { AllSourceClient } from "@allsourcedev/client";

const client = new AllSourceClient({
  baseUrl: "https://allsource-query.fly.dev",
  apiKey: process.env.ALLSOURCE_API_KEY!,
});

// Ingest an event — returns the stored event
const event = await client.ingestEvent({
  event_type: "user.signup",
  entity_id: "user-abc-123",
  payload: { email: "jane@example.com", plan: "pro" },
  metadata: { source: "web", ip: "1.2.3.4" },
});
console.log("Ingested:", event.id);

// Query events back
const { events, count } = await client.queryEvents({
  entity_id: "user-abc-123",
  limit: 50,
});
console.log(`Found ${count} events`);

// Health check (unauthenticated)
const health = await client.getHealth();
console.log("Status:", health.status);
```

## API

### `new AllSourceClient(config)`

| Option           | Type       | Required | Default | Description                          |
|------------------|------------|----------|---------|--------------------------------------|
| `baseUrl`        | `string`   | Yes      | —       | AllSource Query Service URL          |
| `apiKey`         | `string`   | Yes      | —       | API key (sent as `X-API-Key`)        |
| `timeout`        | `number`   | No       | `30000` | Request timeout in milliseconds      |
| `retry`          | `object`   | No       | —       | Retry/backoff overrides              |
| `circuitBreaker` | `object`   | No       | —       | Circuit-breaker overrides            |
| `fetch`          | `function` | No       | global  | Custom `fetch` implementation        |

### `client.ingestEvent(event)` → `Event`

Ingest a single event and return the stored event (`id`, `timestamp`, …).

```typescript
const stored = await client.ingestEvent({
  event_type: "order.placed",
  entity_id: "order-456",
  payload: { total: 99.99, currency: "USD" },
  metadata: { source: "checkout" }, // optional, preserved
});
```

### `client.ingestBatch(events)` → `{ count, events }`

Ingest many events in one request.

### `client.queryEvents(params?)` → `{ events, count }`

Query events with optional filters.

| Param        | Type     | Description                       |
|--------------|----------|-----------------------------------|
| `entity_id`  | `string` | Filter by entity ID               |
| `event_type` | `string` | Filter by event type              |
| `limit`      | `number` | Max events to return              |
| `offset`     | `number` | Number of events to skip          |
| `since`      | `string` | Start time, inclusive (ISO 8601)  |
| `until`      | `string` | End time, inclusive (ISO 8601)    |

### `client.queryAndFold(params, folder)` → `S | undefined`

Query events and fold them into a derived state with an `EventFolder<S>`.

### `client.listProjections()` → `{ projections, total }`

List projections from Core.

### Prime projections

`definePrimeProjection(entityType, fieldPolicies)`,
`projectNode(nodeId)`, `listPrimeProjections()`,
`nodeFieldProvenance(nodeId, field)` — declarative projections with per-field
merge policies and provenance.

### `client.getHealth()` → `HealthResponse`

Service health. Unauthenticated.

### Reliability

Every request goes through exponential-backoff retries (on 408/429/5xx and
network errors) and a circuit breaker. Both are configurable via the constructor.

### Error handling

API errors throw `AllSourceError` with `status` and `body`:

```typescript
import { AllSourceClient, AllSourceError } from "@allsourcedev/client";

try {
  await client.ingestEvent({ /* … */ });
} catch (err) {
  if (err instanceof AllSourceError) {
    console.error(`API error ${err.status}:`, err.body);
    if (err.isUnauthorized()) console.error("Check your API key.");
  }
}
```

## License

MIT
