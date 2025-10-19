# AllSource MCP Server

> AI-Native Temporal Event Store Interface via Model Context Protocol

Transform your event store into an AI-queryable knowledge base. Ask questions in natural language, time-travel through data, and get instant insights from your temporal event stream.

## 🌟 What Makes This Amazing

### Traditional Database Query
```sql
SELECT * FROM events
WHERE entity_id = 'user-123'
  AND timestamp >= '2024-01-01'
  AND timestamp <= '2024-01-18'
ORDER BY timestamp ASC;
```

### With AllSource MCP
```
You: "What did user-123 look like last week?"
AI: [Instant reconstructed state with full context]
```

---

## 🎯 MCP Tools Available

### 1. `query_events` - Flexible Event Querying
Query events with multiple filters: entity_id, event_type, time ranges, limits.

**Example Use Cases:**
- "Show me all user.created events from yesterday"
- "Find events for order-789 since last Monday"
- "Get the last 10 events"

### 2. `reconstruct_state` - Time-Travel State Reconstruction
Replay events to reconstruct entity state at any point in time.

**Example Use Cases:**
- "What did this user look like on January 1st?"
- "Show me the state of this order before it was canceled"
- "Reconstruct the invoice as it was last month"

### 3. `get_snapshot` - Fast Current State
Get the latest state without event replay (much faster).

**Example Use Cases:**
- "What's the current state of user-123?"
- "Show me the latest data for this entity"

### 4. `analyze_changes` - Temporal Diff Analysis
Compare entity state between two points in time.

**Example Use Cases:**
- "What changed for user-123 between Monday and Friday?"
- "Show me the diff from last week to now"
- "What fields were added or removed?"

### 5. `find_patterns` - Event Pattern Detection
Detect frequency patterns, event sequences, and anomalies.

**Example Use Cases:**
- "Find unusual event patterns in the last week"
- "What's the most common event sequence?"
- "Show me event frequency distribution"

### 6. `compare_entities` - Multi-Entity Comparison
Compare event histories across multiple entities.

**Example Use Cases:**
- "Compare user-123 and user-456"
- "Which entities have similar event patterns?"
- "Show differences between these orders"

### 7. `event_timeline` - Chronological Timeline
Get a formatted, easy-to-read timeline of events.

**Example Use Cases:**
- "Show me the timeline for this user"
- "What happened to order-789 chronologically?"

### 8. `explain_entity` - Comprehensive Analysis
Get everything about an entity: state, history, patterns, timeline.

**Example Use Cases:**
- "Tell me everything about user-123"
- "Explain this entity's complete history"
- "Give me a summary of all activity"

### 9. `ingest_event` - Event Creation
Create new events programmatically.

### 10. `get_stats` - Store Statistics
Get comprehensive event store metrics.

### 11. `get_cluster_status` - Cluster Health
Monitor cluster status and health.

---

## 🚀 Quick Start

### 1. Install Dependencies

```bash
cd packages/mcp-server
bun install
```

### 2. Start AllSource Services

```bash
# Terminal 1 - Core Event Store
cd services/core
cargo run --release

# Terminal 2 - Control Plane
cd services/control-plane
go run main.go
```

### 3. Test MCP Server

```bash
cd packages/mcp-server
bun dev
```

### 4. Connect Claude Desktop

See [CLAUDE_DESKTOP_SETUP.md](./CLAUDE_DESKTOP_SETUP.md) for detailed instructions.

---

## 🎪 Demo Scenarios

### Scenario 1: Customer Support

**Agent:** "A customer is asking why their order status changed. Can you help?"

**You to Claude:** "Explain everything about order-789"

**Claude responds with:**
- Complete event timeline
- All state changes
- When each change happened
- Current state vs historical states

**Value:** Instant customer context without digging through logs

---

### Scenario 2: Debugging Production Issue

**Developer:** "Users are reporting their profiles are wrong"

**You to Claude:** "Compare user-123, user-456, and user-789 event patterns"

**Claude responds with:**
- Event count comparison
- Event type differences
- Anomaly detection

**Value:** Quickly identify which user has the problem

---

### Scenario 3: Compliance Audit

**Auditor:** "We need to prove what data existed on December 31st"

**You to Claude:** "Reconstruct invoice-2023-12345 as it was on 2023-12-31 at 23:59:59"

**Claude responds with:**
- Exact state at that moment
- All events that contributed
- Full audit trail

**Value:** Instant compliance without manual research

---

### Scenario 4: Data Analysis

**Analyst:** "We need to understand user behavior patterns"

**You to Claude:** "Find patterns in user.login events from the last month"

**Claude responds with:**
- Frequency distribution
- Common event sequences
- Anomaly detection
- Peak times

**Value:** Real-time behavioral analysis

---

## 💡 Why MCP Integration is Revolutionary

### Before AllSource + MCP

1. **Write complex SQL queries** (30 mins)
2. **Join multiple tables** (frustration)
3. **Handle time zones** (bugs)
4. **Aggregate manually** (error-prone)
5. **Interpret results** (guesswork)

**Total time:** Hours

### With AllSource + MCP

1. **Ask in natural language** (5 seconds)

**Total time:** Seconds

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│         Claude Desktop / LLM            │
│                                         │
│  "What changed for user-123 yesterday?" │
└────────────────┬────────────────────────┘
                 │
                 ▼
┌────────────────────────────────────────────┐
│         AllSource MCP Server               │
│                                            │
│  • Interprets natural language             │
│  • Calls appropriate tools                 │
│  • Formats responses                       │
│  • Provides context                        │
└────────────────┬───────────────────────────┘
                 │
      ┌──────────┴──────────┐
      │                     │
      ▼                     ▼
┌──────────────┐   ┌─────────────────┐
│  Core API    │   │  Control Plane  │
│  (Rust)      │   │  (Go)           │
│              │   │                 │
│  :8080       │   │  :8081          │
└──────────────┘   └─────────────────┘
      │
      ▼
┌──────────────────────────────┐
│    Event Store               │
│  • Indexed events            │
│  • Projections               │
│  • Time-travel queries       │
└──────────────────────────────┘
```

---

## 🔧 Technical Details

### Communication Protocol

The MCP server communicates with Claude (or any MCP client) via **stdio** (standard input/output), using JSON-RPC 2.0.

### Tool Execution Flow

1. **User asks question** in natural language
2. **Claude analyzes** the question
3. **Claude decides** which MCP tool(s) to call
4. **MCP server receives** tool call via stdio
5. **MCP server queries** AllSource APIs
6. **MCP server formats** response
7. **Claude receives** structured data
8. **Claude synthesizes** natural language answer
9. **User gets** instant insight

### Performance

- **Tool latency:** <100ms (local network)
- **Query execution:** <10ms (indexed queries)
- **Time-travel reconstruction:** <50ms (typical entity)
- **Pattern analysis:** <500ms (1000s of events)

---

## 📊 Demo Metrics

Use these talking points in your presentation:

1. **Query Speed**
   - "Notice the response was instant"
   - "No database query needed"
   - "Indexed lookups in microseconds"

2. **Ease of Use**
   - "Zero SQL knowledge required"
   - "Natural language only"
   - "No training needed"

3. **Data Richness**
   - "Full temporal context"
   - "Every change tracked"
   - "Nothing is lost"

4. **AI Intelligence**
   - "Claude understands time relationships"
   - "Automatic pattern detection"
   - "Contextual explanations"

---

## 🎬 Live Demo Tips

### Setup

1. Have AllSource running BEFORE demo
2. Test MCP connection beforehand
3. Pre-populate some demo data
4. Keep terminal logs visible (shows real-time activity)

### Demo Flow

1. **Start simple** - "Show me events for user-123"
2. **Add time-travel** - "What did it look like yesterday?"
3. **Show analysis** - "What changed?"
4. **Demonstrate patterns** - "Find unusual patterns"
5. **Wow factor** - "Explain everything"

### What to Highlight

- **Speed** - "That was instant"
- **Natural language** - "No code required"
- **Temporal power** - "Time-travel through data"
- **AI understanding** - "Claude knew exactly what to query"
- **Production ready** - "This is running on real infrastructure"

---

## 🐛 Troubleshooting

### MCP Server Won't Start

```bash
# Check Bun is installed
bun --version

# Check AllSource is running
curl http://localhost:8080/health
curl http://localhost:8081/health

# Check for port conflicts
lsof -i :8080
lsof -i :8081
```

### Tool Calls Failing

- **Check entity exists** before querying
- **Use ISO timestamps** (YYYY-MM-DDTHH:mm:ssZ)
- **Verify AllSource logs** for errors
- **Test API directly** with curl first

### Claude Desktop Not Connecting

- **Restart Claude Desktop** completely
- **Check config path** is absolute
- **Verify Bun path** in config
- **Check MCP logs** in `~/Library/Logs/Claude/`

---

## 🚢 Production Considerations

### Security

- Add authentication to API endpoints
- Validate all tool inputs
- Rate limit tool calls
- Audit MCP usage

### Scalability

- MCP server is stateless (scales horizontally)
- AllSource Core handles heavy lifting
- Consider caching frequent queries
- Use read replicas for high traffic

### Monitoring

- Log all tool calls
- Track query latency
- Monitor API errors
- Alert on anomalies

---

## 📚 Resources

- [Model Context Protocol Spec](https://spec.modelcontextprotocol.io/)
- [Claude Desktop MCP Guide](https://docs.anthropic.com/claude/docs/model-context-protocol)
- [AllSource Documentation](../../README.md)

---

<div align="center">

**AllSource MCP Server** - *Where AI meets temporal data*

Built with ❤️ and Bun

</div>
