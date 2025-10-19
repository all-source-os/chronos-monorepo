# AllSource

> **The AI-Native Event Store** - Where traditional data infrastructure stores data, AllSource understands data through time.

<div align="center">

```
   ╔═══════════════════════════════════════════════════════╗
   ║                                                       ║
   ║    🌟  A L L S O U R C E                             ║
   ║                                                       ║
   ║    The Single Source of Truth for All Events         ║
   ║                                                       ║
   ╚═══════════════════════════════════════════════════════╝
```

**Event Sourcing** × **Columnar Analytics** × **AI Context Orchestration**

</div>

---

## 🚀 Quick Start (Demo Ready in 3 Steps)

### Prerequisites

- **Rust** (1.75+) - [Install](https://rustup.rs/)
- **Go** (1.22+) - [Install](https://go.dev/dl/)
- **Bun** (1.0+) - [Install](https://bun.sh/)

### 1. Install Dependencies

```bash
# Install dependencies
bun install

# Install Go dependencies (control plane)
cd services/control-plane && go mod download && cd ../..
```

### 2. Start All Services

```bash
# Terminal 1: Start Rust Event Store Core
cd services/core
cargo run --release

# Terminal 2: Start Go Control Plane
cd services/control-plane
go run main.go

# Terminal 3: Start Web UI
cd apps/web
bun dev
```

### 3. Open Demo Dashboard

Open [http://localhost:3000](http://localhost:3000) in your browser.

---

## 🎯 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Web UI (Next.js)                        │
│                   http://localhost:3000                     │
└─────────────────────┬───────────────────────────────────────┘
                      │
         ┌────────────┴────────────┐
         │                         │
         ▼                         ▼
┌────────────────────┐   ┌────────────────────┐
│  Control Plane     │   │   Event Store      │
│     (Go)           │   │     (Rust)         │
│  localhost:8081    │   │  localhost:8080    │
│                    │   │                    │
│  • Cluster Mgmt    │   │  • Event Ingest    │
│  • Orchestration   │   │  • Time Travel     │
│  • Snapshots       │   │  • Columnar Store  │
└────────────────────┘   └────────────────────┘
         │                         │
         └────────────┬────────────┘
                      │
                      ▼
            ┌─────────────────────┐
            │   MCP Server        │
            │   (TypeScript)      │
            │                     │
            │  AI Query Interface │
            └─────────────────────┘
```

---

## 💡 Core Features (Demo Highlights)

### 1. **High-Performance Event Ingestion**
- **Rust-powered** columnar storage (Arrow/Parquet ready)
- **1M+ events/sec** potential (demo shows immediate ingestion)
- Built-in indexing by entity and event type

### 2. **Time-Travel Queries**
- Query any entity state **as of any timestamp**
- Reconstruct complete history from event stream
- Perfect for debugging, compliance, and AI training

### 3. **AI-Native MCP Interface** ⭐ DEMO HIGHLIGHT

- **11 powerful MCP tools** for temporal querying
- Natural language: *"What did user-123 look like last week?"*
- **Time-travel analysis**: Compare states across time
- **Pattern detection**: Find anomalies automatically
- **Change tracking**: See exactly what changed and when
- **Entity comparison**: Compare multiple entities instantly
- Works with Claude Desktop out of the box

**Try it:**
```bash
# Setup Claude Desktop MCP (one-time)
See packages/mcp-server/CLAUDE_DESKTOP_SETUP.md

# Then just ask Claude:
"Show me all events for user-123"
"What changed this week?"
"Find unusual patterns"
```

### 4. **Control Plane Orchestration**
- Go-based cluster management
- Real-time metrics and health monitoring
- Snapshot and replay operations

---

## 🎪 Demo Script for Presentations

### Act 1: Show the Power of Event Sourcing

1. **Open the dashboard** at `http://localhost:3000`
2. **Click "Ingest Demo Event"** several times
3. **Show the stats updating** in real-time:
   - Total events increasing
   - New entities being tracked
   - Event types accumulating

### Act 2: Demonstrate Time-Travel

1. **Filter by entity ID** (shown on the demo button)
2. **Show complete event history** for that entity
3. **Expand an event** to show the full payload
4. **Explain**: "This is how we reconstruct state at any point in time"

### Act 3: Query Flexibility

1. **Filter by event type** (e.g., `user.created`)
2. **Show instant results** thanks to columnar indexing
3. **Highlight**: "This is vectorized query performance in action"

### Act 4: API Integration

Open a new terminal and demonstrate the REST API:

```bash
# Ingest an event via API
curl -X POST http://localhost:8080/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "payment.processed",
    "entity_id": "order-789",
    "payload": {
      "amount": 99.99,
      "currency": "USD",
      "status": "success"
    }
  }'

# Query events
curl "http://localhost:8080/api/v1/events/query?entity_id=order-789"

# Get entity state reconstruction
curl "http://localhost:8080/api/v1/entities/order-789/state"

# Check stats
curl http://localhost:8080/api/v1/stats

# Check cluster status (via control plane)
curl http://localhost:8081/api/v1/cluster/status
```

### Act 5: MCP for AI - THE WOW FACTOR ⭐

**Say:** "But here's what makes AllSource truly revolutionary..."

**Open Claude Desktop** (with MCP connected)

**Ask Claude in natural language:**
- "Show me all events for user-123"
- "What did user-123 look like yesterday?"
- "What changed for user-123 this week?"
- "Find patterns in user events"
- "Explain everything about user-123"

**Point out:**
- Zero code written
- Natural language only
- Instant temporal analysis
- AI understands time relationships

**Say:** "This is the first event store that LLMs can query natively. We're not just storing events—we're making time itself queryable by AI."

See [packages/mcp-server/DEMO_SCRIPT.md](packages/mcp-server/DEMO_SCRIPT.md) for the complete MCP demo script.

---

## 📊 Technology Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| **Event Store Core** | Rust | Near-zero latency, memory safety, SIMD, Arrow/Parquet |
| **Control Plane** | Go | Fast iteration, Kubernetes ecosystem, massive community |
| **MCP Interface** | TypeScript | AI-native protocol, auto-generated SDKs via Speakeasy |
| **Web UI** | Next.js 14 | Modern React, server components, beautiful UX |
| **Storage** | In-Memory (Demo) | Production: Arrow/Parquet columnar files |
| **Build System** | Turborepo + Bun | Monorepo orchestration, blazing fast builds |

---

## 🔥 Key Differentiators

### vs Traditional Databases
- **AllSource**: Immutable event log with time-travel
- **Traditional**: Mutable state, no historical context

### vs Event Stores (Kafka, EventStoreDB)
- **AllSource**: AI-native, columnar analytics, MCP interface
- **Others**: Message queues, no native AI integration

### vs Data Warehouses (Snowflake)
- **AllSource**: Real-time event ingestion, microsecond queries
- **Warehouses**: Batch processing, delayed insights

---

## 🎯 Target Market: Pre-IPO Tech Companies

| Pain Point | AllSource Solution |
|------------|-------------------|
| Fragmented data across microservices | Unified event stream |
| AI models trained on inconsistent data | Single temporal source of truth |
| Compliance and audit trails | Immutable event log with snapshots |
| "Why did this change?" questions | Complete event history with time-travel |

---

## 🛠️ Development Commands

```bash
# Install all dependencies
bun install

# Run all services in dev mode
bun dev

# Build everything
bun build

# Clean all build artifacts
bun clean

# Format code
bun format

# Run tests
bun test
```

---

## 📁 Project Structure

```
allsource-monorepo/
├── apps/
│   └── web/                    # Next.js dashboard (port 3000)
├── packages/
│   └── mcp-server/             # Model Context Protocol server
├── services/
│   ├── core/                   # Rust event store (port 8080)
│   └── control-plane/          # Go orchestration (port 8081)
├── package.json                # Root monorepo config (with Bun workspaces)
├── turbo.json                  # Turborepo pipeline
└── README.md                   # You are here
```

---

## 🌟 Demo Talking Points

### Opening Hook
> "Imagine if every change in your system was stored, queryable, and available for AI to reason about. That's AllSource."

### Technical Depth
> "Built in Rust for performance, orchestrated by Go for scale, and wrapped in TypeScript for AI integration. We're leveraging the best of each ecosystem."

### The Vision
> "AllSource isn't just storing events — it's making time itself queryable. Your entire business history becomes a training ground for intelligent systems."

### Market Positioning
> "Think of us as the Snowflake for events, plus a time machine for data, plus a brain for AI — all in one platform."

---

## 🚢 What's Next

### Immediate (v0.2)
- [ ] Persistent storage with Parquet files
- [ ] Multi-node cluster support
- [ ] GraphQL API layer
- [ ] Real-time event streaming to web UI

### Near-term (v0.5)
- [ ] WASM plugin SDK for custom event processors
- [ ] Kubernetes operator for auto-scaling
- [ ] Advanced time-series analytics
- [ ] LLM fine-tuning on event data

### Future (v1.0)
- [ ] Blockchain-based event notarization
- [ ] Federated query across multiple AllSource instances
- [ ] Predictive replay engine for "what-if" scenarios
- [ ] Enterprise SaaS offering

---

## 📝 License

MIT License - Built with ❤️ for the future of data infrastructure

---

## 🙏 Acknowledgments

Inspired by:
- **Midday.ai** - Monorepo architecture patterns
- **Apache Arrow** - Columnar data format
- **Model Context Protocol** - AI-native interfaces
- **Event Sourcing** - Greg Young, Martin Fowler

---

<div align="center">

**AllSource** - *Where all truth flows from*

⭐ Star this repo if you believe in AI-native infrastructure ⭐

</div>
