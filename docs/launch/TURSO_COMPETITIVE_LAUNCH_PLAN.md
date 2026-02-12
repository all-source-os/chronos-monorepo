# AllSource Launch Plan: Competitive Positioning Against Turso

**Date**: 2026-02-12
**Status**: ACTIONABLE
**Based on**: [AllSource vs Turso Comparison](../roadmaps/ALLSOURCE_VS_TURSO_COMPARISON.md)

---

## Executive Summary

Turso has established strong positioning in the "AI-native database" space with aggressive pricing ($4.99/mo) and broad SDK support. AllSource must differentiate on **temporal intelligence** and **event sourcing**—capabilities Turso fundamentally cannot offer.

---

## Immediate Actions (This Week)

### 1. Messaging Updates

| Current | Change To | Rationale |
|---------|-----------|-----------|
| "AI-native event store" | "Temporal intelligence platform" | Avoid head-to-head with Turso's "AI-native" |
| "Store events" | "Remember everything" | Emotional, differentiating |
| "Query your data" | "Time-travel your data" | ✅ Already updated |
| "Event database" | "Event store with perfect memory" | Avoid "database" (Turso's turf) |

### 2. Hero Section Updates (Web)

**Current**: "Time-travel your data. Let AI agents manage it."

**Recommended update**:
```
Time-travel your data.

Query any point in history. Replay any sequence.
Give your AI agents perfect memory.

469K events/sec • 11.9μs latency • 27 MCP tools
```

### 3. Add Benchmark Section

Create a prominent benchmarks display showing:
- **469,000 events/sec** ingestion
- **11.9μs** p99 query latency
- **27 MCP tools** for AI integration
- **~129MB** memory footprint

*Why*: Turso doesn't publish performance numbers. Our benchmarks are a moat.

### 4. Feature Highlights (Reframe)

| Generic Feature | Turso-Differentiating Frame |
|-----------------|----------------------------|
| Event storage | **Perfect memory** - never lose a change |
| Query API | **Time-travel** - query any point in history |
| Multi-tenant | **Compliance-ready** - immutable audit trails |
| AI integration | **27 MCP tools** - not just SQL access |
| Performance | **469K/sec** - benchmark-proven |

---

## Web App Changes

### Priority 0 (This Sprint)

1. **Update hero subtitle** to emphasize time-travel and benchmarks
2. **Add benchmarks component** - prominent performance metrics
3. **MCP tools section** - showcase the 27 AI integration tools
4. **Remove "database" language** - we're an "event store"

### Priority 1 (Next Sprint)

1. **Use cases page** focusing on:
   - Audit trails & compliance
   - Event replay & debugging
   - AI agent memory
   - Financial transaction history

2. **Comparison page** - AllSource vs EventStoreDB (our actual competitor)
   - Don't create AllSource vs Turso (validates them as competitor)

3. **Interactive demo** - Time-travel query playground

### Priority 2 (Month 2)

1. **Case studies** - Early adopter stories
2. **Integration guides** - MCP + Claude/GPT setup
3. **SDK documentation** - When JS/Python SDKs ready

---

## Pricing Strategy

### Current vs Recommended

| Tier | Current | Recommended | Rationale |
|------|---------|-------------|-----------|
| Free | 10K events | **50K events** | More generous than Turso's effective free |
| Pro | $29/mo | $29/mo | Keep, justify with features |
| Team | $99/mo | $79/mo | Closer to Turso's Scaler |
| Scale | $299/mo | $199/mo | More competitive |

### Messaging on Price

Don't compete on price. Justify premium with:
- "Time-travel queries included at every tier"
- "Immutable audit trail (compliance-ready)"
- "27 MCP tools for AI integration"
- "No per-query charges for time-travel"

---

## SDK Priority

Based on Turso's SDK coverage gap analysis:

| SDK | Turso | AllSource | Priority | Timeline |
|-----|-------|-----------|----------|----------|
| JavaScript | ✅ | ❌ | **P0** | Week 1-2 |
| Python | ✅ | ❌ | **P0** | Week 2-3 |
| Go | ✅ | ❌ | P1 | Month 2 |
| Rust | ✅ | ❌ | P2 | Month 3 |

**Minimum viable launch**: JS + Python SDKs

---

## Marketing Channels

### Where Turso Is Strong (Avoid Direct Competition)

- Edge/serverless communities (Cloudflare, Vercel)
- Mobile development (React Native, Flutter)
- SQLite enthusiasts

### Where AllSource Should Focus

| Channel | Message | Why |
|---------|---------|-----|
| **Event sourcing community** | "Finally, event sourcing without the complexity" | Our natural audience |
| **CQRS practitioners** | "Built-in projections, not bolted-on" | Technical differentiation |
| **Compliance/FinTech** | "Immutable audit trails, query any point in time" | Turso can't match |
| **AI/ML engineers** | "Give your agents memory, not just storage" | MCP angle |
| **Elixir/Phoenix community** | "Native integration, real-time subscriptions" | Technical fit |

### Content Strategy

| Content Type | Topic | Differentiator |
|--------------|-------|----------------|
| Blog post | "Why Event Sourcing in 2026" | Establish category |
| Blog post | "Time-Travel Queries Explained" | Unique capability |
| Tutorial | "Build an Audit Trail in 10 Minutes" | Practical value |
| Video | "AllSource + Claude MCP Demo" | AI integration |
| Benchmark | "469K Events/Sec: How We Got There" | Performance proof |

---

## Competitive Landmines (What NOT to Do)

| Don't | Why |
|-------|-----|
| Claim "AI-native database" | Turso owns this term |
| Compete on price | We'll lose; compete on value |
| Build vector search | Partner with LanceDB instead |
| Target edge/mobile | Not our architecture |
| Create Turso comparison page | Validates them as competitor |
| Mention Turso in marketing | Stay in our lane |

---

## Success Metrics

### Week 1
- [ ] Hero section updated with benchmarks
- [ ] MCP tools section added
- [ ] "Database" language removed

### Month 1
- [ ] JavaScript SDK released
- [ ] Python SDK released
- [ ] 3 blog posts published
- [ ] 50 signups from event sourcing community

### Month 3
- [ ] 10 paying customers
- [ ] 2 case studies published
- [ ] Benchmark page with reproducible tests
- [ ] Featured in 1 event sourcing newsletter

---

## Talking Points (For Team)

### If asked about Turso

> "Turso is great for edge databases and mobile apps. We're focused on event sourcing—applications that need to remember every change and query any point in history. Different tools for different problems."

### If asked about price difference

> "Turso optimizes for lots of small databases. We optimize for deep event history with time-travel queries. Our pricing reflects the value of never losing data and always being able to audit."

### If asked about vector search

> "We're partnering with specialized vector databases like LanceDB rather than building our own. Our focus is temporal queries—combining that with vector search for 'temporal RAG' is on our roadmap."

---

## Implementation Checklist

### Web App (Immediate)

- [ ] Update hero section with benchmark stats
- [ ] Add "How It Works" section emphasizing time-travel
- [ ] Update features section with Turso-differentiating language
- [ ] Add MCP tools showcase
- [ ] Remove/replace "database" with "event store"

### Docs (Week 1)

- [ ] Quick start guide emphasizing time-travel queries
- [ ] MCP integration guide
- [ ] Event sourcing concepts explainer

### Marketing (Month 1)

- [ ] "Why Event Sourcing" blog post
- [ ] Time-travel query demo video
- [ ] Benchmark reproducibility guide

---

**Document Owner**: AllSource Team
**Last Updated**: 2026-02-12
