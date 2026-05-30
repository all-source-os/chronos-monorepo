---
description: Show durable-memory status — what mammoth remembers (AllSource Prime stats) and whether cross-machine sync is on.
---

Report the current state of durable memory.

Steps:
1. If `prime_*` tools are unavailable, tell the user memory isn't wired (the `prime` MCP server must be approved) and stop.
2. Call `prime_stats` and report: total nodes, total edges, nodes-by-type, edges-by-relation, and event count.
3. State the storage mode: local-only by default (a `.prime/` data dir on this machine — durable, survives restarts, no account). If the `prime` server was started with `--sync-to`/`--api-key`, memory also syncs to a hosted AllSource Core.
4. If memory is local-only, mention the upgrade in one line: cross-machine and team memory need a free AllSource account (`allsource-prime --sync-to <url> --api-key <key>`, or the `allsource-onboard` skill) — a *pull*, not a gate.

Keep it to a short status block, not a wall of text.
