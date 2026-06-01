# prime voice file — X / LinkedIn / Show HN campaign

Status: **DRAFT** for owner review. Rides the viral "your voice is your moat"
thesis and lands on prime — honestly. Numbers are real, captured from the live
`allsource-prime 0.21.4` binary in `tooling/voice-demo/RESULTS.md`. Do not publish
until the GATE in `docs/launch/PRIME_VOICE_FILE_LAUNCH.md` is GO and commits are
pushed. Pair each asset with the noted visual.

**Honesty rule (non-negotiable):** lead on `prime_recall` (works, proven). Label
the compressed auto-index / one-file export / `--auto-inject` as **roadmap** (next
Core release) everywhere — never as a shipped feature. Over-claiming on an identity
product destroys the exact trust that is the whole point.

Cross-link mammoth + caveman — same family, complementary, not competitive.

---

## LinkedIn (1 post) — mirrors the source post's register

your voice is your last competitive moat.

we've hit peak ai homogenization. every linkedin post sounds the same. every code
review reads identical. every proposal hedges the same three ways. same model,
default settings, one generic voice.

the fix going around is a voice file: a ~4k-token markdown blob of how you think,
how you write, your battle scars and contrarian takes. build it once, paste it
into any tool.

the instinct is right. the format is already obsolete.

a static markdown blob goes stale the moment you save it. you re-run the interview
to update it — so you never do. you paste the whole 4k-token lump into every
prompt, your database takes bleeding into a post about hiring. it has no history.
and it isn't really shareable with a team.

we built the live version. prime stores each answer as a durable, embedded facet —
not a line in a file. then it doesn't paste your voice, it recalls the slice that
matters for the task in front of you.

real demo: asked it to write a post arguing "adding a database is usually the wrong
fix." the query never said "contrarian take." recall still surfaced the user's own
contrarian thesis as the top hit (score 0.757), then their durability expertise.
the voice-ON draft led with the punchline and a war story; the voice-OFF draft was
hedged, list-y, emoji. same prompt. the only difference was the recalled voice.

one --data-dir, every MCP agent — claude, cursor, codex, gemini. version it like
code. (the auto-compressed one-file export is on our roadmap, next core release —
recall is what's live and it's the better path anyway.)

what would your 100 questions reveal about how you actually think — and wouldn't
you rather they stayed current?

build it: `cargo install allsource-prime` → `/plugin install mammoth` → `/voice run`

#AITools #EngineeringLeadership #VoiceIdentity

---

## X thread (7 tweets)

**1/ (hook — the moat line + the demo clip)**
your voice is your last competitive moat.

peak ai homogenization is here — same model, default settings, one generic voice.

the viral fix is a 4k-token markdown "voice file." right idea. dead format.

we built the live version. 🧵

[ATTACH: 15-sec screen recording — `/voice run`, then ask the agent to write a post;
it recalls the relevant voice facets and the draft sounds like the user. The magic
moment.]

**2/ (the static blob is a v0)**
a markdown voice file is simple and offline — keep it if it works.

but: it goes stale (you never re-run the interview), you paste the WHOLE 4k-token
blob every time, it has zero history, and it isn't really team-shareable.

right idea, wrong data structure.

**3/ (the hero — recall by meaning)**
prime makes it live. each answer = one durable, embedded facet — not a line in a
file. you don't paste your voice. you recall the slice that matters for THIS task.

real run: prompt about "adding a database is the wrong fix" (query never said
"contrarian take") → top hit was the user's own contrarian facet at 0.757.

[ATTACH: the prime_recall results table from RESULTS.md §4 — facet / type / score.]

**4/ (the proof — voice ON vs OFF)**
same prompt, written twice. only difference: the recalled voice slice injected.

OFF: "it can be tempting to reach for another database... 🚀 Complexity 🔄
Consistency 💸 Cost... thoughts below 👇"

ON: "Adding a database is usually the wrong fix." then a war story, no emoji.

that difference IS the product.

[ATTACH: side-by-side voice-ON / voice-OFF screenshot.]

**5/ (portable + history)**
one --data-dir, recalled natively by any MCP agent — claude, cursor, cline,
windsurf, codex, gemini. point them all at the same dir, no per-tool re-paste.

and your voice has history: `prime_history` time-travels every facet. a flat blob
can't.

**6/ (team voice + install)**
the same store scales to a TEAM voice — shared ADR style, review culture,
contrarian takes — via hosted sync. local-only stays the free default.

build yours in 60s:
```
cargo install allsource-prime
/plugin install mammoth
/voice run
```

**7/ (family + link)**
voice file doesn't compete with @caveman or mammoth — same family. caveman
compresses what you say; mammoth never forgets; the voice file is how you say it.

(auto-compressed one-file export = roadmap, next core release. recall is live now.)

repo + the real demo 👇
[LINK: github.com/all-source-os/chronos + blog post]

---

## Show HN

**Title:** Show HN: A durable, queryable "voice file" for AI tools (not a static markdown blob)

**Body:**
There's a viral idea that your "voice" is your last competitive moat against AI
homogenization — encode how you think/write into a ~4k-token markdown file, paste
it into any tool. The instinct is right; the format isn't. A flat blob goes stale,
you paste the whole thing every prompt, it has no history, and it's not really
team-shareable.

We built the live version on AllSource prime. Each interview answer becomes a
durable, embedded `voice` node in an on-disk event store (WAL + Parquet, the same
AllSource Core engine — no account, survives restarts, never leaves your machine).
Then instead of pasting your voice, the agent *recalls* only the facets relevant to
the current task via in-process vector recall.

Real captured demo (`tooling/voice-demo/`, against `allsource-prime 0.21.4`):
for the prompt "argue that adding another database is usually the wrong fix" — a
reworded query that never said "contrarian take" — `prime_recall` returned the
user's own contrarian thesis as the top hit (score 0.757), then their durability
expertise. We then wrote the same post with and without that recalled slice; the
voice-ON draft leads with the punchline and a war story, the voice-OFF draft is the
generic hedged/emoji default. Same prompt, the recalled voice is the only variable.

It's MCP-native, so any MCP agent (Claude Code, Cursor, Cline, Windsurf, Codex,
Gemini) speaks it — one `--data-dir`, recalled everywhere, no per-tool re-paste.
Team voice is an opt-in hosted-sync upgrade, not a gate.

Honest limit: the auto-generated *compressed* index / one-file export / auto-inject
are on the roadmap (a documented Core recall-engine projection gap in 0.21.4 — it
reports 0 nodes for live-recorded facets). Recall, stats, and history all work
today, and recall is the better path anyway, so the voice flow runs end-to-end now.
We'd rather ship the working magic moment than over-claim an export.

It ships in the same plugin as mammoth (durable agent memory) and composes with
caveman (token compression). Plugin layer is MIT; engines (allsource-prime/core)
are Apache-2.0, chronis MIT.

Build it: `cargo install allsource-prime` → `/plugin install mammoth` → `/voice run`

Repo: github.com/all-source-os/chronos
Demo: tooling/voice-demo/RESULTS.md

---

## Per-asset visual notes

| asset | visual | source |
|---|---|---|
| Blog hero | `/assets/blog/your-voice-file-shouldnt-be-dead-markdown.webp` (owner to render) | new |
| X tweet 1 | 15s screen recording — `/voice run` → recalled voice draft (the magic moment) | live capture |
| X tweet 3 | `prime_recall` results table (facet / type / score) | RESULTS.md §4 |
| X tweet 4 | side-by-side voice-ON / voice-OFF | RESULTS.md §6 |
| LinkedIn | reuse the voice-ON/OFF side-by-side or the recall table | RESULTS.md §4/§6 |
| Show HN | text only; link the blog + RESULTS.md (HN rewards runnable proof) | — |

Headline numbers, consistent across all assets (all trace to RESULTS.md):
**12 facets recorded; `prime_recall` top hit 0.757 on a reworded query; voice-ON vs
voice-OFF on the same prompt.** Never cite prime_index/export/auto-inject as
working — roadmap only.
