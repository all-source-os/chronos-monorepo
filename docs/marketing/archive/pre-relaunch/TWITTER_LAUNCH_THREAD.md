# AllSource X.com Launch Thread

Copy-paste ready. Each tweet is under 280 chars and stands alone.

---

## Tweet 1 (Hook)

We built an event store where AI agents pay for their own data access.

469K events/sec. 11.9us queries. x402 micropayments on Base.

AllSource is live in early access.

https://all-source.xyz

---

## Tweet 2 (The Problem)

Every app tracks state. But debugging state changes is hell.

"Why did this order fail?"
"What was the balance at 3pm yesterday?"

Traditional databases store current state. AllSource stores every event. Query any point in time.

---

## Tweet 3 (Performance)

The Rust core is no joke:

- 469,000 events/sec ingestion
- 11.9us p99 query latency
- WAL + Parquet for full durability
- DashMap for in-memory reads

It's a real database, not an in-memory cache. Events survive restarts, crashes, everything.

---

## Tweet 4 (x402 — the differentiator)

Here's what makes AllSource different from every other event store:

Built-in x402 micropayments.

AI agents hit your API. If they're on the Pro tier, they pay per call automatically via Coinbase x402 on Base. No wallet SDK. No crypto knowledge.

Your event store pays for itself.

---

## Tweet 5 (AI-native)

43 MCP tools ship with AllSource.

Connect Claude Desktop and your agent can:
- Query events in natural language
- Build projections
- Monitor health
- Manage schemas

Event sourcing + AI agents is the combination nobody else is building for.

---

## Tweet 6 (vs alternatives)

EventStoreDB: cluster-priced, no self-serve, no AI tools.
Kafka: message broker, not a database.
Supabase: stores current state, not events.
Turso: SQLite, no event sourcing primitives.

AllSource: event store + time-travel + AI-native + x402. $0 to start.

---

## Tweet 7 (Pricing)

Pricing that doesn't require a meeting:

- Developer: Free (100K events/mo)
- Pro: $29/mo (x402 agent endpoints)
- Growth: $79/mo (10M events, unlimited streams)
- Enterprise: Custom

Open source. MIT licensed. Self-host or use our cloud.

---

## Tweet 8 (CTA)

Early access is open. Free tier is generous.

Sign up: https://all-source.xyz
API docs: https://api.all-source.xyz
Status page: https://www.all-source.xyz/status
GitHub: https://github.com/all-source-os/all-source

What would you build with time-travel for your data?
