---
description: Record a fact, decision, or gotcha into durable memory (AllSource Prime) so it can be recalled in future sessions.
---

Record the following into durable memory via the AllSource Prime MCP tools: **{{args}}**

Steps:
1. If `prime_*` tools are unavailable, tell the user memory isn't wired (the `prime` MCP server must be approved) and stop.
2. Decide the node `type` (`decision`, `insight`, `concept`, `project`, `event`, `metric`) and a `domain` for what the user gave you. If they didn't frame it as a decision, default to `insight`.
3. Check for an existing node on this subject with `prime_search` / `prime_neighbors`; **update** it rather than duplicate if one exists.
4. `prime_add_node` with a clear `name`, the chosen `domain`, and a `rationale`/detail property capturing the user's note in your own words.
5. `prime_embed` the node's `entity_id` with `text` = the full fact, so it's recallable by meaning later.
6. Link it with `prime_add_edge` to any obviously related node.

**Never** store raw source code, secrets, or full file contents — record the analysis/decision, not proprietary material. Confirm back what you stored (node name + domain) in one line.
