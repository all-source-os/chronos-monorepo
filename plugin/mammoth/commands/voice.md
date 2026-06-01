---
description: Build and use your durable voice file (AllSource Prime) — run the interview, check status, export the compressed voice file, or turn on team sync. Subcommands: run | status | export | sync.
---

Manage the user's durable **voice file** — their thinking patterns, communication
style, domain expertise, contrarian takes, and strategic frameworks — stored as
embedded `voice` nodes in AllSource Prime, not a static markdown blob. Argument
selects the subcommand: **{{args}}** (default: `run` if empty).

First: if `prime_*` tools are unavailable, tell the user the `prime` MCP server
must be approved (and `allsource-prime` on PATH) and stop. Otherwise dispatch on
the argument. Full procedure for each mode is in the **voice-identity** skill;
design + question bank in `docs/proposals/PRIME_VOICE_FILE.md`.

- **`run`** (default) — Run or resume the structured interview. Check existing
  coverage (`prime_search type:voice`); offer a short pass (~20) or full pass
  (~100) from the facet-grouped bank; record each answer immediately as an embedded
  `voice` node (`prime_add_node` + `prime_embed`, optional `prime_add_edge`). Use
  the user's own words in `statement`. Never record proprietary code or secrets.

- **`status`** — `prime_stats` for total `voice` facets, `event_count`, and the
  `sync` field; `prime_search type:voice` for a per-facet-group tally so the user
  sees coverage and gaps. Report the compressed token count from `prime_index` if
  populated; fall back to the `prime_search` count if it reports 0 (known gap).

- **`export`** — Emit the current voice file as portable markdown. Prefer
  `prime_index` (the compressed view); fall back to enumerating `voice` nodes via
  `prime_search` grouped by `domain` if the index is empty. Tell the user the
  better path is live recall (writing in voice), not pasting a static export.

- **`sync`** — Turn on cross-machine / team voice via the existing upgrade path:
  onboard for a free API key (`allsource-onboard`), restart `prime` with
  `--sync-to https://api.all-source.xyz --api-key <key>` (or `PRIME_SYNC_TO`/
  `PRIME_API_KEY`), verify `prime_stats` → `sync.enabled: true`. Treat the key as a
  secret; never echo or store it. Local-only stays the free default.

To write something in the user's voice, you don't need this command — the
voice-identity skill recalls the relevant voice slice (`prime_recall`) before any
user-facing draft automatically.
