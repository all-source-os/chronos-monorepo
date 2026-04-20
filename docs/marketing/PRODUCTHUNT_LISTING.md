# AllSource ProductHunt Listing

## Name
AllSource

## Tagline (< 60 chars)
The event store that pays your AI agents with x402

## Topics
- Event Sourcing
- Developer Tools
- AI Agents
- Open Source
- Databases

## Description (< 260 chars)
Open-source event store built in Rust. 469K events/sec, 11.9us queries. Time-travel any data point. 43 MCP tools for AI agents. x402 pay-per-call monetization built in. Free tier, $29 Pro, self-host or cloud.

## Full Description

**AllSource is a purpose-built event store that lets AI agents monetize their data access through x402 micropayments.**

**The Problem**

Event sourcing is powerful but the tooling is stuck in 2015. EventStoreDB charges per cluster. Kafka is a message broker, not a database. And none of them know what an AI agent is.

**What makes AllSource different**

1. **It's a database, not a queue.** Durable WAL + Parquet storage. Your events survive restarts, crashes, and datacenter failures. 469K events/sec ingestion, 11.9us p99 query latency.

2. **AI-native from day one.** 43 MCP tools integrate directly with Claude, GPT, and any MCP-compatible agent. Query events, build projections, manage schemas, and monitor health — all through natural language.

3. **x402 agent monetization.** The first event store with built-in pay-per-call. AI agents consume priced API routes and settle payments automatically via Coinbase x402 on Base. No wallet SDK required on the agent side — server-side CDP handles everything.

4. **Time-travel queries.** Query the state of any entity at any historical timestamp. Debug production issues by replaying exactly what happened.

**The Stack**
- Rust core: WAL + Parquet + DashMap for durability + speed
- Go control plane: auth, RBAC, x402 payments, policy engine
- Elixir query service: real-time streaming, billing, rate limiting
- Next.js dashboard: OAuth, event explorer, live metrics

**Pricing**
- Developer: Free (100K events/mo)
- Pro: $29/mo (1M events, x402 agent endpoints, MCP read access)
- Growth: $79/mo billed yearly (10M events, unlimited streams)
- Enterprise: Custom

**Links**
- Website: https://all-source.xyz
- API: https://api.all-source.xyz
- Status: https://status.all-source.xyz
- GitHub: https://github.com/all-source-os/all-source

## Maker's First Comment

Hey PH! I'm Decebal, builder of AllSource.

I started this because I needed an event store for AI agent workflows — capture every action, query any point in time, and let agents pay for what they use. Existing options were either enterprise-priced (EventStoreDB), framework-specific (Rails Event Store), or just message queues pretending to be databases (Kafka).

AllSource is the event store I wished existed: fast (Rust core, 469K events/sec), durable (WAL + Parquet, not in-memory), and AI-native (43 MCP tools + x402 micropayments).

The x402 angle is what I'm most excited about. Any AI agent can call AllSource's API. If they're on the Pro tier, they pay per call automatically through Coinbase's x402 protocol — no wallet setup, no crypto knowledge needed on the agent side. It's the first event store where your data access pays for itself.

We're in early access. The free tier is generous (100K events/mo), and the whole thing is open source (MIT). I'd love to hear:

1. Does the x402 agent monetization angle make sense to you?
2. What would you store in an event store that you can't store elsewhere?
3. Anything confusing in the onboarding?

Thanks for checking us out!
