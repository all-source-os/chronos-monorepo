# Prime Voice File — a live, durable, queryable identity index

**Status:** proposal + shipped thin layer (skill + command + convention; no server change)
**Owner:** Prime / mammoth
**Tools used:** existing `prime_*` MCP tools only — verified against
`apps/prime-mcp/src/tools.rs` and `apps/prime-mcp/src/main.rs`.

---

## The trend, and the weakness we exploit

Ruben Hassid's pattern, now viral: run yourself through a ~100-question interview,
compress the transcript into a ~4k-token markdown **"voice file,"** paste it into
any AI tool so the output sounds like you. *"Your voice is your last competitive
moat."*

The markdown is a **dead blob**. Four concrete weaknesses:

| Weakness of the markdown voice file | What Prime already does instead |
|-------------------------------------|---------------------------------|
| Goes stale — you re-run the interview to update it | Append a facet node; the index is regenerated on demand |
| One undifferentiated 4k-token lump pasted every time | `prime_recall` returns only the facets relevant to *this* task |
| Can't be queried by relevance | Vector + graph recall, ranked by meaning |
| No history — you can't see how your voice evolved | `prime_history` time-travels every facet |

Prime already has the better primitive. The new work is **the flow and the
framing** — a `voice` domain convention, a structured interview, a skill, and a
slash command — *not* new server code.

---

## 1. The voice-file schema (a convention, not a migration)

A voice file is **not** a markdown blob and **not** a new table. It is a set of
embedded `voice` nodes in a Prime store, recorded with `prime_add_node` +
`prime_embed` and cross-linked with `prime_add_edge`. No migration, no schema
enforcement — Prime nodes are schemaless; this is a **naming/property
convention** the skill follows.

### Node shape

Each interview answer becomes one node:

```jsonc
// prime_add_node
{
  "type": "voice",                       // single node type for the whole voice file
  "properties": {
    "name": "Add a database is usually the wrong fix",   // short title
    "domain": "voice.contrarian",        // facet group → see below (powers the index)
    "facet": "contrarian_take",          // machine-readable facet tag
    "statement": "When availability or scale hurts, the reflex is 'add another \
                  database'. I think that's usually wrong — the better fix is \
                  replication of what you already run, not a new stateful component."
  }
}
```

Then embed it so it's recallable by meaning:

```jsonc
// prime_embed  (server embeds in-process via fastembed; no external model)
{ "id": "<entity_id from prime_add_node>",
  "text": "<the full statement>",
  "metadata": { "facet": "contrarian_take", "domain": "voice.contrarian" } }
```

Then cross-link related facets (optional, raises recall quality):

```jsonc
// prime_add_edge
{ "source": "<node A>", "target": "<node B>", "relation": "relates_to" }
```

### The five facets (matching the post's structure)

Every facet maps to a `domain` so the compressed index groups them, and to a
`facet` tag so the skill can pull a single facet on demand.

| Facet | `domain` | `facet` tag | What it captures |
|-------|----------|-------------|------------------|
| Thinking patterns | `voice.thinking` | `thinking_pattern` | How you reason, decompose, decide |
| Communication style | `voice.communication` | `communication_style` | Sentence shape, tone, what you cut |
| Domain expertise / battle scars | `voice.expertise` | `domain_expertise` | What you've shipped, debugged, learned the hard way |
| Contrarian takes | `voice.contrarian` | `contrarian_take` | Opinions you'll defend against consensus |
| Strategic frameworks | `voice.frameworks` | `strategic_framework` | Reusable mental models you apply |

This *is* the durable, queryable replacement for the markdown blob: each facet is
an independently recallable, versioned, embedded fact — not a line in a file.

### Provenance / privacy guard (carried over from mammoth-memory)

Record the user's **self-description and analysis** as voice facets. **Never**
auto-ingest proprietary employer code, secrets, or full file contents into the
voice store. The voice file is *identity*, not a codebase.

---

## 2. The 100-question interview, structured

A bank organized by facet. The agent walks the user through it and records each
answer as an embedded `voice` node **as it goes** — there is no separate "compress
the transcript" step, because `prime_index` generates the compressed view on
demand. A user can do a **short pass (~20, four per facet)** or the **full pass
(~100)**. Questions are prompts, not a script — the agent adapts and follows up.

### Facet A — Thinking patterns (`thinking_pattern`)
1. When you hit a hard problem, what's your literal first move?
2. Do you reason from first principles or from analogy/precedent? Give an example.
3. How do you decide a decision is reversible vs. irreversible?
4. What do you do when you have to decide without enough information?
5. How do you weight the cost of being wrong vs. the odds of being right?
6. When you disagree with a "best practice," how do you decide whether to follow it?
7. How do you break a vague problem into tractable pieces?
8. What's a question you ask yourself that others don't?
9. How do you know when you're overthinking something?
10. When you're stuck, what unsticks you — more data, a walk, a rubber duck, sleep?
11. How do you tell a real constraint from an assumed one?
12. What's your relationship with being wrong in public?
13. How do you decide what NOT to do?
14. When two experts you respect disagree, how do you form your own view?
15. What's your default when the data and your gut conflict?
16. How do you sanity-check a plan before committing?
17. What kind of problem energizes you vs. drains you?
18. How do you handle a decision you'll have to defend later?
19. What's a thinking habit you've deliberately trained?
20. How do you decide when "good enough" is actually enough?

### Facet B — Communication style (`communication_style`)
21. Do you lead with the conclusion or build to it? Why?
22. What's the first thing you cut from your own writing?
23. What words or phrases do you refuse to use? (e.g. "amazing," "game-changing")
24. How do you use humor — dry, absent, self-deprecating, sharp?
25. Short punchy sentences or longer constructed ones?
26. How do you open a piece so someone keeps reading?
27. How concrete do you get — do you always reach for an example/number?
28. How do you handle disagreement in writing — direct or softened?
29. What's your tell — a phrase or structure people recognize as you?
30. How formal are you by default, and when do you flex?
31. Do you use lists, or prose? When each?
32. How do you end a piece — call to action, mic drop, or just stop?
33. How do you sound when you're certain vs. genuinely unsure?
34. What's the longest a paragraph should be, for you?
35. Do you swear, use emoji, use exclamation marks? Where's the line?
36. How do you explain something technical to a non-expert?
37. What makes writing sound fake to you?
38. How do you give bad news?
39. How do you praise without sounding like marketing?
40. What's a sentence you'd be proud to have written?

### Facet C — Domain expertise / battle scars (`domain_expertise`)
41. What's your single deepest area of expertise?
42. What's a problem in your domain that everyone underestimates?
43. What did production teach you that no book did?
44. What's the worst bug/incident you debugged, and what it taught you?
45. What do juniors in your field consistently get wrong?
46. What's a tool/technique you trust that's out of fashion?
47. What's a popular tool/technique you avoid, and why?
48. What's the hardest tradeoff in your domain?
49. What do you know now that you wish you'd known five years ago?
50. What's a war story you tell to make a point?
51. What's a metric you watch that others ignore?
52. What scales badly that people assume scales fine?
53. What's "obvious" to you that surprises others?
54. What's a failure mode you design against by default?
55. What's the most expensive mistake you've seen (or made)?
56. What's a heuristic from your field you'd put on a poster?
57. What's the gap between how your domain is taught and how it's practiced?
58. What's a hill you'd die on technically?
59. What do you refuse to outsource to a tool/AI, and why?
60. What's a hard-won shortcut you've earned?

### Facet D — Contrarian takes (`contrarian_take`)
61. What's an opinion you hold that most of your field disagrees with?
62. What "best practice" do you think is usually wrong?
63. What's overrated in your industry right now?
64. What's underrated that you'd champion?
65. What trend are you deliberately not chasing?
66. What's a sacred cow you'd happily slaughter?
67. Where does consensus advice break down in practice?
68. What do you think the next five years will prove people wrong about?
69. What's a take you've softened on, and why?
70. What's a take you've hardened on?
71. What do you wish people would stop saying?
72. What's a default you flip that surprises people?
73. What problem do most people solve the wrong way?
74. What's a "rule" you break on purpose?
75. What would you tell your industry if they had to listen for 60 seconds?
76. What's a fashionable solution to a problem you don't think most people have?
77. Where is the conventional wisdom right, but for the wrong reasons?
78. What's a comfortable belief you think is lazy?
79. What's a fight worth picking?
80. What's the most contrarian thing you actually act on (not just say)?

### Facet E — Strategic frameworks (`strategic_framework`)
81. What's a mental model you apply almost daily?
82. How do you prioritize when everything is urgent?
83. What framework do you use to make a tough call?
84. How do you decide what to delegate vs. do yourself?
85. How do you think about risk vs. reward?
86. What's your model for when to go fast vs. slow?
87. How do you decide when to cut your losses?
88. What's your approach to ambiguity (e.g. make the implicit explicit)?
89. How do you scope a project so it actually ships?
90. What's your model for building vs. buying?
91. How do you evaluate a new opportunity?
92. What's your framework for hiring / who you want around you?
93. How do you think about leverage — where one effort pays off many times?
94. What's your model for trust — how it's earned and lost?
95. How do you decide what to learn next?
96. What's your default move when a plan meets reality?
97. How do you balance short-term and long-term?
98. What's a framework you've stolen and made your own?
99. How do you know a strategy is working before the results land?
100. If you could install one framework in everyone you work with, what is it?

The agent records each answer as a `voice` node (`type:"voice"`, the right
`domain`/`facet`, the answer as `statement`) and embeds it. Skipped questions are
just not recorded; the user can resume any time and `prime_search` /
`prime_recall` show what's already captured.

---

## 3. The three wedges, mapped to `prime_*` tools

### Wedge 1 — Live, durable, queryable

The voice file is a Prime domain, not a file.

- **Durable:** every `prime_add_node` / `prime_embed` appends an immutable event
  to the WAL (CRC32, fsync), snapshotted to Snappy Parquet. It survives restarts.
  Verified in `tooling/voice-demo/RESULTS.md` §1 — `prime_stats` returns the 12
  recorded `voice` nodes (`event_count: 31`).
- **Live:** updating your voice = appending one facet, not re-running a 100-Q
  interview and re-compressing a file.
- **Queryable / relevant slice:** `prime_recall` (`text` = the writing task)
  returns *only* the facets that matter for this task, ranked by meaning — you
  never paste 4k tokens again. Verified in `RESULTS.md` §4: for *"adding another
  database is usually the wrong fix,"* the top hit is the user's own contrarian
  facet (score **0.757**), then their durability/event-sourcing expertise.
- **The compressed view:** `prime_index` is the auto-generated equivalent of the
  post's hand-compressed 4k-token markdown — always current, regenerated on
  demand. Verified working in `RESULTS.md` §2 (allsource-prime 0.21.6): it returns
  the live, populated index — **12 nodes, 5 domains, token_count 77** — for nodes
  recorded via the live MCP write path. The 0-node bug is fixed (commit 4b61441).
- **History:** `prime_history` on any facet shows when it was recorded and every
  change. A markdown blob has no provenance. Verified in `RESULTS.md` §5.

### Wedge 2 — Portable + multi-agent

One Prime `--data-dir`, recalled natively by **any** MCP agent — the `prime_*`
tools are MCP-standard, not Claude-specific. Add this stanza to each tool's MCP
config, pointing at the **same** data dir, and your voice follows you across
tools with no copy-paste:

```jsonc
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/voice",   // ← same dir in every tool
        "--auto-inject",                   // inject the voice index every conversation
        "--auto-inject-max-tokens", "1000"
      ]
    }
  }
}
```

Config file locations (same stanza, same `--data-dir`):

| Tool | MCP config location |
|------|---------------------|
| Claude Code (project) | `.mcp.json` at project root (or `cn prime setup`) |
| Claude Desktop | `~/.claude/claude_desktop_config.json` → `mcpServers` |
| Cursor | `.cursor/mcp.json` (project) or `~/.cursor/mcp.json` (global) → `mcpServers` |
| Cline (VS Code) | Cline MCP settings JSON → `mcpServers` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` → `mcpServers` |
| Codex CLI | `~/.codex/config.toml` → `[mcp_servers.prime]` (`command`/`args`, TOML) |
| Gemini CLI | `~/.gemini/settings.json` → `mcpServers` |

`--auto-inject` exposes `prime://auto-context` as an MCP resource so the agent's
system prompt automatically carries the compressed voice index every conversation
(verified flag in `apps/prime-mcp/src/main.rs`). Now that the index is populated
(0.21.6), the resource carries the real compressed voice index. The voice follows
the user because it's one store, spoken by every MCP agent natively — no per-tool
re-paste.

### Wedge 3 — Team voice / shared identity

The same mechanism scales to a team's collective voice — shared ADR style,
code-review culture, conventions, contrarian takes — via Prime's hosted sync. Add
two flags (no new code, no new billing — reuse the existing onboarding/upgrade
path in `plugin/mammoth/UPGRADE.md`):

```jsonc
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/voice",
        "--auto-inject",
        "--sync-to", "https://api.all-source.xyz",   // hosted Core (Control Plane fronts auth)
        "--api-key", "<tenant key>"                    // from `allsource-onboard` or the dashboard
      ]
    }
  }
}
```

`--sync-to` + `--api-key` (verified flags, `apps/prime-mcp/src/main.rs`; they must
be supplied together) spawn a push-only loop shipping `prime.*` events to the
team's hosted tenant. Every teammate signed into the same tenant recalls the same
team voice; the web Memory tab shows it. **Local-only stays the free default;
hosted/team is the pull-not-gate upgrade.** Auth terminates at the Control Plane —
Core never authenticates public traffic (per `CLAUDE.md`). Prefer `PRIME_API_KEY`
(env) over an inline `--api-key` so the key never lands in a committed config.

---

## 4. Convention vs. server (honest separation)

Everything in this proposal is built from **existing tools as a convention** — no
change to `apps/prime-mcp/`. The compressed-index path that was broken when this
was first proven is now fixed; one narrow residual TODO remains and is documented
here rather than papered over.

### Works today (verified against the real binary)

- The `voice` node type + `domain`/`facet`/`statement` property shape.
- Recording the interview via `prime_add_node` + `prime_embed` + `prime_add_edge`.
- The relevant-slice recall loop via **`prime_recall`** (the load-bearing path —
  verified working, §4 of `RESULTS.md`; top hit 0.7572).
- **`prime_index`** — the auto-generated compressed voice file — returns the live,
  populated index (**12 nodes, 5 domains, token_count 77**) for nodes recorded via
  the live MCP write path. The 0-node bug is **fixed** in `allsource-prime 0.21.6`
  (commit 4b61441). Verified in `RESULTS.md` §2.
- `/voice export` can emit that compressed index (the `prime_index` output) as
  portable markdown.
- `--auto-inject` exposes the populated index as the `prime://auto-context`
  resource — works now that the index is populated.
- `/voice status` via `prime_stats` (+ `prime_search type:voice`) — verified.
- `prime_history` for voice evolution — verified.
- Portability stanza and team `--sync-to`/`--api-key` path — verified flags.

### Residual TODO (narrow, documented — NOT the index)

**`prime_context`'s L2 *vector* arm still returns an empty `[]` vectors list.**
`prime_context` returns the populated *index* correctly; only its vector
sub-field is unpopulated, because of a documented `// TODO: vector search
integration` in `context_l2` (`apps/core/src/prime/recall/api.rs`). This is **not**
the old 0-node index bug — the index works. Captured in `RESULTS.md` §3.

**Impact on the voice flow:** none. The relevant-slice recall and the
compressed-index export both run on paths that work. For the vector-recall path,
use **`prime_recall`** (which works); don't rely on `prime_context`'s vector
sub-field until that TODO lands. This is a Core recall-engine follow-up tracked
outside the voice feature.

---

## 5. Where this lives — `plugin/mammoth/`, not a new `plugin/voice/`

The voice flow ships as an **additional skill + command inside the existing
`plugin/mammoth/` plugin**, not a sibling `plugin/voice/`. Rationale:

- mammoth is already "the durable-memory layer over Prime's `prime_*` tools,
  local-only by default, syncs when you want." The voice file is a **specialized
  record/recall pattern on the same Prime store and the same `.mcp.json`** — it
  needs zero new wiring.
- A new plugin dir would duplicate the entire `.mcp.json`, `plugin.json`,
  `install.sh`/`install.ps1`, `UPGRADE.md`, and fragment the install story for no
  benefit. Two plugins pointing at Prime is worse UX than one plugin with two
  skills.
- The portability and team-sync stories are **identical** to mammoth's — same
  flags, same Control Plane path, same `UPGRADE.md`. Reuse, don't fork.

Shipped files:

- `plugin/mammoth/skills/voice-identity/SKILL.md` — interview + voice-aware
  writing loop + status/export.
- `plugin/mammoth/commands/voice.md` — `/voice run | status | export | sync`.

The one judgement call: a voice file ideally wants its **own** `--data-dir`
(`~/.prime/voice`) separate from general project memory (`.prime/`), so your
identity isn't diluted by per-project facts. The skill documents running a second
`prime` MCP entry (e.g. server key `voice`) pointed at `~/.prime/voice` for users
who want that separation; the default shares the mammoth store and filters by
`type:voice` / `domain:voice.*`.

---

## Proof

`tooling/voice-demo/` drives the real `allsource-prime` binary over stdio MCP
against a throwaway temp `--data-dir`, records 12 voice facets, and captures:
`prime_stats` (facets recorded), `prime_index` (the populated compressed index —
12 nodes / 5 domains / 77 tokens), `prime_recall` (the working relevant-slice
recall, top hit 0.7572), `prime_history` (provenance), and a voice-ON vs voice-OFF
completion for the same prompt. See `tooling/voice-demo/RESULTS.md`.
