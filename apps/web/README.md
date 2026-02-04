---
title: "Web App"
status: CURRENT
last_updated: 2026-02-04
category: service
port: 3000
technology: TypeScript
---

# AllSource Event Store - Web App

> The web application for AllSource Event Store.

## Quick Start

```bash
# Install dependencies
bun install

# Start development server
bun run dev
```

Visit **http://localhost:3000** to access the app.

## Project Structure

```
apps/web/
├── src/
│   ├── app/
│   │   ├── (auth)/            # Authentication routes
│   │   ├── (marketing)/       # Marketing pages
│   │   ├── ui-test/page.tsx   # UI component testing
│   │   └── page.tsx           # Homepage
│   │
│   └── lib/event-store/
│       ├── types.ts           # TypeScript types
│       └── client.ts          # API client
│
└── package.json
```

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
