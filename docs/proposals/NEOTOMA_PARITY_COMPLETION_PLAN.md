# Neotoma Parity Completion Plan

*Date: 2026-05-29 (status updated 2026-06)*
*Status: internal planning — derived from a live-code re-audit of the 6 gaps in `docs/articles/neotoma-comparison.md`. Not for publication.*

---

## Status — SHIPPED (2026-06)

The plan below is executed. The whole "deterministic primitives reachable beyond MCP" gap (§2 cross-cutting) plus the Gap 3 toggle are done, on Core REST + gateway + all four SDKs + dashboard:

| Step (from §3 sequence) | Status | Commits |
|---|---|---|
| 1. Gap 3 toggle API + dashboard | ✅ shipped | Core `806c0a9`, gateway `58d0a04`, dashboard `f576c25` (bead t-445a) |
| 2. Core REST routes (projections + provenance) | ✅ shipped | `f3f2578` (bead t-2ac8) |
| 3. Gateway proxy for the new Prime routes | ✅ shipped | `63a8e2c` (bead t-9501) |
| 4. SDK methods, all four languages | ✅ shipped | `6894976` (bead t-8bf4) |
| 5. Docs + comparison/internal-doc copy | ✅ this update | comparison-doc rows + conclusion + this header (bead t-061e) |
| 6. Gap 4 enforced-templates follow-up | ⏸ deferred | bead t-9d5a (p3) |

Also shipped alongside: a per-tool `/install` integration hub (`f80afa9`) and the two-access-planes clarification (`adb758c`). One genuinely-new follow-up surfaced and is tracked: **hosted MCP-over-HTTP transport** (bead t-dbee53) — today `allsource-prime`'s hosted Fly app serves Prime's REST plane, not the MCP protocol over HTTP.

**Caveat (verification):** every layer is covered by unit/in-process/mocked tests (router-level, controller-level, repo replay, SDK mock-server). There is **no end-to-end test** of the full Core→gateway→SDK chain against a running stack yet.

---

## Why this exists

`docs/articles/neotoma-comparison.md` (2026-05-23) enumerated 6 gaps with effort estimates and a 5→6→4→3→1→2 sequence. Since then, code has landed against most of them — but the comparison doc's snapshot is stale and commit messages over-claim. This plan re-audits each gap **against the code that exists today** and only plans the work that is genuinely open.

**Headline finding:** five of the six gaps are functionally shipped. The remaining work is *not* "build the primitive" — the primitives exist. It is (a) one missing admin toggle for Gap 3, and (b) a cross-cutting reach problem the original doc never named: **the new Prime primitives (declarative projections, per-field provenance, templates) are exposed only through the prime-mcp MCP server. They have no Core REST surface and no SDK methods.** A buyer who comes through the SDK or the REST API — not MCP — cannot see the very features we built to neutralize Neotoma. That is the real open gap.

---

## 1. Audit summary

### Gap status (file:line evidence)

| Gap | True status | Evidence | What's actually left |
|---|---|---|---|
| **Gap 1** — declarative projection primitive w/ merge policies | **Shipped at MCP+Core layer; not in SDK/REST** | Pure fold + 4 policies: `apps/core/src/prime/projections/declarative.rs:191` (`merge_array`), enum at `:1`–onward; persisted as events: `apps/core/src/prime/facade.rs:1546` (`define_projection`) + replay `:1574` (`load_projection_defs`); MCP tools `prime_define_projection`/`prime_list_projections`/`prime_project_node`: `apps/prime-mcp/src/tools.rs:262`,`277`,`282`,`343`–`477`; registry hydrate at startup `apps/prime-mcp/src/main.rs:136`. Unit + round-trip tests `declarative.rs:366`–`538`, `facade.rs:2370`–`2463`. | No SDK methods (Rust/TS/Py/Go) and no Core REST route — MCP-only. Projection registry is single-tenant in-process (`apps/prime-mcp/src/projection_registry.rs:1` header note). |
| **Gap 2** — per-field provenance as a first-class query | **Shipped at MCP+Core layer; not in SDK/REST** | `provenance_for_field` + `fold_with_provenance` + `Provenance`/`ProvenanceSnapshot`: `apps/core/src/prime/projections/declarative.rs:262`,`288`,`312`; MCP tool `prime_node_provenance`: `apps/prime-mcp/src/tools.rs:293`,`480`–`516`. Tests `declarative.rs:494`–`538`. | The doc proposed `GET /api/v1/prime/nodes/{id}/fields/{field}/provenance` — that REST route does **not** exist (`apps/core/src/infrastructure/web/prime_api.rs:29`–`52` has no projection/provenance routes). MCP-only, no SDK. |
| **Gap 3** — schema enforcement at write time | **Shipped (enforcement); per-tenant toggle has no external API** | `SchemaEnforcement{Permissive,Warn,Strict}`: `apps/core/src/domain/entities/tenant.rs:13`–`28`; tenant accessors `:451`,`455`; ingest enforcement `enforce_schema_if_configured`: `apps/core/src/infrastructure/web/api.rs:331`–`393`, wired into v1 single + batch ingest at `:413`,`:508`. Tests `tenant.rs:678`–`761`. | Toggle is in-process only. Code comment is explicit: `tenant.rs:11` "Tenants opt into stricter modes via the tenant repository (no admin HTTP yet — that's a follow-up)." No control-plane/query-service endpoint to set the mode (grep of `apps/query-service`, `apps/control-plane`, `apps/web/src` returns nothing). |
| **Gap 4** — pre-built entity templates | **Shipped (MCP, descriptive)** | 8 templates as embedded JSON: `apps/prime-mcp/templates/*.json` (person, contact, organization, task, decision, transaction, meeting, document); loader `apps/prime-mcp/src/templates.rs:39`–`69`; MCP tools `prime_list_templates`/`prime_load_template`: `apps/prime-mcp/src/tools.rs:306`–`321`,`341`,`518`. Tests `templates.rs:77`–`143`. | Templates are **descriptive, not registerable as enforced schemas** (`templates.rs:11`). The doc's "register with one call" is partially met — they're loadable guides, not one-call registrations into Core's `/schemas` registry. Optional follow-up, not a hard gap (see §2 Gap 4). No SDK exposure. |
| **Gap 5** — marketing: cross-tool sync | **Shipped** | `/prime` page leads with "Same memory, every agent surface" + per-client config (Claude Desktop, Claude Code, Cursor `~/.cursor/mcp.json`, OpenCode): `apps/web/src/app/(marketing)/prime/page.tsx:151`,`191`,`210`–`264`. Dedicated `/connect` minting flow: `apps/web/src/app/(marketing)/connect/connect-client.tsx`. | Nothing structurally. ChatGPT is named in the comparison-page copy but the `/prime` setup guide covers Claude Desktop, Claude Code, Cursor, OpenCode (not ChatGPT) — minor copy parity item only. |
| **Gap 6** — honest comparison page | **Shipped** | `/compare/agent-memory` is a real 5-way breakdown (platform memory, RAG/Mem0·Zep, file-based, database CRUD, event-sourced=us+Neotoma): `apps/web/src/app/(marketing)/compare/agent-memory/page.tsx:28`,`46`,`64`,`100`–`118`. In sitemap (`apps/web/src/app/sitemap.ts`). | Nothing. Done. |

### Neotoma primitive coverage (all 7)

| Neotoma primitive | AllSource equivalent | Covered? |
|---|---|---|
| **Observation** | Core events; surfaced to Prime as `Observation{observed_at, source_priority, specificity_score, source_event_id, fields}` (`apps/core/src/prime/facade.rs:1521`) | **Yes** |
| **Source** | Event `metadata` (`source_priority`, `specificity_score` read at `facade.rs:1512`); no content-addressed dedup primitive | **Partial — by design.** No content-addressed source store; not a declared gap. Note in Open Questions. |
| **Reducer** | `fold()` over observations with declarative `MergePolicy` (`declarative.rs`, `fold` exported in `projections/mod.rs`) | **Yes (Gap 1)** |
| **Entity snapshot** | `EntitySnapshot` from `fold`, via `prime_project_node` (`tools.rs:452`) | **Yes (Gap 1)** |
| **Relationship** | Prime graph edges — `prime_add_edge`, `/api/v1/prime/edges` (`prime_api.rs:39`) | **Yes (pre-existing)** |
| **Schema** | Core schema registry `/api/v1/schemas` + `SchemaEnforcement` modes (`api.rs:143`, `tenant.rs:13`) | **Yes (Gap 3, minus toggle API)** |
| **Memory graph** | Prime graph + vectors + recency, `prime_recall` (`apps/core/src/prime/recall/`) | **Yes (pre-existing; exceeds Neotoma — adds vectors)** |

All 7 have a verdict. The only primitive without a first-class AllSource equivalent is **Source** (content-addressed dedup), and that is a deliberate non-target, not one of the 6 gaps.

---

## 2. Per-gap implementation plan (open items only)

Gaps 5 and 6 are shipped and verified — no work item. Gaps 1, 2, and 4 are shipped at the MCP+Core layer; their only open work is the **SDK/REST reach** item (consolidated below as a single cross-cutting work item, because they share one surface). Gap 3's only open piece is the toggle API.

### Gap 3 (open piece) — Per-tenant schema-enforcement toggle via the gateway

Enforcement logic is done. What's missing is a way for a tenant to *turn it on* without direct DB access.

**Engineering**
- **Surface (public, through the gateway — Core never authenticates):**
  - Control Plane / Query Service: `PATCH /api/v1/tenants/me/settings` (or extend the existing tenant-settings handler if one exists) with body `{ "schema_enforcement": "permissive" | "warn" | "strict" }`. Authenticated as the tenant. Returns the updated setting.
  - The gateway translates this into a Core admin call. Core needs an **internal-only** route to mutate tenant enforcement: `PATCH /api/v1/admin/tenants/{tenant_id}` accepting `{ "schema_enforcement": "<mode>" }`, guarded the same way Core's other internal admin routes are (not public; gateway-to-Core only).
- **Data model:** none new — `Tenant.set_schema_enforcement` already exists (`tenant.rs:455`) and bumps `updated_at`. Persist via the existing tenant repository write path used by other tenant mutations. Confirm the tenant-repo backend (whatever persists `Tenant`) serializes the field — `schema_enforcement_serde_roundtrip` (`tenant.rs:686`) proves the type round-trips; verify the repo's stored shape includes it and that legacy rows default to `Permissive` (already covered by `schema_enforcement_default_is_permissive`, `tenant.rs:678`).
- **Files to modify:**
  - Core: add the admin handler in `apps/core/src/infrastructure/web/api.rs` (alongside existing tenant handlers) and route it in `apps/core/src/infrastructure/web/api_v1.rs` (internal/admin section).
  - Query Service: add/extend the tenant-settings controller + route (look under `apps/query-service/lib/.../controllers/` for the existing tenant or settings controller; if none, add one and a route in the router).
  - Web: surface a "Schema enforcement" select (Permissive / Warn / Strict) in `apps/web/src/app/dashboard/settings/page.tsx` calling the gateway endpoint. Optional but high-value for the "visible to a buyer" requirement.
- **SDKs to update:** add `set_schema_enforcement(mode)` / `getTenantSettings()` to Rust, TS, Py, Go SDKs **only if** tenant-settings management is already part of the SDK surface. If SDKs are event/query-only today (likely — grep shows no tenant-admin methods), **do not** add tenant admin to them; this is a dashboard + gateway concern, not an SDK concern. Decide per Open Question Q2.

**Tests**
- Core integration test (in `apps/core/tests/` or `apps/core/src/...` integration module): register a schema for `event_type=order.created`; set tenant to `Strict` via the new admin route; POST a non-conforming event to `/api/v1/events` → assert HTTP 422 with a `schema_violation` body; flip to `Permissive` via the route → same event now returns 200. Flip to `Warn` → 200 with a warn log.
- Query Service test: authenticated `PATCH .../tenants/me/settings` with `strict` returns 200 and a subsequent non-conforming ingest through the gateway is rejected; a different tenant's enforcement is unaffected (isolation).
- **Acceptance criteria (observable):** a tenant who has only an API key and the dashboard can switch their enforcement mode and immediately observe non-conforming writes being rejected (Strict) or warned (Warn), with no operator DB access and no Core restart.

**Product + GTM**
- Comparison-page row: change the "Schema enforcement at write" line for AllSource from "schemas exist but not strictly enforced" to "opt-in strict mode (per-tenant)". Update `apps/web/src/app/(marketing)/compare/agent-memory/page.tsx` and the internal `neotoma-comparison.md` head-to-head table (row currently marked ⚠ at `neotoma-comparison.md:72`).
- Docs: add a "Schema enforcement" section to `apps/web/src/app/(marketing)/docs/prime/` (or the Core/schemas doc) showing how to register a schema and flip to Strict, with the 422 body example. This is the artifact a regulated-industry buyer reads.

**Effort + risk**
- Doc said Small for the whole gap. Enforcement is done; the toggle is genuinely Small (1–2 days incl. dashboard + tests). **Risk if not shipped:** the feature exists but is invisible/unusable to self-serve tenants — we built it and still lose the "the system enforces the model" conversation because the buyer can't turn it on.

### Cross-cutting (Gaps 1, 2, 4 reach) — Expose projections, provenance, and templates beyond MCP

This is the largest genuinely-open item and the one the original doc missed. The primitives are built and tested; they are reachable only via `prime-mcp`. A buyer evaluating through the REST API or an SDK sees none of them. Per CLAUDE.md, **all SDKs must stay in sync** and **new public client surface goes through the gateway**, not Core directly.

**Engineering — Core REST routes (internal; gateway fronts them publicly)**
Add to `apps/core/src/infrastructure/web/prime_api.rs` (`prime_router`, currently `:29`–`52`), nested under `/api/v1/prime`:
- `POST /api/v1/prime/projections` — body `{ "entity_type": "...", "field_policies": { "<field>": "last_write|highest_priority|most_specific|merge_array" } }` → persists via `Prime::define_projection` (`facade.rs:1546`). Response `{ entity_type, replaced, persisted }`.
- `GET /api/v1/prime/projections` — returns all defs (calls `load_projection_defs`, `facade.rs:1574`). Response `{ "projections": [...] }`.
- `GET /api/v1/prime/nodes/{id}/snapshot` — folds observations via `fold` → `{ entity_type, fields, observation_count }` (mirror `call_project_node`, `tools.rs:452`).
- `GET /api/v1/prime/nodes/{id}/fields/{field}/provenance` — the route the doc named (`neotoma-comparison.md:132`). Returns `{ field, value, source_event_id, source_event_at, merge_policy_applied }` via `provenance_for_field` (`declarative.rs:288`). `as_of` query param is a stretch (see Open Q3).
- `GET /api/v1/prime/templates` and `GET /api/v1/prime/templates/{name}` — only if templates move into Core (see Gap 4 note below). Otherwise templates stay MCP-only and the SDK ships a static bundled copy.

**Data model:** none new. All four routes are read/replay over existing events; the only write is `prime.projection.defined` (already exists, `facade.rs:1557`). The REST handlers must replace the MCP in-process registry cache with a per-request `load_projection_defs` call **or** instantiate the same hydrate-on-startup cache inside the Core process. For hosted multi-tenant Prime, the cache must become **per-tenant** — the current single-tenant `RwLock<HashMap>` (`projection_registry.rs:1` header) is correct for stdio Prime but wrong for hosted. Recommend: in Core's HTTP path, skip the static cache and call `load_projection_defs` scoped to the request's tenant (Core events are already tenant-scoped via per-event `tenant_id`); keep the in-process cache only in the stdio MCP binary.

**Engineering — Gateway (Query Service)**
- Proxy the five GET/POST routes above under the gateway's existing Prime passthrough (look for how the Query Service forwards `/api/v1/prime/*` today; mirror that). Auth + rate-limit at the gateway; Core stays internal-only.

**Engineering — SDKs (Rust/TS/Py/Go, all four)**
Add matching methods to each SDK in `sdks/`:
- `define_projection(entity_type, field_policies)` / `definePrimeProjection`
- `list_projections()`
- `project_node(node_id)` → snapshot
- `field_provenance(node_id, field)` → provenance
- (templates) `list_templates()` / `load_template(name)` if templates get a REST route; else ship the 8 templates as a static bundled constant in each SDK.
Files: each SDK's Prime client module (`sdks/rust/src/...`, `sdks/typescript/src/...`, `sdks/python-client/...`, `sdks/go/...`). Per CLAUDE.md, SDKs are standalone HTTP clients — they call the gateway URL, never import Core/Prime crates.

**Engineering — MCP**
No new tools needed — `prime_define_projection`, `prime_list_projections`, `prime_project_node`, `prime_node_provenance`, `prime_list_templates`, `prime_load_template` already exist (`tools.rs:341`–`346`). Keep them; the REST routes are additive, not a replacement.

**Tests**
- Core: integration tests in `apps/core/tests/` hitting the new HTTP routes — define a projection over `POST /api/v1/prime/projections`, ingest 3 observations with differing `source_priority`/`observed_at`/`specificity_score`, then assert (a) `GET .../snapshot` returns the correctly-merged field set per policy, and (b) `GET .../fields/{field}/provenance` credits the expected `source_event_id` for a `highest_priority` field and 404/empty for a `merge_array` field. These mirror the existing pure-fold tests (`declarative.rs:391`,`494`,`523`) but prove the wiring end-to-end over HTTP.
- SDK: one round-trip test per SDK against a running Core/gateway (or the existing SDK integration harness) — define → project → provenance.
- **Acceptance criteria (observable):** a developer using only the TypeScript SDK (no MCP, no Claude Desktop) can register a projection, read an entity's merged snapshot, and ask "where did `status` come from?" and get the source event — proving the Neotoma-parity primitives are reachable on the same axis a Neotoma REST/CLI user would use.

**Product + GTM**
- Docs: add a "Declarative projections & provenance" page under `apps/web/src/app/(marketing)/docs/prime/` with copy-paste SDK + curl examples. This is what makes Gap 1/2 *visible* — today they're invisible outside MCP.
- Comparison page: the `/compare/agent-memory` event-sourced column already claims "full audit trail — every fact is preserved with provenance" (`page.tsx:107`). Add one concrete line: "ask `provenance(entity, field)` and get the source event — via MCP, REST, or any SDK." Keeps the claim honest now that it's true on every surface.
- Internal: update `neotoma-comparison.md` rows for "Per-field provenance" (`:70`) and "Declarative merge policies" (`:71`) from ✗ to ✓.

**Effort + risk**
- Doc estimated Gap 1 at 1–2 weeks and Gap 2 at 1 week assuming the primitives didn't exist. They do. The remaining REST+4-SDK+docs work is **Medium: ~3–5 days** total (routes are thin wrappers over tested functions; the bulk is 4 SDKs × test). **Risk if not shipped:** we lose the parity argument to anyone not on MCP — i.e. every SDK and REST evaluator — despite having built the hard part.

### Gap 4 (optional follow-up) — Templates as one-call registrable schemas

Today templates are descriptive MCP guides (`templates.rs:11`), not registerable into Core's `/schemas`. The doc's framing ("register with one call ... like database migrations") implies they seed the schema registry. This is **optional** and depends on Gap 3 adoption: only meaningful for tenants in Strict mode who want the templates to *be* their enforced schemas.

**Engineering (only if pursued):** add `POST /api/v1/prime/templates/{name}/register` (gateway-fronted) that converts the template's `properties` into a JSON Schema and registers it via the existing `register_schema` path (`api.rs:1142`). Reuse the 8 JSON files in `apps/prime-mcp/templates/` — but to register into Core they must be reachable from Core, so either move the template JSON into a shared `crates/` location both binaries embed, or duplicate them into Core (templates are small static data; CLAUDE.md isolation forbids one app importing another app's source, so a `crates/prime-templates` crate is the clean home if shared).
**Tests:** register `task` template into Strict tenant; assert a `task` event missing the required `title` (`templates.rs:121` proves `title` is required) is rejected.
**Decision:** defer until a tenant actually asks for enforced templates. Mark as a follow-up, not part of the core parity push.

---

## 3. Sequenced delivery plan

The doc's order (5→6→4→3→1→2) optimized ground-per-effort assuming everything was greenfield. The re-audit changes the picture: 5, 6 are done; 1, 2, 4-core are done at the engine layer. So the live sequence is short, and the dependency that matters is **the toggle (Gap 3) and the REST/SDK reach are independent of each other** but both depend on nothing — both are pure wiring over shipped logic.

Recommended order:

1. **Gap 3 toggle API + dashboard control** *(independent, ~1–2 days)*. Smallest, unblocks the regulated-industry conversation immediately, and the enforcement engine is already proven. Ship first because it's the cheapest remaining "make it usable" win.
2. **Cross-cutting REST routes on Core (Gaps 1/2 reach)** *(independent of #1, ~1–2 days)*. Thin handlers over tested fold/provenance functions. Do this before SDKs because the SDKs target these routes.
3. **Gateway proxy for the new Prime routes** *(depends on #2)*. Required before SDKs can authenticate against them.
4. **SDK methods, all four languages (Gaps 1/2 reach)** *(depends on #3, ~2–3 days)*. Parallelizable across the four SDKs once the gateway contract is frozen. This is the item that actually closes the parity gap for non-MCP buyers.
5. **Docs + comparison-page/internal-doc copy updates** *(depends on #1 and #4 landing; ~half-day)*. Makes everything visible. Pairs naturally with each shipping item — update the relevant row as each lands rather than batching.
6. **Gap 4 enforced-templates follow-up** *(deferred — only if a tenant asks; depends on #1 being adopted in Strict mode)*.

**Parallelization:** Steps 1 and 2 can run concurrently (different files, no shared surface). Step 4's four SDKs parallelize once Step 3 freezes the contract. Total critical path: ~1 week, comfortably inside the doc's "3–4 weeks" because the engine work is already done.

**Why not keep the doc's order?** Gaps 5/6 are shipped (would be no-ops). Gap 4-core is shipped. Putting Gap 1 last (as the doc did, at 1–2 weeks) is wrong now — its engine exists; only its reach is open, and that reach is the single highest-value remaining item because it touches every non-MCP evaluator. The toggle (Gap 3) goes first only because it's even smaller and self-contained.

---

## 4. Explicit non-goals (defend, do not chase)

Restating the three deliberate non-targets from `neotoma-comparison.md` §"Where AllSource should defend, not chase". **No work item in this plan touches these, and no executor should propose closing them:**

1. **No vectors-by-design is Neotoma's bet, not ours to copy.** AllSource keeps hybrid recall (vectors + graph + recency, `prime_recall`). Do **not** drop or de-emphasize vectors to look more "deterministic."
2. **SQLite-local-by-default is their lane.** Keep Prime's local stdio mode first-class (the developer-on-laptop story), but do **not** pivot Core to SQLite or position local-first as the primary mode. Our bet is hosted multi-tenant + local supported.
3. **Developer-preview maturity framing is theirs.** Do **not** dilute the "shipped, paying tenants, 469K events/sec, 11.9μs reads" positioning to match their alpha latitude.

Also not chased (and not a documented gap): **content-addressed Source dedup.** Neotoma's `Source` primitive has no first-class AllSource equivalent; we cover provenance via event metadata. Only revisit if a buyer specifically needs content-addressed source storage.

---

## 5. Open questions (human decision required before building)

1. **Gap 3 admin auth boundary.** What is the existing gateway→Core trust mechanism for internal admin calls? The new `PATCH /api/v1/admin/tenants/{id}` on Core must reuse it (Core never authenticates public callers per CLAUDE.md). If no internal-admin pattern exists yet, that's a prerequisite to design, not assume.
2. **Should tenant-settings management live in the SDKs at all?** Current SDKs appear to be event/query clients with no tenant-admin surface. Adding `set_schema_enforcement` to all four SDKs may be scope creep — a dashboard + gateway concern. Default recommendation: dashboard-only, no SDK methods for the toggle. Confirm.
3. **`as_of` provenance.** The doc's proposed provenance route included `?as_of=...` (`neotoma-comparison.md:132`). The shipped `provenance_for_field` folds the *current* observation set; time-bounded provenance ("who set this field as of last Tuesday?") needs the fold restricted to `observed_at <= as_of`. Is point-in-time provenance in scope for v1 of the REST route, or a fast-follow? It's a natural extension (filter observations before fold) but adds test surface.
4. **Templates home if Gap 4 enforced-registration is pursued.** The 8 template JSON files live in `apps/prime-mcp/templates/`. CLAUDE.md isolation forbids Core importing from another app. If templates must be registerable into Core's schema registry, do we (a) promote them to a shared `crates/prime-templates` crate both binaries embed, or (b) keep them MCP-only and have SDKs bundle a static copy? Recommend (a) only if Gap 4 enforced-registration is greenlit; otherwise leave as-is.
5. **ChatGPT in the cross-tool list.** The comparison page names ChatGPT (`page.tsx`-level copy) but the `/prime` setup guide covers Claude Desktop, Claude Code, Cursor, OpenCode — not ChatGPT. Either add a ChatGPT/custom-connector setup snippet to `/prime` or trim the claim. Pure copy decision.
