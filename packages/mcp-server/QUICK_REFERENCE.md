# AllSource MCP Tools - Quick Reference Card

> Print this and keep it handy during your demo!

---

## 🛠️ The 11 MCP Tools

### 1️⃣ `query_events` - Basic Event Query
**What:** Get events with filters
**Ask:** "Show me all events for user-123"
**Ask:** "Find user.login events from yesterday"

### 2️⃣ `reconstruct_state` - Time-Travel State
**What:** Replay events to rebuild state at any time
**Ask:** "What did user-123 look like last Monday?"
**Ask:** "Show me order-789 as it was on Jan 15th"

### 3️⃣ `get_snapshot` - Fast Current State
**What:** Get latest state (faster than reconstruction)
**Ask:** "What's the current state of user-123?"
**Ask:** "Show me the latest data for this entity"

### 4️⃣ `analyze_changes` - Temporal Diff
**What:** Compare states between two times
**Ask:** "What changed for user-123 between Monday and Friday?"
**Ask:** "Show me the diff from last week to now"

### 5️⃣ `find_patterns` - Pattern Detection
**What:** Detect frequency, sequences, anomalies
**Ask:** "Find unusual patterns in user events"
**Ask:** "What's the most common event sequence?"
**Ask:** "Show me event frequency distribution"

### 6️⃣ `compare_entities` - Multi-Entity Comparison
**What:** Compare event histories across entities
**Ask:** "Compare user-123 and user-456"
**Ask:** "Which entities have similar patterns?"

### 7️⃣ `event_timeline` - Chronological View
**What:** Formatted timeline of events
**Ask:** "Show me the timeline for user-123"
**Ask:** "What happened to order-789 chronologically?"

### 8️⃣ `explain_entity` - Comprehensive Analysis
**What:** Everything about an entity
**Ask:** "Tell me everything about user-123"
**Ask:** "Explain this entity's complete history"

### 9️⃣ `ingest_event` - Create Events
**What:** Add new events
**Ask:** "Create a user.login event for user-123"

### 🔟 `get_stats` - Store Statistics
**What:** Event store metrics
**Ask:** "What statistics do you have?"
**Ask:** "How many events are stored?"

### 1️⃣1️⃣ `get_cluster_status` - Health Check
**What:** Cluster health info
**Ask:** "What's the cluster status?"

---

## 💡 Demo Question Templates

### Time-Travel
- "What did [entity] look like [timeframe] ago?"
- "Show me [entity] as it was on [date]"
- "Reconstruct [entity] at [timestamp]"

### Change Analysis
- "What changed for [entity] this week?"
- "Compare [entity] between [date1] and [date2]"
- "Show me the diff from [time] to now"

### Pattern Detection
- "Find patterns in [entity_type] events"
- "What unusual patterns exist?"
- "Show me frequency distribution"
- "Detect anomalies in [event_type]"

### Entity Intelligence
- "Explain everything about [entity]"
- "Tell me the complete history of [entity]"
- "Compare [entity1] and [entity2]"
- "Show me the timeline for [entity]"

### Quick Queries
- "Show me all events for [entity]"
- "Get the latest state of [entity]"
- "What statistics do you have?"
- "What's the cluster status?"

---

## 🎯 Demo Flow Cheat Sheet

### 1. Start Simple (30 sec)
**Ask:** "What statistics do you have about AllSource?"
**Highlight:** Instant answer, no code

### 2. Show Events (30 sec)
**Ask:** "Show me all events for user-123"
**Highlight:** Complete event history

### 3. Time-Travel (1 min)
**Ask:** "What did user-123 look like when first created?"
**Then:** "What does user-123 look like now?"
**Highlight:** State reconstruction, historical queries

### 4. Change Analysis (1 min)
**Ask:** "What changed for user-123 since creation?"
**Highlight:** Detailed diff, perfect for debugging

### 5. Patterns (1 min)
**Ask:** "Find patterns in user events"
**Highlight:** AI-powered analysis

### 6. Wow Moment (1 min)
**Ask:** "Explain everything about [random entity]"
**Highlight:** Complete intelligence in one query

---

## 🚨 Troubleshooting Quick Fixes

| Problem | Quick Fix |
|---------|-----------|
| MCP not connected | Restart Claude Desktop |
| Tool failed | Check entity exists first |
| Slow response | Check AllSource logs |
| Wrong results | Use ISO timestamps |

---

## 📊 Key Stats to Mention

- **11 MCP tools** for temporal querying
- **Sub-100ms** tool latency
- **Natural language only** - zero SQL
- **Time-travel** - query any historical state
- **Pattern detection** - AI-powered insights
- **Rust core** - production performance

---

## 💬 Elevator Pitch

> "AllSource is the first event store that AI can query natively. Ask questions in natural language, time-travel through data, and get instant insights. No SQL, no code, just intelligence."

---

<div align="center">

**Quick Ref v1.0** - *Keep this handy!*

🌟 Remember: You're not demoing a database, you're demoing **AI-powered time-travel** 🌟

</div>
