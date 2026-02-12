# Chronos Marketing Materials

**Version:** 0.9.0
**Last Updated:** 2026-02-11

---

## Key Messages

### One-liner
Open-source event sourcing with time-travel debugging

### Tagline (ProductHunt)
Store every event. Query any point in time. Debug like a time traveler.

### Elevator Pitch (30 seconds)
Chronos is an open-source event sourcing platform that lets you capture every state change in your application. Query historical data at any point in time, replay events to debug issues, and build real-time dashboards with streaming updates. Think "Git for your application data" - but with 469K events/second performance.

---

## X.com Launch Thread Draft

```
🚀 Launching Chronos - Event sourcing infrastructure for modern apps

After months of building, we're going live with early access.

What is it? An open-source platform for event sourcing that actually scales.

🧵 Thread:

---

1/ The Problem

Every app tracks state, but debugging state changes is hell.

"Why did this order fail?"
"When did the user change their email?"
"What was the account balance yesterday at 3pm?"

Traditional databases don't help. You need event sourcing.

---

2/ What Chronos Does

✅ Store every event immutably
✅ Query any point in time (time travel!)
✅ Real-time streaming to your dashboards
✅ 469K events/sec throughput
✅ Self-hosted or cloud

Built with Rust core, Elixir query layer, and React dashboard.

[GIF: Event explorer with time travel]

---

3/ The Stack

- Rust core: 15.7 MB Docker image, zero external deps
- Elixir query service: OTP-supervised, Phoenix API
- Go control plane: RBAC, auth, policy engine
- Next.js dashboard: OAuth, real-time updates

Total production footprint: ~129 MB

---

4/ Developer Experience

Create event:
```bash
curl -X POST https://api.chronos.dev/events \
  -d '{"type": "order.placed", "data": {"id": 123}}'
```

Time travel query:
```sql
SELECT * FROM orders AS OF '2026-02-10T15:00:00Z'
```

[GIF: Creating an event and seeing it in the explorer]

---

5/ Why Open Source?

Event sourcing is infrastructure. You shouldn't be locked in.

- MIT licensed
- Self-host anywhere
- Full audit trail
- No vendor lock-in

GitHub: github.com/all-source-os/chronos-monorepo

---

6/ Early Access

We're opening early access today.

Sign up, poke around, break things, tell us what sucks.

👉 chronos.allsource.dev

What would you build with time-travel for your data?

---

/end

cc @relevant_accounts
```

---

## ProductHunt Listing Draft

### Name
Chronos

### Tagline
Open-source event sourcing with time-travel debugging

### Description

**Chronos is an open-source event sourcing platform that captures every state change and lets you query any point in time.**

**The Problem**
Debugging state changes in production is painful. "What happened?" "When?" "Why?" Traditional databases can't answer these questions because they only store current state.

**The Solution**
Chronos stores every event immutably. You can:
- Query historical state at any timestamp
- Replay events to reproduce bugs
- Stream real-time updates to dashboards
- Build audit trails automatically

**Performance**
- 469,000 events/second ingestion
- 11.9μs p99 query latency
- 15.7 MB Docker image (Rust core)
- Zero external database dependencies

**Tech Stack**
- Rust core for performance
- Elixir query service with OTP supervision
- Go control plane for auth/RBAC
- Next.js dashboard with OAuth

**Self-hosted or Cloud**
MIT licensed, deploy anywhere. Or use our hosted version (coming soon).

### Key Features
1. Time-travel queries - Query state at any historical timestamp
2. Real-time streaming - WebSocket feeds for live dashboards
3. Event explorer - Visual timeline with filtering
4. API keys & RBAC - Granular access control
5. 27 MCP tools - AI-native interface via Claude Desktop

### Maker Comment

Hey ProductHunt! 👋

I'm [Name], maker of Chronos.

I built this because I was tired of debugging production issues by grepping logs. "What was the state of order #12345 at 3pm yesterday?" shouldn't require a PhD in log archaeology.

Event sourcing solves this, but existing solutions are either:
- Enterprise-priced and complex (EventStoreDB, Axon)
- Framework-specific (Rails Event Store)
- Just message queues (Kafka)

Chronos is event sourcing infrastructure that's:
- Fast (469K events/sec, Rust core)
- Developer-friendly (REST API, time-travel queries)
- Open source (MIT, self-host anywhere)

We're launching early access today. The dashboard is polished, but some features still use demo data while we finish the backend integration.

Would love your feedback on:
1. What's confusing in the onboarding?
2. What features are missing?
3. Would you use this? Why/why not?

Thanks for checking us out! 🙏

---

## HackerNews Show HN Draft

### Title
Show HN: Chronos – Open-source event sourcing with time-travel debugging

### Post

I built Chronos because debugging state changes in production shouldn't require reconstructing history from logs.

**What it is:**
An event sourcing platform that stores every state change and lets you query any point in time. Think "git log" for your application data.

**Why I built it:**
- Debugging: "What was the order state at 3pm?" → instant answer
- Audit: Every change has a timestamp and actor
- Replay: Reproduce bugs by replaying event sequence
- Real-time: Stream events to dashboards via WebSocket

**Tech choices:**
- Rust core: 469K events/sec, 15.7MB Docker image
- Elixir query layer: OTP supervision, Broadway pipelines
- Go control plane: JWT auth, RBAC
- Next.js dashboard: OAuth, real-time updates

**What it's not:**
- Not a message queue (though it can feed them)
- Not a CQRS framework (though it enables CQRS)
- Not trying to replace your database (it complements it)

**Open source:**
MIT licensed, self-host anywhere. No phone call required.

GitHub: https://github.com/all-source-os/chronos-monorepo
Try it: https://chronos.allsource.dev

Looking for feedback on:
1. Is the time-travel query syntax intuitive?
2. What's missing for your use case?
3. Would you use this alongside your existing stack?

---

## Performance Stats (for marketing)

| Metric | Value | Context |
|--------|-------|---------|
| Ingestion | 469,000 events/sec | Single node, Rust core |
| Query p99 | 11.9μs | Hot path, no validation |
| Concurrent writes | 7.98ms | 8 threads |
| Core image size | 15.7 MB | Distroless base |
| Total stack | ~129 MB | All 4 services |

---

## Feature List (for landing page)

### Core Features
- **Immutable event storage** - Every state change captured with timestamp
- **Time-travel queries** - Query state at any historical point
- **Real-time streaming** - WebSocket feeds for live updates
- **Schema registry** - JSON Schema validation for events
- **Multi-tenancy** - Isolated namespaces per tenant

### Developer Experience
- **REST API** - Simple HTTP endpoints for all operations
- **Query DSL** - SQL-like queries with time-travel
- **OAuth login** - Google and GitHub authentication
- **API keys** - Scoped keys with rotation and revocation
- **OpenAPI spec** - Auto-generated documentation

### Operations
- **Docker images** - Production-optimized, minimal footprint
- **Helm charts** - Kubernetes deployment ready
- **Prometheus metrics** - Full observability
- **RBAC** - Role-based access control
- **Audit logging** - Track all administrative actions

### AI-Native
- **27 MCP tools** - Claude Desktop integration
- **Event management** - Create, query, delete via AI
- **Dry-run mode** - Preview operations before executing
- **Audit trail** - All AI actions logged

---

## Visual Assets Needed

### Screenshots (1270x760px)
1. [ ] Dashboard overview (dark mode)
2. [ ] Event explorer with timeline
3. [ ] Live event feed streaming
4. [ ] API keys management
5. [ ] Onboarding flow

### GIFs (800x600px, <5MB)
1. [ ] Event explorer search and filter
2. [ ] Live event stream in action
3. [ ] Time-travel query execution
4. [ ] Creating an API key
5. [ ] OAuth login flow

### Demo Video (60 seconds)
Storyboard:
1. (0-10s) Login with GitHub OAuth
2. (10-20s) Dashboard overview, stats
3. (20-35s) Create an event, see it in explorer
4. (35-50s) Time-travel query demonstration
5. (50-60s) Live feed streaming, call to action

---

## Competitive Positioning

| Feature | Chronos | EventStoreDB | Kafka | Rails Event Store |
|---------|---------|--------------|-------|-------------------|
| Time-travel queries | ✅ | ✅ | ❌ | ✅ |
| Real-time streaming | ✅ | ✅ | ✅ | ❌ |
| Self-hosted | ✅ | ✅ | ✅ | ✅ |
| Open source | MIT | BSD-3 | Apache 2 | MIT |
| Ingestion rate | 469K/s | 100K/s | 1M+/s | 10K/s |
| Setup complexity | Low | Medium | High | Low |
| Cloud option | Coming | Yes | Yes | No |
| AI integration | ✅ | ❌ | ❌ | ❌ |

**Our differentiators:**
1. Rust performance with developer-friendly API
2. AI-native (MCP server with 27 tools)
3. Full-stack solution (not just storage)
4. Modern dashboard with OAuth

---

## Launch Checklist Reference

See beads epic `chronos-monorepo-2ie` for full task tracking.

**View with:**
```bash
br show chronos-monorepo-2ie
br list --parent chronos-monorepo-2ie
```
