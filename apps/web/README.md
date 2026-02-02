---
title: "Web Demo"
status: CURRENT
last_updated: 2026-02-02
category: service
port: 3000
technology: TypeScript
---

# AllSource Event Store - Web Demo

> A comprehensive, interactive demo showcasing all features of the AllSource Event Store.

## Quick Start

```bash
# Install dependencies
bun install

# Start development server
bun run dev
```

Visit **http://localhost:3000/demo** to explore the interactive demo.

## Features Demonstrated

### 1. Event Management
- ✅ Single and batch event ingestion
- ✅ Event querying by entity ID and event type
- ✅ Real-world scenarios: E-commerce and IoT events
- ✅ Event metadata and versioning

### 2. Queries & Streaming
- ✅ Query by entity ID (aggregate history)
- ✅ Query by event type
- ✅ Time-range queries
- ✅ As-of queries (time-travel)
- ✅ Real-time WebSocket streaming

### 3. Projections
- ✅ 5 types: EntitySnapshot, EventCounter, TimeSeries, Funnel, Custom
- ✅ Lifecycle management (Created → Running → Paused)
- ✅ Statistics and monitoring

### 4. Schema Management
- ✅ JSON Schema registration with versioning
- ✅ 4 Compatibility modes: None, Backward, Forward, Full
- ✅ Event validation

### 5. Analytics
- ✅ Frequency analysis (time-bucketed)
- ✅ Event correlation detection
- ✅ Statistical summaries

### 6. Security & Multi-tenancy
- ✅ Multi-tenant isolation
- ✅ RBAC with 4 roles
- ✅ JWT authentication & API keys
- ✅ Rate limiting & encryption

### 7. Monitoring & Metrics
- ✅ Real-time metrics visualization
- ✅ 18+ metric types
- ✅ Health checks

## Project Structure

```
apps/web/
├── src/
│   ├── app/
│   │   ├── demo/page.tsx         # Main comprehensive demo
│   │   ├── ui-test/page.tsx      # UI component testing
│   │   └── page.tsx              # Homepage
│   │
│   └── lib/event-store/
│       ├── types.ts              # TypeScript types
│       ├── client.ts             # API client
│       └── demo-data.ts          # Sample data generators
│
└── package.json
```

## Demo Sections

| Section | Features |
|---------|----------|
| **Overview** | Feature highlights, performance metrics (469K events/sec) |
| **Event Management** | E-commerce & IoT event generation, querying |
| **Queries & Streaming** | Entity queries, time-travel, WebSocket setup |
| **Projections** | Creating & monitoring materialized views |
| **Schema Management** | JSON Schema registration, versioning |
| **Analytics** | Frequency analysis, correlations, statistics |
| **Security** | Multi-tenancy, RBAC, authentication |
| **Monitoring** | Real-time metrics dashboard |

## API Integration

```typescript
import { eventStoreClient } from "@/lib/event-store/client";

// Create events
const events = await eventStoreClient.createEventBatch([...]);

// Query events
const results = await eventStoreClient.queryEvents({
  entity_id: "order-123",
  event_type: "OrderPlaced"
});

// Create projection
const projection = await eventStoreClient.createProjection({
  name: "Order Counter",
  projection_type: "EventCounter",
  status: "Created",
  config: { batch_size: 100 }
});

// Get metrics
const metrics = await eventStoreClient.getMetrics();
```

## Environment Variables

Create `.env.local`:

```env
NEXT_PUBLIC_EVENT_STORE_URL=http://localhost:3900
NEXT_PUBLIC_CONTROL_PLANE_URL=http://localhost:3901
```

## Tech Stack

- **Framework**: Next.js 15.3.5 (React 19)
- **Language**: TypeScript 5.8.3
- **Styling**: Tailwind CSS 4.1.11
- **UI Components**: @allsource/ui
- **Code Quality**: Biome.js 1.9.4
- **Runtime**: Bun 1.1.29

## Development Commands

```bash
bun run dev          # Start dev server
bun run build        # Build for production
bun run type-check   # TypeScript check
bun run lint         # Lint code
bun run format       # Format code
```

## Performance Highlights

- **469K events/sec** - High-throughput ingestion
- **<100ms latency** - Fast query response times
- **60-80% compression** - Efficient storage
- **1000s of connections** - Scalable WebSocket streaming
