---
title: "Elixir Query Service"
status: CURRENT
last_updated: 2026-02-02
category: service
port: 3902
technology: Elixir
---

# AllSource Query Service (Elixir)

**AI-Native Event Processing & Projections on the BEAM**

Migrated from Clojure to leverage the BEAM VM's superior concurrency and OTP supervision for real-time event processing.

## 🎯 Why Elixir?

### Concurrency & Performance
- **Actor Model**: Each projection as an isolated GenServer process
- **Millions of Processes**: BEAM handles millions of lightweight processes
- **No GC Pauses**: Per-process garbage collection
- **Preemptive Scheduling**: Fair process scheduling

### Fault Tolerance (OTP)
- **Supervision Trees**: Auto-restart failed projections
- **"Let It Crash"**: Isolate failures
- **Hot Code Reloading**: Deploy without downtime

### Streaming
- **Broadway**: High-throughput pipelines
- **GenStage**: Built-in backpressure
- **Phoenix Channels**: WebSocket streaming

## 📖 Quick Start

```elixir
# Query DSL
import QueryServiceEx.Application.DSL.QueryDSL

from_events()
|> where(event_type: "order.placed")
|> since(days_ago(7))
|> limit(100)

# Projections
{:ok, pid} = ProjectionServer.start_link(
  projection: my_projection,
  entity_id: "user-123"
)
```

## 🏗️ Architecture

```
domain/           # Pure business logic
application/      # Use cases & DSL
infrastructure/   # External adapters
```

## 🚀 Features

✅ **Query DSL** with Elixir macros - Fluent, pipe-friendly query building
✅ **Projection GenServers** with OTP supervision - Real-time materialized views
✅ **Pipeline Processors** - Composable event transformation pipelines
✅ **Phoenix HTTP API** - RESTful endpoints on port 3902
✅ **Tesla HTTP Client** - Integration with Rust Core event store
✅ **Broadway Integration** - High-throughput event processing foundation
✅ **Telemetry** - Built-in observability and metrics
✅ **Docker Support** - Production-ready containerization

### Test Coverage

**242 tests passing** (7 doctests + 235 tests, 0 failures)

- Query entity: 37 tests ✅
- Projection entity: 37 tests ✅
- Query DSL: 54 tests ✅
- RustCoreClient: 34 tests ✅
- ProjectionServer: 24 tests ✅
- Pipeline entity: 57 tests ✅
- PipelineProcessor: 24 tests ✅
- Phoenix controllers: 5 tests ✅

## API Endpoints

The service runs on **http://localhost:3902**

### Events
- `GET /api/events` - List events
- `POST /api/events` - Create event
- `POST /api/events/batch` - Batch create
- `GET /api/events/entity/:id` - By entity
- `GET /api/events/type/:type` - By type

### Queries
- `POST /api/query` - Execute query (DSL or simple)

### Projections
- `GET /api/projections` - List all
- `GET /api/projections/:name` - Get details
- `POST /api/projections` - Create new

### System
- `GET /api/health` - Health check
- `GET /api/metrics` - Runtime metrics

## Development

```bash
# Install dependencies
mix deps.get

# Run tests
mix test

# Start server (port 3902)
mix phx.server

# Interactive shell
iex -S mix
```

## Production Deployment

```bash
# Build Docker image
docker build -t allsource-query-service:latest .

# Run with Docker
docker run -p 3902:3902 \
  -e RUST_CORE_URL=http://rust-core:3900 \
  -e SECRET_KEY_BASE=$(mix phx.gen.secret) \
  allsource-query-service:latest
```

**Built with ❤️ on the BEAM** 🦎✨

