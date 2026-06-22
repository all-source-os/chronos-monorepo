# Runbook: "Data is synced but I can't see it" — diagnosing pipeline visibility

**Use when:** an agent/client writes data that is confirmed stored (events ingested, tool calls returned IDs), but a downstream surface (dashboard tab, API, report) shows nothing. The classic shape: *"Prime says it saved 24 nodes, the Memory tab says 'No memory yet'."*

This runbook exists because a single instance of this class took far longer than it should have — the data was fine the whole time; **five independent things in the read/identity path were each capable of producing an empty screen.** The goal here is to reach the real cause in minutes, not hours, and to avoid the specific traps that cost time.

---

## 1. The mental model: every hop re-resolves identity, tenant, and store

A write and a read are **not** symmetric. Draw the actual path and mark, at every hop, three things that can independently differ between the write side and the read side:

```
 writer ──> gateway A ──> Core (store)  <── gateway B <── frontend
   │            │            │                  │            │
 which       which        which              which        which
 identity?   tenant?      partition?         tenant?      identity?
```

The empty screen is almost never "the data is gone." It is almost always **the read path resolves a different (identity → tenant → partition) tuple than the write path.** Enumerate the path first; debug second. In this monorepo the canonical path is:

```
allsource-prime (laptop)
  --sync-to--> control-plane (api.all-source.xyz)  --ingest--> Core (allsource-core)
                                                                  ^
  dashboard --> web /api/events --> Query Service (allsource-query) --read--+
```

Two different gateways (**control-plane** writes, **Query Service** reads) touch the same Core. Anything that makes them resolve a different tenant makes the data invisible without any data loss.

---

## 2. Fastest-path diagnostic ladder (cheap → expensive)

Run these **in order** and stop at the first contradiction. Each step is one command; do not skip ahead to code reading until the cheap probes have localized the hop.

### Step 0 — Trust the empty-state copy, then distrust it
If the surface has a good empty state ("No memory is reaching **this tenant**"), it is telling you the read tenant has zero rows — *not* that no data exists. Treat "this tenant has nothing" and "nothing was written" as **different hypotheses** from the first second.

### Step 1 — Confirm the data exists at the store, via the WRITE path's gateway
```bash
curl -s "$WRITE_GATEWAY/api/v1/events/query?event_type_prefix=prime.&limit=1&order=desc" \
  -H "Authorization: Bearer $KEY" | python3 -m json.tool
```
Look at `total_count` and the **`tenant_id` field on a returned event**. This tells you (a) the data is durable and (b) the exact tenant string it is stored under. Write that string down — it is the source of truth for the rest of the investigation.

### Step 2 — Query the READ path's gateway with the SAME key
```bash
curl -s "$READ_GATEWAY/api/events?event_type_prefix=prime.&limit=1" \
  -H "Authorization: Bearer $KEY"
```
If write-gateway = N and read-gateway = 0 with the **same key**, the divergence is in the read gateway's identity/tenant resolution or in which store it reads. Proceed.

### Step 3 — Write a probe through each gateway, read it back through ALL gateways
This is the single most decisive test. It removes staleness, caching, and lazy-load as variables (the probe is brand new) and isolates the hop:
```bash
PROBE="node:probe:$(openssl rand -hex 3)"   # unique id; do NOT use Math.random in scripts that resume
# write via gateway A
curl -s -XPOST "$GW_A/api/v1/events" -H "Authorization: Bearer $KEY" \
  -H 'Content-Type: application/json' \
  -d "{\"event_type\":\"prime.node.created\",\"entity_id\":\"$PROBE\",\"payload\":{}}"
# read it back via A and via B
curl -s "$GW_A/.../query?entity_id=$PROBE" -H "Authorization: Bearer $KEY"   # expect 1
curl -s "$GW_B/.../events?entity_id=$PROBE" -H "Authorization: Bearer $KEY"  # 0 ⇒ A and B disagree
```
**Then do the reverse** (write via B, read via A and B). If each gateway sees only its own writes, you have narrowed it to "different tenant **or** different store" — but you have **not** yet distinguished them. See the trap in §3.

### Step 4 — Read the STORED `tenant_id` on a probe each gateway wrote
The discriminator that §3's symmetric result cannot give you:
```bash
curl -s "$GW_B/api/events?entity_id=$PROBE_WRITTEN_VIA_B" -H "Authorization: Bearer $KEY" \
  | python3 -c "import json,sys; e=json.load(sys.stdin)['data'][0]; print('stored tenant_id:', e['tenant_id'])"
```
- If gateway B stamped a **different tenant string** than Step 1's truth (e.g. `community` vs `acme-at-gmail-com`) → **same store, wrong tenant resolution in B.** Go to the code path that resolves tenant in B (§5).
- If both stamped the **same** tenant yet still can't see each other → **genuinely different stores.** Compare each service's Core URL (`env | grep -i core`, fly secrets, `/status/services` probe URLs).

### Step 5 — Only now read code, and read the *deployed* config, not just the source
Resolve the tenant assignment in the offending gateway top-down: auth pipeline → tenant-context plug → controller. Critically, check **runtime configuration / edition flags**, not only the happy-path code:
```bash
fly ssh console -a <app> -C "/bin/sh -lc 'env | grep -iE \"edition|tenant|core\"'"
```
A single env var (an "edition", a "mode", a "default tenant") can route every request down a branch that ignores the authenticated identity entirely.

---

## 3. Traps that cost time on this class of bug (read before you theorize)

1. **Declaring success off the wrong read path.** Verifying via the *write* gateway and calling it fixed is the cardinal error. Always verify through the **exact** path the user's surface uses (same gateway, ideally same session/cookie, not just an API key). If the user can see the screen and you're hitting a different endpoint, you have not verified anything.
2. **A symmetric "each sees only its own writes" result does NOT prove two stores.** It is equally explained by one store with two different tenant scopes. You must read the **stored `tenant_id`** (§4) to tell them apart. Two of this session's hours died in this ambiguity.
3. **Subagent / hypothesis hand-off without re-reading the data shape.** An investigator concluded "the DTO `id` is a random UUID" from source; the *actual* deployed endpoint returned `id = <slug>`. Verify a hypothesis against a real response body before acting on it, especially for serialization shape.
4. **Assuming a deploy shipped because the command exited 0.** A `fly deploy` reported success but created **no new release** (cache/no-op); the old code kept running and the symptom persisted. Confirm a *new release/version number* and, when in doubt, redeploy with `--no-cache`. Then re-test.
5. **Auth-skipped data planes resolve tenant from the request, not the token.** Core's `/api/v1/events*` is in `AUTH_SKIP_PREFIXES`: the Bearer token is ignored and the `?tenant_id=` **query param wins**. So "who the key is" matters only insofar as the *gateway* turns it into the right param. Don't assume the token scopes the query.
6. **Identity can be correct on one endpoint and wrong on another.** `/api/auth/me` returned the right tenant (it reads the JWT claim) while `/api/events` used the wrong one (it ran through a tenant-context plug with a different branch). Never generalize tenant resolution from one endpoint to the whole service.
7. **MCP launchers pass args verbatim.** `~`/`$HOME`/`${HOME}` in a `--data-dir` are **not** shell-expanded by Claude Desktop/DXT; a config value can send the process to a junk path. Inspect the *running* process args (`ps`), not the config file's intent.

---

## 4. Command cookbook (this stack)

```bash
# The key a local MCP/DXT is actually using (decoded, redacted), incl. its tenant claim:
PID=$(pgrep -f "allsource-prime" | head -1)
KEY=$(ps -o command= -p "$PID" | grep -oE -- '--api-key [A-Za-z0-9._-]+' | awk '{print $2}')
python3 -c "import json,base64,sys; p='$KEY'.split('.')[1]; p+='='*(-len(p)%4); print(json.loads(base64.urlsafe_b64decode(p)))"

# Where prime actually writes on disk + whether sync ran (cursor exists only after a successful push):
ls -la ~/.prime/memory; cat ~/.prime/memory/.prime_sync_cursor.json 2>/dev/null

# Which Core each service points at (run per app):
fly ssh console -a allsource-query        -C "/bin/sh -lc 'env | grep -i core'"
curl -s https://api.all-source.xyz/api/v1/status/services        # control-plane's view of backends
curl -s https://allsource-query.fly.dev/health                   # QS version + core node health

# Confirm a deploy actually shipped:
fly releases -a <app> | head -3                                  # a NEW version row must appear
```

Note: the control-plane container is distroless — `fly ssh -C` with `printenv`/`sh` fails. Use `/status/services` and `/health` to infer its backend instead.

---

## 5. Worked example — the five layered causes behind one empty Memory tab

All five were real; the first four were fixed and the screen was *still* empty, which is why the discipline above matters.

| # | Layer | Symptom it produced | Real fix |
|---|-------|---------------------|----------|
| 1 | UI minted API keys with role `service_account` (underscore) vs the canonical `serviceaccount` | every key 403'd on read+write at the gateway | `api_key_controller.ex` mint the canonical string |
| 2 | DXT `--data-dir ${HOME}/...` not expanded; cwd `/` | writes never persisted | prime expands `~`/`$HOME`/`${HOME}` (`0.21.6`) |
| 3 | Dashboard fetched latest-200 then client-filtered `prime.*` | prime events drowned by other types | server-side `event_type_prefix` filter |
| 4 | **Red herring**: `tenant_context.tenant_id/1` read `id` not `tenant_id` | *looked* like the bug; was a no-op because the real tenant endpoint returns `id = <slug>` | harmless hardening only |
| 5 | `ALLSOURCE_EDITION=community` on the hosted Query Service | `TenantContext` hardcoded every request to the `community` tenant; prime synced under the real slug → dashboard read `community` → empty | set `ALLSOURCE_EDITION=enterprise` on `allsource-query` |
| 6 | **The actual final blocker — a frontend double-unwrap.** `apiClient.request()` unwraps `{data:X}→X`; the events body `{count, data:[...]}` became the bare array, then `useEvents` did `.data` again → `undefined` → `events=[]` | the API returned 71 rows the whole time; only the *rendered page* was empty — invisible to every API-level test | `useEvents` treats the value as the already-unwrapped array |

The fast path would have reached the truth in: Step 1 (data exists under `acme-at-gmail-com`), Step 3+4 (QS *writes* land under `community`), Step 5 (`env | grep edition` → `community`) — then, crucially, **load the rendered page with the user's real session** and watch it stay empty while the network tab shows 71 rows, which points straight at the client. Everything else was time spent reading through the wrong path, assuming a deploy shipped, and (twice) declaring victory from an API call instead of the screen. The single highest-leverage habit: **verify through the user's exact rendered path, with their session, every time — an API that returns rows proves nothing about what the component renders.**

---

## 6. Definition of done

- The probe written through the **user's exact read path** comes back with a count > 0.
- The stored `tenant_id` on that probe matches the tenant the user is logged into.
- A **new** release/version is confirmed live (not assumed), and the symptom is re-tested against it.
- The fix is stated at the layer it actually lives (config/edition vs code), not the first plausible code path.

---

## 7. 2026-06-22 recurrence — the double-unwrap was never fully killed, plus a tenant/logout trap

Same "no data anywhere" report, three compounding causes — none was data loss (Core held **7,725 streams + 141 event-types + 500+ events** under `decebal-dobrica-at-gmail-com`, tier `studio`, the whole time):

1. **Wrong-tenant session.** The user's browser was pinned to an empty auto-provisioned tenant, not their data tenant. Proof that beat all guessing: `/api/tenant` returned a full record (`id`, tier, quota) for `decebal-dobrica-at-gmail-com` but `tenant?.id` rendered `—` in Settings → the session was elsewhere. `AuthPipeline` copies `tenant_id` **verbatim from the JWT claim** (`auth_pipeline.ex:108`); `TenantContext` then **silently auto-provisions an empty tenant + returns 200** when that id is unknown (`tenant_context.ex:141`) — a mismatch masquerades as "no data, no error." Ground truth came from prod logs: `fly logs -a allsource-query | grep UserSocket` printed `user_id` + `tenant_id` for the live session. Re-login with the Google account whose `TenantSlug(email)` equals the data tenant resolved it; no data was moved.
2. **Logout was a no-op**, so the user couldn't escape the wrong session. `auth-store.logout()` only cleared localStorage; the `auth_token` cookie survived → next `/api/auth/session` re-hydrated. Fixed: explicit expired Set-Cookie matching the callback's attributes (`session/route.ts` DELETE) + hard `window.location` reload + `localStorage.removeItem("auth-storage")` in the header.
3. **The double-unwrap from §5 #6 was only patched in `useEvents`.** Five more consumers still did `data?.data` on the already-unwrapped array and rendered empty **even on the correct tenant**: `useEventsByEntity`, `useStreams`, `useEventTypes`, `useReplays`, and `use-notification-preferences`. Hooks that did a single `response.data` (`pipelines/page.tsx`, `use-dashboard-stats`) were always fine. **Lesson: when you find a response-shape bug, grep the whole client for every consumer of that shape — `rg "data\?\.data"` — and fix them as a set, not the one that was reported.** Settings Tenant ID now falls back to `user?.tenant_id` (the JWT claim) so the user can always read their live tenant even when `/api/tenant` is empty.
