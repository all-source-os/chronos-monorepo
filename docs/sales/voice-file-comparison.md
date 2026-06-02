# Static markdown voice file vs prime voice (recall)

**Status: DRAFT** — reusable on the `/compare` surface and in sales. A fair
comparison: static markdown genuinely wins on simplicity and one-file portability
today; prime wins on durability, queryability, history, portability across agents,
and team sharing. No strawman — the audience includes people who already built a
markdown voice file, and they should keep it if it's working.

Every prime claim traces to the captured demo in `tooling/voice-demo/RESULTS.md`
(live `allsource-prime 0.21.6`) or `docs/proposals/PRIME_VOICE_FILE.md`.

## The idea both share

Your voice is your last competitive moat against AI homogenization. Encode how you
think, write, and decide — your battle scars, contrarian takes, and frameworks —
into a portable identity layer you bring to any AI tool. Both approaches agree on
this. They differ on the data structure.

## Comparison

| axis | static markdown voice file | prime voice (recall) |
|---|---|---|
| **Simplicity / zero-infra** | ✅ one file, nothing to run | ⚠️ runs a local MCP server (`allsource-prime`) |
| **Works offline today** | ✅ yes | ✅ yes — local `--data-dir`, no account |
| **Stays current** | ❌ re-run the interview + recompress (so most never do) | ✅ append one facet; no recompress step |
| **Query by relevance** | ❌ paste the whole ~4k-token blob every prompt | ✅ `prime_recall` returns only the relevant slice (demo top hit **0.757** on a reworded query) |
| **History / provenance** | ❌ flat snapshot, no record of how your voice evolved | ✅ `prime_history` time-travels every facet (real `created` event in the demo) |
| **Durability** | ✅ a file is a file | ✅ event store — WAL (CRC32 + fsync) + Snappy Parquet; survives restarts |
| **Portable across MCP agents** | ⚠️ re-paste the file into each tool | ✅ one `--data-dir`, recalled natively by every MCP agent (Claude Code, Cursor, Cline, Windsurf, Codex, Gemini) |
| **Team sharing** | ❌ hand a file around | ✅ hosted sync (`--sync-to`) — shared team voice, local-only stays free default |
| **Auto-compressed one-file export** | ✅ that's all it is | ✅ `prime_index` generates the compressed voice index on demand — always current (demo: 12 nodes / 5 domains / 77 tokens) |

## Where each genuinely wins

**Static markdown wins:** simplicity, zero infrastructure, and a single portable
file you can hand to a ghostwriter or version in Obsidian with nothing installed.
If that's all you need, it's a fine v0 — keep it.

**prime wins:** durability with provenance, query-by-relevance (you recall the
slice that matters for the task instead of pasting everything), history, native
portability across every MCP agent without re-pasting, and team voice.

## The proof (real, captured)

For the prompt *"argue that adding another database is usually the wrong fix"* — a
reworded query that never mentioned "contrarian take" — `prime_recall` returned the
user's own contrarian thesis as the top hit (score **0.757**), then their
durability/event-sourcing expertise and a relevant framework. Writing the same post
with vs. without that recalled slice produced a sharp difference: the voice-ON draft
led with the punchline and a concrete war story; the voice-OFF draft was the generic
hedged, list-y, emoji default. The recalled voice was the only variable. Full
capture: `tooling/voice-demo/RESULTS.md`.

## Honest note on the one residual limit

The auto-generated *compressed* index (`prime_index`), the one-file `/voice export`
of that view, and `--auto-inject` all **work** as of `allsource-prime 0.21.6`: the
0-node projection bug is fixed (commit 4b61441), and `prime_index` returns the live,
populated voice index (12 nodes / 5 domains / 77 tokens) on demand. The one residual
limit, kept honest: `prime_context`'s L2 *vector* arm still returns an empty list (a
documented `// TODO: vector search integration`). `prime_context` returns the
populated index correctly; only its vector sub-field is unpopulated. Use
`prime_recall` for the vector path — it works and is proven. We state the one TODO
plainly rather than dress it up.

## Family fit

prime's voice file is the **identity layer** of the same durable memory substrate
mammoth gives your agent, and it composes with caveman: caveman compresses what
your agent *says*; mammoth remembers what it *learned*; the voice file is *how you
say it*.

## Licensing

Plugin layer is MIT; the engines (allsource-prime / AllSource Core) are Apache-2.0;
chronis is MIT. (Not "all MIT.")
