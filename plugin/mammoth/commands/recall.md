---
description: Recall what was learned in past sessions from durable memory (AllSource Prime) — decisions, gotchas, project facts.
---

Recall from durable memory whatever is relevant to: **{{args}}**

Steps:
1. If `prime_*` tools are unavailable, tell the user memory isn't wired (the `prime` MCP server must be approved) and stop.
2. Call `prime_context` with `query` = the user's request (tier `L2` — adds the compressed index + vectors + graph). For a narrow lookup, `prime_recall` with `text` = the query and `top_k` 5 is enough.
3. If nothing relevant comes back, say so plainly — do **not** fabricate prior context. A cold/empty store is expected early on.
4. If results come back, answer the user's question **from the recalled memory**, and make the source visible ("from an earlier session: …") so they see the recall working.

If the user gave no argument, call `prime_stats` and summarize what's in memory (node/edge counts by type, domains).
