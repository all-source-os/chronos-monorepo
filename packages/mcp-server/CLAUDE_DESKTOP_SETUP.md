# Claude Desktop Integration - AllSource MCP Server

## 🎯 Quick Setup

### 1. Locate Claude Desktop Config

The Claude Desktop config file is located at:

**macOS:**
```
~/Library/Application Support/Claude/claude_desktop_config.json
```

**Windows:**
```
%APPDATA%\Claude\claude_desktop_config.json
```

### 2. Add AllSource MCP Server

Open the config file and add the AllSource server:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "bun",
      "args": [
        "run",
        "/Users/YOUR_USERNAME/Projects/chronos/chronos-monorepo/packages/mcp-server/src/index.ts"
      ],
      "env": {
        "ALLSOURCE_CORE_URL": "http://localhost:8080",
        "ALLSOURCE_CONTROL_URL": "http://localhost:8081"
      }
    }
  }
}
```

**IMPORTANT:** Replace `/Users/YOUR_USERNAME` with your actual path!

### 3. Start AllSource Services

Before using Claude Desktop, make sure AllSource is running:

```bash
# Terminal 1 - Rust Core
cd services/core
cargo run --release

# Terminal 2 - Go Control Plane
cd services/control-plane
go run main.go
```

### 4. Restart Claude Desktop

Close and reopen Claude Desktop completely.

### 5. Verify Connection

In Claude Desktop, you should see a 🔌 icon in the bottom right. Click it to see "allsource" listed as a connected server.

---

## 🎪 Demo Conversation Examples

### Example 1: Basic Event Query

**You:** "Show me all events for user-123"

**Claude:** *Uses `query_events` tool*
```
📊 Found 5 events

{
  "events": [
    {
      "id": "...",
      "event_type": "user.created",
      "entity_id": "user-123",
      ...
    },
    ...
  ]
}
```

### Example 2: Time-Travel Query

**You:** "What did user-123 look like yesterday at 3pm?"

**Claude:** *Uses `reconstruct_state` tool with `as_of` timestamp*
```
🔄 Reconstructed state for "user-123"
📅 As of: 2024-01-18T15:00:00Z
📊 Events processed: 3
⏰ Last updated: 2024-01-18T14:30:00Z

{
  "current_state": {
    "name": "Alice",
    "role": "engineer",
    "status": "active"
  }
}
```

### Example 3: Change Analysis

**You:** "What changed for user-123 between yesterday and today?"

**Claude:** *Uses `analyze_changes` tool*
```
🔍 Change Analysis for "user-123"
📅 From: 2024-01-18T00:00:00Z
📅 To: now
➕ Added fields: 1
✏️  Modified fields: 2
➖ Removed fields: 0

{
  "added": ["department"],
  "modified": [
    {
      "field": "role",
      "before": "engineer",
      "after": "senior-engineer"
    },
    {
      "field": "salary",
      "before": 100000,
      "after": 120000
    }
  ],
  "removed": []
}
```

### Example 4: Pattern Detection

**You:** "Find unusual patterns in user events from the last week"

**Claude:** *Uses `find_patterns` tool*
```
🔎 Pattern Analysis
📊 Events analyzed: 150
🎯 Pattern type: all

{
  "frequency": [
    { "event_type": "user.login", "count": 89 },
    { "event_type": "user.updated", "count": 34 },
    { "event_type": "user.created", "count": 27 }
  ],
  "common_sequences": [
    "user.created → user.updated",
    "user.login → user.logout",
    ...
  ]
}
```

### Example 5: Entity Comparison

**You:** "Compare user-123 and user-456 to see what's different"

**Claude:** *Uses `compare_entities` tool*
```
🔬 Entity Comparison
📊 Entities compared: 2
⏰ Timeframe: all time

[
  {
    "entity_id": "user-123",
    "event_count": 45,
    "event_types": ["user.created", "user.updated", "user.login"]
  },
  {
    "entity_id": "user-456",
    "event_count": 12,
    "event_types": ["user.created", "user.login"]
  }
}
```

### Example 6: Comprehensive Explanation

**You:** "Explain everything about user-123"

**Claude:** *Uses `explain_entity` tool*
```
📋 Entity Explanation: "user-123"

🔹 Total Events: 45
🔹 Event Types: 5
🔹 Created: 2024-01-15T10:00:00Z
🔹 Last Updated: 2024-01-19T14:22:00Z

{
  "entity_id": "user-123",
  "current_state": { ... },
  "total_events": 45,
  "event_types": ["user.created", "user.updated", ...],
  "lifecycle": [
    { "when": "2024-01-15T10:00:00Z", "what": "user.created" },
    { "when": "2024-01-15T11:30:00Z", "what": "user.updated" },
    ...
  ]
}
```

---

## 🚀 Advanced Demo Scenarios

### Scenario 1: Debugging Production Issues

**You:** "Show me the timeline of events for order-789 over the last 24 hours"

**Claude:** *Uses `event_timeline` tool*

### Scenario 2: Compliance Audit

**You:** "Reconstruct the state of invoice-456 as it was on December 31st at 11:59pm"

**Claude:** *Uses `reconstruct_state` with specific timestamp*

### Scenario 3: User Behavior Analysis

**You:** "Find all users who had the same event sequence as user-123"

**Claude:** *Combines multiple tool calls*

### Scenario 4: System Monitoring

**You:** "Are there any anomalies in the event patterns from the last hour?"

**Claude:** *Uses `find_patterns` with anomaly detection*

---

## 🎯 Why This Is Amazing

### Traditional Approach:
```
Developer: "Let me write a SQL query..."
*20 minutes later*
Developer: "Hmm, the data is across 5 tables..."
*1 hour later*
Developer: "Here's your answer"
```

### With AllSource + MCP:
```
You: "What changed for user-123 yesterday?"
Claude: [Instant answer with full context]
```

### Key Advantages:

1. **Natural Language** → No SQL, no code, just ask
2. **Time-Travel** → Query any historical state
3. **Contextual** → Claude understands temporal relationships
4. **Comprehensive** → Full event history in one query
5. **Fast** → Real-time responses from optimized indexes

---

## 🐛 Troubleshooting

### MCP Server Not Showing

1. Check path in config is correct (absolute path!)
2. Ensure Bun is installed: `bun --version`
3. Restart Claude Desktop completely
4. Check logs: `~/Library/Logs/Claude/mcp*.log` (macOS)

### Connection Errors

1. Make sure AllSource services are running:
   - Core on http://localhost:8080
   - Control Plane on http://localhost:8081

2. Test manually:
   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8081/health
   ```

### "Tool Failed" Errors

- Check AllSource Core logs for errors
- Verify entity IDs exist before querying
- Ensure timestamps are in ISO format

---

## 📊 Demo Metrics to Highlight

During your presentation, emphasize:

1. **Query Speed** - "Notice how fast Claude got the answer"
2. **Time-Travel** - "We just queried data from 3 days ago"
3. **No Code** - "I didn't write a single SQL query"
4. **Contextual Understanding** - "Claude knew to compare the states"
5. **Pattern Detection** - "It found anomalies we didn't even ask for"

---

## 🎬 Live Demo Script

### Part 1: Setup (30 seconds)
1. Show Claude Desktop with 🔌 connected
2. Open terminal showing AllSource running

### Part 2: Basic Query (1 minute)
1. "Show me all events for user-123"
2. Point out instant response
3. Show the event history

### Part 3: Time-Travel (1 minute)
1. "What did user-123 look like yesterday?"
2. Explain how it replayed events
3. Show the reconstructed state

### Part 4: Change Analysis (1 minute)
1. "What changed for user-123 this week?"
2. Show the diff
3. Explain the value for debugging

### Part 5: Pattern Detection (1 minute)
1. "Find unusual patterns in user events"
2. Show frequency analysis
3. Discuss anomaly detection potential

### Part 6: The Wow Moment (30 seconds)
1. "Explain everything about user-123"
2. Show comprehensive analysis
3. "All of this in 5 seconds, with natural language"

---

<div align="center">

**AllSource + Claude Desktop** = *Time-Travel Queries in Natural Language*

🚀 The Future of Data Infrastructure is AI-Native

</div>
