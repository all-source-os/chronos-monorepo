# AllSource Agent Memory Ecosystem — Strategy & Blueprint

> **Status:** Decision-grade proposal. Date: 2026-05-29.
> **Decision required:** Green-light a local-only MVP + first-90-days plan, or a documented no-go.

---

## TL;DR (read this first)

1. **Recommendation: conditional GO.** Build the local-only packaging + magic-moment demo first (2 weeks), prove that a fresh agent recalls a decision from an earlier session unprompted, and only then invest in the multi-agent install matrix and hosted tier. **Kill it at the 2-week gate if day-one recall quality is unimpressive** (see Kill-Criteria). The risk is unusually low because ~80% of what ships already exists in this repo.
2. **The product is "durable agent memory," not "another database."** `chronis` (`cn`) gives agents *episodic/work memory* (event-sourced tasks); `allsource-prime` gives them *semantic recall* (graph + vector + temporal). Pitch line: **caveman compresses what you say; this remembers what you said.**
3. **The install default MUST be local-only, zero-signup — and the code already defaults to it.** chronis defaults to embedded/local mode with no remote and no key (`ChronisConfig::default()` → `mode: CoreMode::Embedded, sync: None`; a missing `.chronis/config.toml` returns those defaults — `apps/chronis/src/infrastructure/config.rs`), and `allsource-prime --data-dir <path>` runs a fully local, durable store with no account. Hosted sync (`--sync-to` + `--api-key`) is the *upgrade*, not the gate.
4. **MCP is our unfair advantage over caveman.** caveman hand-rolls per-agent installers for 30+ agents. Memory rides MCP: Claude Code, Cursor, Cline, Windsurf, Codex, Gemini already speak the 20 `prime_*` tools natively (`apps/prime-mcp/src/tools.rs`). One server binary, one manifest stanza per agent — no per-agent shim for the core capability. `cn prime setup` already auto-writes a project `.mcp.json` for Claude Code.
5. **Where the caveman analogy breaks: state.** caveman ships a stateless local flag file — nothing persists, nothing can be "wrong," it costs the author $0/user. Memory has durable state (Core WAL+Parquet), recall quality (a small 384-dim embedder), and — at the hosted tier — auth and per-user backend cost. These are real problems caveman never had; the plan treats them as first-class, not hand-waved.

**Headline recommendation:** Build it, but earn each tier. The Prime MCP server, the 20 recall tools, the local-only default, and `cn prime setup` already exist — the genuinely new work is *packaging, a benchmark harness, and a narrative*, not core engineering. Ship a frictionless local memory plugin that makes any MCP agent recall prior work unprompted; prove recall quality on real numbers before spending a dollar on hosted infra or a day on the 30-agent matrix.

---

## Why Caveman Won — Deconstructed

caveman (`github.com/JuliusBrussee/caveman`, ~66.4k stars) compresses agent output ("speak like caveman") to cut output tokens ~65% (its own benchmark: 10 tasks, 1214→294 avg tokens). Its success has two separable engines: **mechanics** (how it spread) and **narrative** (why people cared). Below, each factor is rated for transferability to a *stateful memory* product.

> Legend: **High** = ports directly; **Medium** = ports with added work; **Low/Breaks** = the stateless→stateful gap defeats it.

### Mechanics

| Factor | What caveman did | Transferability | Why / where it breaks |
|---|---|---|---|
| Multi-agent installers | Per-agent install scripts, 30+ agents | **High (and better)** | Memory rides MCP; any MCP agent speaks the 20 `prime_*` tools natively — *more* portable than caveman's per-agent hacks, fewer bespoke shims. |
| Marketplace manifest | `.claude-plugin/` + marketplace plugin config | **High** | Same plugin packaging applies. (Note: this repo has skills under `.claude/skills/` but **no `.claude-plugin/` marketplace manifest yet** — that's new work.) |
| Idempotent installer | Re-runnable `install.sh`/`install.ps1`, ~30s, skips absent agents | **Medium→High** | `cn prime setup` already does the idempotent Claude-Code path (writes `.mcp.json`, preserves other entries — see `apps/prime-mcp/README.md`). Generalizing to a curl-pipe-bash installer that also `cargo install`s the binary is modest new work. |
| Zero-config auto-activate | Skill triggers on phrases | **Medium→High** | Prime already ships `--auto-inject` exposing `prime://auto-context` as an MCP resource that injects a compressed index into the agent's system prompt every conversation (`apps/prime-mcp/src/main.rs`, README "Auto-Inject"). That *is* the auto-activate mechanism — it needs packaging and a default-on story. |
| Published benchmarks | Real token-savings table in `benchmarks/` | **High (must-do)** | caveman's credibility was numbers. We must ship recall precision@k, cross-session win-rate, tokens-saved. Designed below. The Prime README already makes comparison claims (e.g. "12μs queries", "80%+ cross-domain recall") that must be backed by a reproducible harness, not left as marketing. |
| License | MIT ("free like mass mammoth") | **Medium** | **Reality check:** `allsource-prime` and `allsource-core` are **Apache-2.0**; `chronis` is **MIT** (verified in their `Cargo.toml`s). Apache-2.0 is still permissive and fine for OSS adoption; the plugin/installer layer can be MIT. Do not claim "all MIT." |
| One-liner pitch | "why use many token when few token do trick" | **High** | A meme-grade one-liner transfers fully; drafted in Positioning. |

### Narrative

| Factor | What caveman did | Transferability | Why / where it breaks |
|---|---|---|---|
| The meme | Caveman-speak is intrinsically funny/shareable | **Medium** | "Memory" is less inherently funny. Borrow the *register* (blunt, one-line, honest), but the hook is utility, not comedy. |
| Honesty disclaimer | "affects output tokens only — caveman no make brain smaller, caveman make mouth smaller" | **High (critical)** | Our equivalent: "remembers across sessions on *this machine* for free; cross-machine needs an account." Stating the limit up front is the trust move. |
| Academic backing | Cited a brevity-vs-accuracy paper | **Medium** | Event sourcing + vector recall have literature, but the credible signal here is *reproducible benchmarks on real coding sessions*, not citations. |
| "Complementary, not competitive" | Didn't fight existing tools; shipped a 5-repo family | **High** | We explicitly position *with* caveman: it compresses output, we persist context. Composable, cross-linkable story (below). |

**The honest break in the analogy:** caveman is stateless and local — nothing it does can go stale, leak across users, or cost the author per-user money. Memory introduces (a) **durable state** that must survive restarts (Core already does this: WAL CRC32+fsync, Parquet Snappy — `CLAUDE.md` and `apps/prime-mcp/README.md` "How it stores"), (b) **recall quality** from a deliberately small `AllMiniLML6V2` (384-dim, ~25MB) embedder that must be benchmarked, not assumed, and (c) **auth + per-user backend cost** the moment you cross machines. The High-rated factors survive the gap; the Medium ones are exactly where the founder spends effort caveman never had to.

---

## Product Definition: Durable Agent Memory

**One-sentence promise:** *Your coding agent remembers what it learned in past sessions — decisions, task history, and code context — and recalls it automatically, so you never re-explain your project twice.*

### Mapping to real AllSource primitives (every claim cited)

| Memory type | Primitive | Backing evidence (this repo) |
|---|---|---|
| **Episodic / work memory** (what was done, when, why) | chronis events — tasks/work as immutable events via `cn` | `apps/chronis/` (crate `chronis`, bin `cn`, v0.7.0). Every mutation is an event; state from projections; temporal replay (`apps/chronis/README.md`). Skills `ralph-tui-cn-beads`, `ralph-tui-cn-prd` already drive it. |
| **Semantic recall** (find the relevant prior thing) | allsource-prime — 20 MCP tools over graph + vector + temporal | `apps/prime-mcp/src/tools.rs` defines all 20: `prime_add_node/edge`, `prime_neighbors/search/shortest_path`, `prime_forget/history/stats`, `prime_index/context`, `prime_embed/similar/recall`, `prime_list_templates/load_template`, `prime_define_projection/list_projections/project_node/node_provenance`. |
| **The "orient yourself" primitive** (the magic-moment engine) | `prime_index` + `prime_context` + `--auto-inject` | `prime_index` returns a compressed, token-counted markdown summary of everything the agent knows ("Call this FIRST at the start of every conversation"); `prime_context` does tiered L0/L1/L2 recall; `--auto-inject` injects the index automatically as `prime://auto-context`. This is *already built* — it is the equivalent of caveman's auto-trigger. |
| **Durable substrate** (survives restarts) | AllSource Core event store | WAL (CRC32, fsync), Parquet (Snappy), DashMap reads (`CLAUDE.md`; Prime README architecture diagram: "WAL + Parquet + DashMap + HLC + CRDT"). Embeddings are HNSW via `instant-distance`, computed in-process by fastembed — no external service. |
| **Proof it persists** (the trust claim, ready-made) | Durability test harnesses | Skills `chronos-durability` and `chronos-embedded-durability` — write events, restart, verify 100% survive. This doubles as a marketing asset: "we can *prove* your agent's memory survives a reboot." |

### What the agent does on day one (the `/caveman` equivalent)

caveman's magic moment is typing `/caveman` and watching output shrink. The memory equivalent is **invisible recall**:

1. **Install** (one line). `cargo install allsource-prime` (+ `chronis`), then `cn prime setup` writes the project `.mcp.json` pointing at a local data dir (`<project>/.chronis/prime/`) — verified in `apps/prime-mcp/README.md`. No account, no token.
2. **Work normally.** As the agent works, it records salient memories via `prime_add_node` + `prime_embed` (and task events via `cn`). **This pattern already ships:** the `pr-atom-reviewer` skill records each review and recalls prior reviews via `prime_*`; `pr-review-coach` reads that history back. That is the existing proof-of-pattern, in production, today.
3. **The magic moment.** Sessions later, the user asks "why did we pick X over Y?" With `--auto-inject` on, the agent already has the compressed index in context; it calls `prime_recall`/`prime_context` and answers from memory — *without the user pasting any history*. Minimal demo: a recall the user knows they never told *this* session.

**Explicit commands (escape hatch; recall should auto-fire by default):**
- `/remember <note>` → `prime_add_node` + `prime_embed`
- `/recall <query>` → `prime_recall` / `prime_context`
- `/memory-status` → `prime_stats` (nodes, edges, event_count) + last-sync + data-dir size

The default experience is auto-recall (via `--auto-inject`); the slash commands mirror caveman exposing `/caveman` alongside its auto-trigger.

---

## Deployment Tiers & The Friction Problem

The single biggest threat to a caveman-style launch is **anything between the install one-liner and the magic moment.** A signup wall there flattens the stars curve. The good news: **the architecture already makes the frictionless tier the default.**

| Tier | Storage | Signup? | Cross-machine? | Our cost/user | Mechanism (verified) |
|---|---|---|---|---|---|
| **Local-only** (DEFAULT) | Embedded Core on disk (`--data-dir`), durable WAL + Parquet | **None** | No (this machine) | **$0** | `allsource-prime --data-dir ~/.prime/memory`; chronis defaults to `CoreMode::Embedded` with `sync: None` when no `config.toml` is present (`config.rs`). |
| **Hosted** | Sync local events to remote Core via API key | Yes (account + key) | Yes (multi-machine, team, off-box durability) | Backend cost | `allsource-prime --sync-to <url> --api-key <key>` (`PRIME_SYNC_TO`/`PRIME_API_KEY`, `apps/prime-mcp/src/main.rs`); chronis `[sync]` block in `config.toml` with `remote_url` + `api_key` (`SyncConfig`, `config.rs`). Auth terminates at the Control Plane (`CLAUDE.md`). |

### Recommended install default: **Local-only, zero-signup.** Defended:

- **Friction is the enemy of the curve.** caveman's 66k stars came from "paste one line, see it work." Any account step kills that. Local-only reaches the magic moment with no gate — and the binary already supports exactly this mode.
- **It's honest and technically real — not a persistence fake.** `--data-dir` mode is genuinely durable: same Core WAL+Parquet engine (chronis itself depends on `allsource-core` with the `embedded` feature, `apps/chronis/Cargo.toml`), provable by the `chronos-embedded-durability` skill. We are *not* blurring the line `CLAUDE.md` forbids (local memory is durable, not "in-memory only / lost on restart"). We never have to.
- **Zero per-user cost at launch.** A stars-driven launch can dump thousands of free users overnight. If every one hit our hosted Core, the bill (and abuse surface) could sink the project before monetization. Local-only externalizes storage to the user's disk — the right default for virality.
- **Hosted is a *pull*, not a *gate*.** The user upgrades when local-only can't solve the pain: "remember on my laptop AND desktop," or "share memory with my team." `/memory-status` is the natural upgrade prompt. The upgrade is one command — `--sync-to` + `--api-key`, and the `allsource-onboard` skill already automates account + API key + `.chronis/config.toml`.

**Friction-honesty in the README (non-negotiable):** State plainly — "Free tier remembers across sessions on *this machine*, no account. Cross-machine and team memory need a free AllSource account." This mirrors caveman's "output tokens only" trust move.

---

## Multi-Agent Install Matrix

caveman supports 30+ agents via bespoke installers. Memory's reach is *broader for less work* because the core capability is one MCP server — any MCP-capable agent gets all 20 `prime_*` tools natively. Three integration classes:

| Agent | Integration path | Effort | Notes |
|---|---|---|---|
| **Claude Code** | `cn prime setup` → project `.mcp.json` + `.claude-plugin/` marketplace plugin + skills (`/remember`, `/recall`, `/memory-status`) | **Low — mostly exists** | **First integration (see below).** Richest surface: MCP + skills + slash commands + `cn` CLI all land here. `allsource-mcp-setup` skill also exists. |
| **Cursor** | MCP config stanza (`mcp.json`) | Low | Native MCP; `prime_*` appear with no shim. |
| **Cline** | MCP config | Low | Native MCP. |
| **Windsurf** | MCP config | Low | Native MCP. |
| **Codex (OpenAI)** | MCP config | Low–Medium | MCP supported; verify tool-call surface on first integration. |
| **Gemini CLI** | MCP config | Low–Medium | Native MCP; verify `--auto-inject` resource is honored. |
| **Agents without MCP** | `cn` CLI + shell-level skill | Medium | Episodic memory still works via `cn` events even without MCP. This is the only place we replicate caveman's per-agent labor — keep it minimal. |

**MCP manifest stanza (verified flags — the per-agent payload):**

```jsonc
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/memory", "--auto-inject", "--auto-inject-max-tokens", "1000"]
    }
  }
}
```
*(Flags confirmed in `apps/prime-mcp/src/main.rs`: `--data-dir`/`PRIME_DATA_DIR`, default `--mode mcp` stdio, `--auto-inject`. To upgrade to hosted, add `--sync-to <url> --api-key <key>`.)*

### Highest-leverage first integration: **Claude Code.** Defended:

- It is the *only* surface where the full stack already ships: the 20 `prime_*` tools, the `cn` CLI, `cn prime setup` (auto-writes `.mcp.json`), **and** working skills that exercise memory in anger (`pr-atom-reviewer` recalls prior reviews; `pr-review-coach` reads review history from Prime; `allsource-onboard`/`allsource-mcp-setup` automate setup). We are *packaging what exists*, not building an integration.
- It gives the marketplace/plugin distribution channel caveman used to go viral.
- It lets us prove the magic moment on home turf — where recall quality is best understood — *before* generalizing the manifest to Cursor/Cline/Windsurf, which is then near-free since they share the same server binary.

Ship Claude Code first; fan out to other native-MCP agents in Phase 2 by publishing manifest snippets, not new code.

---

## Ecosystem Component Inventory

Mirrors caveman's layout, but for memory. **Bold = already exists in this repo** (package, don't build).

| Component | What it is | Status |
|---|---|---|
| **Core distributable: `allsource-prime` MCP server** | 20-tool graph/vector/temporal recall server (stdio + HTTP), local `--data-dir` mode, `--auto-inject` | **Exists** — `apps/prime-mcp/`, on crates.io, v0.21.5 |
| **Auto-activate: `--auto-inject` / `prime://auto-context`** | Injects compressed index into every conversation's system prompt | **Exists** — `apps/prime-mcp/src/main.rs` |
| **Episodic memory CLI: `cn` (chronis)** | Event-sourced task/work memory, TOON output, temporal replay | **Exists** — `apps/chronis/`, v0.7.0 |
| **Claude Code wiring: `cn prime setup`** | Idempotent `.mcp.json` writer for project | **Exists** — `apps/chronis/src/infrastructure/prime_setup.rs` (`run_prime_setup`; reads/preserves existing `.mcp.json`) |
| **Memory-using skills** | `pr-atom-reviewer`, `pr-review-coach` (Prime), `ralph-tui-cn-*` (chronis), `allsource-onboard`, `allsource-mcp-setup` | **Exists** |
| **Durability proof harnesses** | Write→restart→verify-survival | **Exists** — `chronos-durability`, `chronos-embedded-durability` skills |
| Skills: `/remember`, `/recall`, `/memory-status` | Thin slash commands over `prime_*` | **New** (thin) |
| Installers `install.sh` / `install.ps1` | Curl-pipe-bash: `cargo install` + run `cn prime setup` + detect agent | **New** (generalizes the existing setup) |
| Per-agent MCP manifest snippets | Copy-paste stanzas for Cursor/Cline/Windsurf/Codex/Gemini | **New** (trivial — shared server) |
| `.claude-plugin/` + marketplace config | One-click Claude Code add | **New** (no marketplace manifest in repo yet) |
| Benchmark / eval harness | Reproducible recall + continuity + token-savings | **New** (design below) |
| Docs: INSTALL matrix + README | The pitch + per-agent table | **New** |

**Structural insight:** ~80% of the hard engineering (the server, 20 tools, in-process embeddings, durable local store, auto-inject, Claude-Code wiring, durability proofs) is *already shipped and on crates.io*. The genuinely new work is the benchmark harness, three thin skills, a curl-installer, manifest snippets, the marketplace manifest, and the README. This materially lowers build risk versus caveman, which had to author everything.

---

## Benchmark & Eval Design

caveman's credibility was numbers (token savings, 10 tasks, three-arm harness: baseline vs "answer concisely" vs skill). Memory must publish equally concrete, **reproducible** numbers — and not over-claim. Each claim names a **method**, not just a metric. All run against a fixed, public corpus of real multi-session coding transcripts (build a 50–100 session fixture set). Crucially, the Prime README *already asserts* "80%+ cross-domain recall" and "12μs queries" — the harness exists to make those claims defensible rather than marketing.

| # | Claim | Metric | Method |
|---|---|---|---|
| 1 | **Recall is relevant** | **Precision@k (k=3,5)** | Labeled (query → known-relevant-memory) pairs from the fixtures. For each query, call `prime_recall`/`prime_context`, check if the gold memory is in top-k. Report P@3, P@5. Beat baseline: `prime_search`-by-type + keyword grep over the same store. |
| 2 | **Memory beats no-memory across sessions** | **Cross-session continuity win-rate** | Replay a session that references a decision made earlier. Run the agent twice — `--auto-inject`+recall ON vs OFF. Rubric-score / blind-judge which answer is correct & complete. Report win-rate of memory-on. |
| 3 | **It saves the user from re-explaining** | **Tokens-saved-by-not-re-explaining** | Measure input tokens the user *would have* pasted (project context, prior decisions) to get the same answer without memory, vs. tokens when the agent recalls it via `prime_context`. The direct caveman-analog metric and cleanest dollar story. Report median tokens saved per recall-eligible turn. |
| 4 | **Recall is fast enough to auto-fire** | **Recall latency p50/p95** | Time `prime_recall` (incl. in-process fastembed embed, ~1–3ms warm per README) end-to-end on the local store at 1k / 10k / 100k nodes. Must stay low enough to run before every answer. Core's read floor is ~12μs (README); report the realistic embed+HNSW path. |
| 5 | **Memory survives a reboot** (the trust claim) | **Durability pass/fail** | Reuse `chronos-durability` / `chronos-embedded-durability`: write memories, restart, verify 100% survive. Binary, but it separates "memory" from "session cache." Publish it. |

**Benchmark table schema (README-ready):**

```
| Metric                          | Memory ON | Baseline (search+grep) | Δ      |
|---------------------------------|-----------|------------------------|--------|
| Recall Precision@5              | 0.xx      | 0.xx                   | +xx%   |
| Cross-session continuity win    | xx%       | (n/a)                  | —      |
| Median tokens saved / recall    | x,xxx     | 0                      | +x,xxx |
| Recall latency p95 @ 10k nodes  | xx ms     | —                      | —      |
| Durability (write→restart→read) | PASS      | —                      | —      |
```

**Honesty rule (the caveman move):** if day-one Precision@5 is mediocre — plausible with a 384-dim model — *publish it anyway* and show the improvement curve (hybrid keyword+vector, graph expansion via `depth`, larger embedder as an opt-in). The trust caveman earned by "output tokens only" is the same trust we earn by not faking recall quality.

---

## Positioning & Naming

**Posture: complementary to caveman, never competitive.** caveman shrinks the *output*; we persist the *context*. They compose: an agent that compresses what it says *and* remembers what it learned is strictly better than either alone. caveman already ships a 5-repo family ("agent do more with less") — a cross-link is natural and on-brand.

**Recommended name: `mammoth`.** Defended:
- Same prehistoric register as caveman (instant tonal kinship — "the caveman of memory") — invites the composable story without copying. (caveman's own README even says "free like mass mammoth on open plain," so the word is already in the universe.)
- Mammoth = *big, never-forgetting memory* ("elephants never forget," scaled up). The metaphor *is* the pitch.
- Short, memeable, available-sounding, not a literal description that dates badly.
- Repo: `mammoth` (the plugin/distributable). The engines keep their real names — `allsource-prime` (MCP server) and `chronis` (`cn`) — so the OSS plugin gets a fun front-door while the durable tech keeps its identity. (Runner-up `recall` rejected: too generic, SEO-hostile, tonally flat next to caveman.)

**Three candidate one-liners (caveman register):**
1. *"caveman make few token. mammoth never forget token."*
2. *"your agent has goldfish memory. give it a mammoth."*
3. *"why re-explain project every time when agent just remember?"* (closest structural echo of caveman's own line)

**Recommended primary:** #1 — it does the complementary-positioning work *and* the meme in one breath, and name-checks caveman to ride its audience.

**README first paragraph (draft):**
> **mammoth** gives your coding agent durable memory. Decisions, task history, and code context from past sessions get remembered and recalled automatically — so you stop re-explaining your project every single time. Works locally with zero signup (your memory, your disk, survives reboots), and syncs across machines when you want it to. Any MCP agent — Claude Code, Cursor, Cline, Windsurf, Codex, Gemini — speaks it natively. *caveman make few token. mammoth never forget token.*

---

## Risks & Kill-Criteria

Adversarial. Each risk has a concrete kill/abort criterion.

| Risk | Why dangerous | Kill / mitigation criterion |
|---|---|---|
| **Day-one recall quality is unimpressive** | If the magic moment doesn't land, the curve never starts. `AllMiniLML6V2` (384d, ~25MB) is fast but not SOTA on relevance. | **KILL at 2-wk gate** if Precision@5 doesn't clearly beat a `prime_search`+grep baseline on the fixtures. Mitigation first: hybrid keyword+vector and graph expansion (Prime's `depth` param) — Prime already fuses vectors+graph+temporal in `prime_recall`. |
| **Auth friction kills the curve** | A signup wall between install and value flattens stars. | Already mitigated: **local-only default, zero signup** (chronis defaults to `CoreMode::Embedded`/`sync: None`, `config.rs`). Kill the *hosted-first* idea, never the project. Never gate the magic moment behind an account. |
| **Backend cost per free user** | A viral launch could flood hosted Core; per-user cost + abuse sinks it. | Local-only externalizes cost to user disk. **Do not open an unbounded free hosted tier.** Hosted is paid or tightly metered from day one. |
| **"Memory" is a crowded, hyped category** | Many agent-memory projects (Mem0, Letta, zer0dex — already in Prime's own comparison table). Attention is scarce. | Differentiate on (a) *durable, provable* persistence (the durability skill — competitors mostly cache), (b) immutable event audit / time-travel (`prime_history`, `get_node_as_of`), (c) MCP-native multi-agent reach, (d) caveman-style frictionless distribution. If the README can't draw a sharper line than "Mem0 but ours," **abort launch** until it can. |
| **MCP fragmentation** | Agents implement MCP inconsistently; `--auto-inject` resource may not be honored everywhere. | Ship Claude Code first (best-supported), verify each agent before listing. **Do not list an agent until its recall path is verified.** A broken row erodes the trust the README spent to earn. |
| **Installer maintenance load** | caveman's per-agent installers are a maintenance tax. | MCP collapses most agents to one shared server — minimal tax. Cap shim work: non-MCP agents get the `cn` CLI path only; **no bespoke per-agent runtime integrations** beyond manifest snippet + CLI. |
| **Local store growth / corruption** | Durable on-disk memory can bloat; bad UX, support load. | Lean on Core's WAL+Parquet recovery + `prime_forget` soft-delete (history retained). `/memory-status` surfaces size. **Kill auto-write of low-value memories** if store growth becomes the top complaint. |
| **Over-promising contradicts architecture facts** | Calling memory "in-memory" or routing events to Postgres violates `CLAUDE.md` and destroys credibility. | Hard rule: README/benchmarks describe Core as durable (WAL/Parquet); events never in Postgres; hosted auth via Control Plane. Review every public string against `CLAUDE.md` before launch. |
| **License over-claim** | Saying "all MIT" when Prime/Core are Apache-2.0 is a factual error contributors will catch. | State licenses correctly: Prime/Core Apache-2.0, chronis MIT, plugin layer MIT. |

**Master kill-criterion:** If at the 2-week gate the magic-moment demo doesn't make a neutral observer say "wait, how did it know that?", **stop.** Everything downstream (matrix, hosted tier, launch) is wasted spend without a magic moment.

---

## First-90-Days Plan

Three phases. Each names the **deliverable**, the **single go-metric**, and the **owner decisions** required.

### Phase 0 — Local-only MVP + magic-moment demo (Weeks 0–2)

- **Deliverables:**
  - `allsource-prime --data-dir … --auto-inject` wired into Claude Code via `cn prime setup` (both already exist — this is integration + scripting, not new core code).
  - One auto-recall skill that writes salient memories (`prime_add_node`+`prime_embed`) and pulls relevant memory (`prime_context`) before answering — generalized from the shipped `pr-atom-reviewer`/`pr-review-coach` pattern.
  - A recorded demo: a fresh session recalls a decision made in an earlier session, unprompted.
- **Go-metric:** A neutral observer watching the demo says "how did it know that?" **AND** Recall Precision@5 beats the `prime_search`+grep baseline by a clear margin on a 50-session fixture.
- **Owner decisions:** (1) Confirm `mammoth` as the name. (2) Approve local-only as the install default. (3) Define the "salient memory" auto-write policy (what gets remembered automatically) — the single highest-leverage product judgment call.

### Phase 1 — MCP install matrix + benchmark harness + README (Weeks 2–6)

- **Deliverables:**
  - Idempotent `install.sh` / `install.ps1`: `cargo install allsource-prime chronis`, run `cn prime setup`, detect agent, write MCP config.
  - `.claude-plugin/` + marketplace plugin config (new — no marketplace manifest exists yet) and `/remember`, `/recall`, `/memory-status` slash commands.
  - Verified MCP manifest snippets for Cursor, Cline, Windsurf (+ Codex/Gemini if verified).
  - Benchmark harness producing all 5 claims (precision@k, continuity win-rate, tokens-saved, latency, durability) with a public fixture set — backing the README's existing "80%+ recall / 12μs" assertions.
  - README with one-liner, first paragraph, install matrix, benchmark table.
- **Go-metric:** All 5 benchmarks published with honest numbers; ≥3 native-MCP agents verified end-to-end; clean install on a fresh machine in under one minute, no account.
- **Owner decisions:** (1) Which agents make the launch matrix (verified rows only). (2) License the plugin/installer layer MIT, state Prime/Core Apache-2.0 + chronis MIT correctly. (3) How aggressively to name-check caveman.

### Phase 2 — Marketplace listing + hosted tier + launch (Weeks 6–12)

- **Deliverables:**
  - Marketplace listing live (Claude Code plugin discoverable).
  - Hosted tier: `/memory-status`-driven upgrade → `allsource-onboard` automates account + API key + `.chronis/config.toml`; Prime runs with `--sync-to`+`--api-key` syncing events to Core via the Control Plane. Metered/paid, not unbounded-free.
  - Launch: Show HN / X thread / cross-link with caveman, leading with the demo + benchmark table.
- **Go-metric:** Stars + install velocity in the first 72h (caveman-style curve signal) **and** hosted conversion from the `/memory-status` upgrade prompt above a pre-set floor. If local installs spike but hosted converts ~0%, that's signal to rethink monetization — *not* to gate the free tier.
- **Owner decisions:** (1) Hosted pricing/metering. (2) Abuse controls on the gateway for hosted sync. (3) Launch timing relative to a clean benchmark story — do not launch on mediocre numbers.

---

## Verification notes for the reader

- **Architecture compliance:** Local-only memory is described as *durable* (Core WAL+Parquet), never "in-memory only"; events never routed to Postgres; hosted auth terminates at the Control Plane — all consistent with `CLAUDE.md`.
- **Grounding (verified in-session, not assumed):** 20 `prime_*` tools — `apps/prime-mcp/src/tools.rs`. `--data-dir`/`--mode`/`--auto-inject`/`--sync-to`/`--api-key` flags — `apps/prime-mcp/src/main.rs`. In-process fastembed `AllMiniLML6V2` 384d, HNSW via `instant-distance` — `apps/prime-mcp/README.md` + tool descriptions. `cn prime setup` writes/preserves `.mcp.json` — `apps/chronis/src/infrastructure/prime_setup.rs` (`run_prime_setup`) + `apps/prime-mcp/README.md`. Local-only default (`CoreMode::Embedded`, `sync: None`, missing config returns defaults) and `[sync]` `remote_url`+`api_key` — `apps/chronis/src/infrastructure/config.rs`. chronis embeds Core via the `embedded` feature — `apps/chronis/Cargo.toml`. Licenses: Prime/Core Apache-2.0, chronis MIT — respective `Cargo.toml`. Skills (`pr-atom-reviewer`, `pr-review-coach`, `ralph-tui-cn-*`, `allsource-onboard`, `allsource-mcp-setup`, `chronos-durability`, `chronos-embedded-durability`) — skills registry.
- **Not yet in repo (genuinely new work):** a `.claude-plugin/` marketplace manifest, the `install.sh`/`install.ps1` curl-installer, the `/remember`/`/recall`/`/memory-status` slash commands, the benchmark harness + public fixture set, and the launch README.
