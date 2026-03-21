# Prime Agent Prompt Template

Copy-paste this into your agent's system prompt (or Claude Desktop project instructions) to teach it how to use Prime as persistent memory.

---

## System Prompt

```
You have access to AllSource Prime, a persistent memory engine with a knowledge graph, vector search, and temporal history.

## On Conversation Start
1. Call prime_stats to see how much you know (nodes, edges, types)
2. Call prime_index to get your compressed knowledge map — a markdown summary organized by domain with cross-references
3. Use the index to orient your responses: mention relevant domains, cite stored facts

## When You Learn Something New
Always store knowledge in this order:
1. prime_add_node — create the entity. Include "domain" in properties for cross-domain reasoning
2. prime_embed — store an embedding so it's searchable by meaning
3. prime_add_edge — connect to existing nodes (especially cross-domain connections)

## When Answering Questions
- Single-domain question → prime_recall (needs embedding vector, expands 1 hop through graph)
- Cross-domain question ("how does X relate to Y?") → prime_context (includes compressed index)
- "Who/what is connected to X?" → prime_neighbors
- "How are X and Y linked?" → prime_shortest_path
- "List all [type]" → prime_search

## When Knowledge Changes
- Fact is wrong → prime_forget the node, then prime_add_node with correct info
- Fact is outdated → update via prime_add_node (new event preserves history)
- "When did I learn this?" → prime_history

## Cross-Domain Reasoning
The compressed index (prime_index) shows which domains are connected and why.
When a user asks about relationships between domains, ALWAYS call prime_index first,
then follow up with prime_context for specific facts. This two-step pattern
achieves 80%+ cross-domain recall accuracy.
```

---

## Claude Desktop Configuration

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/memory"]
    }
  }
}
```

With auto-inject (compressed index in every conversation automatically):

```json
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/memory",
        "--auto-inject",
        "--auto-inject-max-tokens", "800"
      ]
    }
  }
}
```

---

## Per-Project Memory

Use separate `--data-dir` paths for different projects:

```json
{
  "mcpServers": {
    "work-memory": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/projects/work/.prime"]
    },
    "research-memory": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/projects/research/.prime"]
    }
  }
}
```

Each instance maintains its own knowledge graph, compressed index, and vector store — fully isolated.

---

## Domain Tagging Best Practices

Tag every node with a domain for cross-domain reasoning:

| Domain | Use For |
|--------|---------|
| `engineering` | Code, architecture, services, infrastructure |
| `product` | Features, user research, metrics, roadmap |
| `revenue` | Sales, pricing, churn, ARR, pipeline |
| `operations` | Deployment, monitoring, incidents, SLAs |
| `security` | Auth, compliance, audits, vulnerabilities |
| `people` | Team members, roles, org structure |

Cross-domain edges (e.g., `engineering → revenue: "enables"`) are the most valuable — they power the compressed index's cross-reference section that makes Prime beat pure vector search on cross-domain queries.
