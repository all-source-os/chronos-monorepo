---
name: voice-identity
description: Build and use a durable "voice file" — your thinking patterns, communication style, domain expertise, contrarian takes, and strategic frameworks — backed by AllSource Prime instead of a static markdown blob. Runs a structured interview that records each answer as an embedded `voice` node, then BEFORE writing anything user-facing (LinkedIn post, ADR, code review, proposal, email) recalls the relevant voice slice and writes in that voice. The viral "voice file" idea, but live, queryable, portable across MCP agents, and team-shareable. Use when the user says "build my voice file", "set up my voice", "/voice", "write this in my voice", "make this sound like me", "capture how I think/write", or asks for user-facing writing once a voice file exists. Triggers PROACTIVELY: recall the voice slice before drafting any first-person user-facing text. voice = identity layer; mammoth = the memory engine underneath.
---

# voice-identity

A durable **voice file** backed by the AllSource Prime MCP server (`prime_*`
tools) — the same Prime store mammoth uses, with a `voice` node convention. It
replaces the viral "run a 100-question interview, hand-compress to a 4k-token
markdown blob, paste it everywhere" pattern with something **live** (update by
appending a facet), **queryable** (recall only the relevant slice per task),
**portable** (one store, every MCP agent), and **team-shareable** (`--sync-to`).

Design rationale, the full 100-question bank, the three wedges, and the honest
convention-vs-server-gap line: `docs/proposals/PRIME_VOICE_FILE.md`.

**Hard rule — provenance / privacy:** record the user's *self-description and
analysis* — how they think, write, and decide. **NEVER** write proprietary
employer code, secrets, API keys, or full file contents into a `voice` node or
embedding. The voice file is *identity*, not a codebase.

**Availability gate:** only act if `prime_*` tools are present. If not, skip
silently unless the user asks — then tell them the `prime` MCP server must be
approved and `allsource-prime` must be on PATH (`cargo install allsource-prime`,
or the mammoth installer).

---

## The voice schema (convention)

One node type, `voice`, with five facet groups. Each interview answer = one node.

| Facet | `domain` | `facet` tag |
|-------|----------|-------------|
| Thinking patterns | `voice.thinking` | `thinking_pattern` |
| Communication style | `voice.communication` | `communication_style` |
| Domain expertise / battle scars | `voice.expertise` | `domain_expertise` |
| Contrarian takes | `voice.contrarian` | `contrarian_take` |
| Strategic frameworks | `voice.frameworks` | `strategic_framework` |

Recording one answer (three calls):

1. `prime_add_node` → `type:"voice"`, `properties:{ name, domain, facet, statement }`
   where `statement` is the user's answer in their own words.
2. `prime_embed` → `id` = the returned `entity_id`, `text` = the full statement,
   `metadata:{ facet, domain }`. The server embeds in-process (fastembed); no
   external model. This is what makes the facet recallable by meaning.
3. `prime_add_edge` (optional) → `relation:"relates_to"` linking facets in the
   same group or across groups. Cross-links raise recall quality.

Tool params come from each tool's input schema (authoritative — don't guess).

---

## Mode 1 — RUN / RESUME the interview

Triggered by "build my voice file", "/voice run", "set up my voice".

1. **Check what's captured.** `prime_search` with `type:"voice"` (or `prime_stats`
   → `nodes_by_type.voice`). If facets exist, tell the user what's covered and
   offer to resume or extend rather than restart.
2. **Pick the pass.** Offer a **short pass (~20, four per facet)** or the **full
   pass (~100)**. The question bank is in `docs/proposals/PRIME_VOICE_FILE.md` §2,
   grouped by facet. Walk the user through it conversationally — these are prompts,
   not a rigid script; follow up to draw out specifics ("give me the concrete
   example", "what's the war story").
3. **Record as you go.** After each answer, record it immediately (node + embed +
   optional edge). Do **not** batch and "compress at the end" — there is no
   compression step; `prime_index` generates the compressed view on demand. Use
   the user's own phrasing in `statement` — voice fidelity matters here.
4. **Skip freely.** Unanswered questions are simply not recorded. The user can
   resume any session; `prime_search type:voice` shows progress.
5. **Confirm sparingly.** Don't narrate every write. Periodically summarize: "logged
   4 thinking-pattern facets, 3 communication facets — want to keep going or switch
   facets?"

---

## Mode 2 — WRITE in the user's voice (proactive default)

Triggered before producing ANY user-facing first-person writing — LinkedIn post,
ADR, code review, proposal, email, README intro — or on "write this in my voice".

1. **Recall the relevant slice.** Call **`prime_recall`** with `text` = the writing
   task (e.g. the post topic), `top_k` 5–8, `depth` 1. This returns the facets that
   matter for *this* task, ranked by meaning — you do NOT paste the whole voice
   file. (`prime_recall` is the load-bearing recall path; `prime_context` L2 is an
   equally valid hybrid-recall path — see the note below.)
2. **Write from the recalled facets.** Fold the recalled `statement`s into the
   draft: adopt the recalled communication-style facets (sentence shape, tone,
   words to avoid), lead with the recalled thinking patterns, deploy the recalled
   contrarian take or framework if it fits the topic. The voice-ON vs voice-OFF
   contrast in `tooling/voice-demo/RESULTS.md` is the target — punchline-first, a
   concrete number/war story, no hype words, the user's actual opinions.
3. **Don't invent voice.** If recall is empty (cold voice file), say so and offer to
   run the interview — never fabricate a voice the user hasn't recorded.
4. **Make recall visible.** Briefly note which facets you wrote from ("drew on your
   'add a database is the wrong fix' take + your durability expertise") — that
   visibility is the magic moment.

> **Recall path note (honest):** `prime_recall` returns the correct ranked slice,
> and `prime_index` returns the live, populated compressed voice index (12 nodes /
> 5 domains / 77 tokens in the demo) as of `allsource-prime 0.21.6`. `prime_context`
> L2 now returns **full hybrid recall** too — the compressed index plus vector hits
> plus graph nodes (fixed in commit 5083017); the earlier empty-vector-arm gap is
> resolved. So the skill can use **either** `prime_recall` **or** `prime_context` L2
> for the vector-recall path; both work. (L0 is stats-only / vectorless by design —
> that's the tier's contract, not a limit.) The provenance/privacy guard above still
> applies regardless of which path you use: record self-analysis only, never
> proprietary code or secrets. See the proposal § "Convention vs. server" and
> `tooling/voice-demo/RESULTS.md`.

---

## Mode 3 — STATUS (`/voice status`)

1. `prime_stats` → report `nodes_by_type.voice` (total facets), `event_count`, and
   the `sync` field (local-only vs syncing to a hosted tenant).
2. `prime_search type:"voice"` → tally facets per `domain` so the user sees coverage
   per facet group ("thinking 6, communication 5, expertise 4, contrarian 3,
   frameworks 2 — contrarian + frameworks are thin").
3. `prime_index` for the compressed view + token count — it returns the live,
   populated index (works as of 0.21.6). The `prime_search` tally remains a fine
   cross-check / fallback.

---

## Mode 4 — EXPORT (`/voice export`)

Emit the current voice file as portable markdown — the bridge to the post's
"4k-token file," but generated live from durable facts.

1. **Preferred (works):** `prime_index` → its `index` markdown IS the compressed
   voice file. Emit it verbatim — it returns the live, populated index as of
   `allsource-prime 0.21.6`.
2. **Fallback:** `prime_search type:"voice"`, group nodes by `domain`, and render a
   markdown doc with one `##` section per facet group and each `statement` as a
   bullet. Not token-compressed, but complete and current. Use this only if you want
   the full enumerated export rather than the compressed `prime_index` view; it is
   no longer required (the index works), just an option.

The user can paste either into any tool — but the better path is Mode 2 (recall the
relevant slice live), not pasting a static export.

---

## Mode 5 — SYNC / team voice (`/voice sync`)

Share one voice across machines or a whole team. This is the same upgrade path as
mammoth's `/memory-sync` — reuse it, don't reinvent.

1. Confirm intent: cross-machine or team voice ships `prime.*` events to a hosted
   AllSource Core tenant. Local-only is the free default and already works.
2. Get a free account + API key via the `allsource-onboard` skill (or the
   dashboard). Treat the key as a secret — never echo it, never write it into Prime.
3. Restart the `prime` server with `--sync-to https://api.all-source.xyz --api-key
   <key>` (or `PRIME_SYNC_TO`/`PRIME_API_KEY` env). Auth terminates at the Control
   Plane; local store is unchanged, sync is purely additive.
4. Verify with `prime_stats` → `sync.enabled: true`. Team voice — shared ADR style,
   review culture, conventions — now recalls consistently across the tenant.

See `plugin/mammoth/UPGRADE.md` for the full path.

---

## Portability (works with no extra steps)

The voice file is one Prime `--data-dir`, recalled natively by any MCP agent
(Claude Code, Claude Desktop, Cursor, Cline, Windsurf, Codex, Gemini) — point each
tool's MCP config at the **same** `--data-dir` (e.g. `~/.prime/voice`). With
`--auto-inject` on, `prime://auto-context` carries the voice index into every
conversation automatically. Exact per-tool stanzas:
`docs/proposals/PRIME_VOICE_FILE.md` §3, Wedge 2.

Tip: a voice file ideally wants its own `--data-dir` (`~/.prime/voice`) so identity
isn't diluted by per-project facts. If sharing the mammoth store, filter by
`type:"voice"` / `domain:"voice.*"`.

---

## Verification checklist

- [ ] Checked `prime_*` tools exist before acting (silent skip if not).
- [ ] Each recorded answer is a `voice` node with `domain` + `facet` + `statement`,
      embedded so it's recallable; cross-linked where useful.
- [ ] Recorded the user's self-analysis only — NO proprietary code, secrets, or
      full file contents.
- [ ] Before any user-facing writing, recalled the relevant slice via `prime_recall`
      and wrote from it; made the recall visible; never fabricated voice on a cold
      store.
- [ ] `/voice status` reports the real facet count (`prime_index` returns the
      populated index as of 0.21.6; `prime_search` tally is a fine cross-check).
- [ ] Sync/team uses the existing `--sync-to`/`--api-key` upgrade path; key treated
      as a secret.
