# Prime as a Folk + Notion Replacement — Design

*Date: 2026-06-10*
*Status: Validated brainstorm → buildable plan. Not yet scheduled.*
*Author: all.source team*

---

## One-liner

Build a **unified, durable, recall-first entity graph** on AllSource Prime that
replaces Folk (relationship CRM) and Notion (docs + databases) with a single
typed graph — people, orgs, deals, docs, notes — queryable by one hybrid-recall
box neither incumbent can match. Notion connects as an **optional two-way sync
satellite**; Folk imports once.

The moat is not a better block editor. It is the **cross-tool graph + time-travel**
that you get for free from event sourcing and that neither Folk nor Notion has.

## Decisions (locked in brainstorm)

1. **Product shape:** AI-native memory layer + *thin views*. Prime is the system of
   record and the brain. We do **not** rebuild Notion's WYSIWYG/CRDT editor.
2. **Wedge:** *Unified entity graph* from day one — person/org/deal/doc/note are one
   typed node graph, one recall box spans all. This is the differentiator over
   running Folk AND Notion separately.
3. **Collaboration:** Small team, async. Shared tenant workspace, roles
   (owner/member/viewer), comments, @mentions. **No CRDT / live cursors.** The Core
   event log gives edit history for free; conflicts resolve last-write-wins.
4. **Notion connector:** Continuous **two-way** sync. Prime is source of truth.
   Changes flow both ways via timestamps; **per-field last-write-wins**. The event
   log retains both sides, so LWW never loses data — it only chooses what is "current."

## Why Prime is the right substrate

Prime already ships the hard half:

- Typed **graph** (nodes + edges with properties)
- **Vectors** (384-dim, in-process fastembed) for semantic search
- **Hybrid recall** = vector similarity + graph proximity (BFS) + temporal recency
- **Projections** — materialized views folded from events
- **Event-sourced provenance** — every field change is a Core event → time-travel, audit
- **Multi-tenant** isolation (Control Plane owns auth/roles)

What's left is an *application layer* (content model, views, collab, connectors, UI),
not new storage infrastructure. We add no second database. (See ADR-020 — Prime is
stateless over Core; this plan keeps that property.)

---

## Architecture mapping

### Nodes (typed entities)

| Node | Replaces | Key properties |
|---|---|---|
| `person` | Folk contact | name, emails, phones, tags, source, last_contacted_at |
| `org` | Folk company | name, domain, industry |
| `deal` | Folk pipeline item | stage, value, close_date, owner |
| `doc` | Notion page | title, markdown blocks, author |
| `database` | Notion DB schema | field definitions (a user-defined node type) |
| `db_row` | Notion DB row | dynamic typed fields per its `database` |
| `task` | both | status, due, assignee |
| `interaction` | Folk activity | channel (email/meeting/call), ts, summary |
| `comment` | both | body, author, target node |

### Edges (typed relations)

`works_at` (person→org), `owns` / `with` (person→deal, deal→org),
`mentions` (doc/interaction→person/org/deal, **auto-extracted**),
`knows` (person→person, with strength), `assigned_to` (task→person),
`references` (doc→doc, backlinks), `involves` (interaction→person).

### Vectors

Embed doc bodies, contact notes, deal context, interaction summaries → semantic
search + the recall box. Embedding is server-side (fastembed), no client model.

### Projections = views (free from events)

- **Pipeline board** — group `deal` nodes by `stage`
- **Contact timeline** — `interaction` nodes for a person, ordered by ts
- **Database view** — `db_row` nodes filtered/grouped → table / kanban / calendar / gallery
- **Backlinks** — incoming `mentions` / `references` edges for any node

### Recall — the wedge

> "Who do I know at ACME and what did we last discuss?"

Resolves as: vector match on docs/interactions → graph hop person→org → temporal
recency ranking. **Folk and Notion are separate silos and cannot answer this.**
One graph is the whole pitch.

### Provenance

Every field change is a Core event. Free: time-travel ("what did this contact look
like last quarter"), audit, and conflict-safe sync (both sides retained).

---

## Feature gaps to build

Prime has graph / vector / recall / durability. Missing pieces, grouped:

### A. Content model
- **Markdown block doc model** — `doc` node with structured block body. No CRDT; LWW.
- **User-defined databases** — schema definition + typed rows with custom fields.
  This is Notion's core; maps to dynamic node types + property validation.
- **View renderers** — table, kanban, calendar, gallery (UI over projections).

### B. CRM depth (Folk parity)
- **Pipelines + value rollups** — projection over `deal` by stage.
- **Reminders / follow-ups** — temporal + **scheduler (NEW)**.
- **Email/calendar sync** — Gmail/GCal ingest → `interaction` nodes (**NEW**; reuse
  the MCP Gmail/GCal integration pattern).
- **Contact enrichment** — domain → company info (**NEW, optional 3rd-party**).

### C. Collaboration
- **Workspace membership + roles** — Control Plane extension (owner/member/viewer).
- **Comments + @mentions** — `comment` node + edge to target.
- **Per-node visibility / ACL** — read scope on nodes (**NEW**).
- **Notifications** — mentions, reminders, assignments (**NEW**).

### D. AI layer (mostly have)
- **Unified recall box** — have the engine; need UI + scoping.
- **Auto entity extraction** — parse docs/emails → create person/org/deal nodes +
  `mentions` edges (LLM, **NEW**).
- **Entity resolution / dedup** — same person across sources; **critical for sync** (**NEW**).
- **Smart suggestions** — "haven't talked to X in 30d", "this note names a new company"
  (derived from graph + temporal, **NEW**).

### E. Notion connector (optional, two-way)
- **Notion OAuth.**
- **Mapping layer** — page↔`doc`, DB↔`database`, property↔field, relation↔edge.
- **Sync engine** — cursor-based poll + write-back. Per-field LWW via Notion
  `last_edited_time` vs Prime event ts. Event log keeps both sides → no data loss.
- **Conflict policy** — LWW per field; surfaced in history for manual override.
- **Folk import** — one-time, Folk API → person/org/deal nodes.

### F. UI
- **Next.js thin views** (reuse `apps/web`): recall inbox, contact card, graph view,
  pipeline board, doc editor, database view.

---

## Phasing (buildable)

| Phase | Scope | Goal |
|---|---|---|
| **P0** Schema & API | node/edge types, property validation, workspace roles, ACL model | foundation |
| **P1** Workspace MVP | doc model, user-defined DB + table view, unified recall box, auto entity extraction, thin web views | **prove cross-tool recall** |
| **P2** CRM depth | pipelines, reminders/scheduler, activity timeline, Gmail/GCal sync, enrichment | Folk parity |
| **P3** Collaboration | comments/@mentions, notifications, sharing/visibility | team-ready |
| **P4** Notion connector | OAuth, mapping, two-way sync + conflict policy; Folk import | optional satellite |
| **P5** Smart layer | suggestions, dedup hardening, kanban/calendar views | stickiness |

**Ship P1 first** — the unified recall box over docs + contacts is the thing that
proves the wedge. Everything after is depth and parity.

## YAGNI / explicitly out of scope (for now)

- Real-time co-editing (CRDT/OT, live cursors) — event-log history covers the need.
- A full WYSIWYG editor — markdown blocks only.
- Per-block permissions — node-level ACL is enough for MVP.
- Bidirectional Folk sync — one-time import only; we replace Folk, not orbit it.

## Open questions for implementation phase

- Notion's webhook coverage is limited; confirm poll interval + rate limits before P4.
- Entity-resolution strategy (deterministic keys vs embedding similarity threshold).
- Scheduler: reuse an existing Core/Control-Plane mechanism or add one.

---

*References: `apps/core/src/prime/` (graph + vectors + recall), ADR-015
(vector index generation counter), ADR-020 (Prime stateless over Core),
`docs/articles/turbovec-vs-allsource` reasoning on the index layer.*
