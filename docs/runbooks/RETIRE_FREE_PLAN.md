# Runbook: Retire the Free Plan (migrate existing free tenants)

**Status:** operator procedure — run by the fleet owner, on demand.
**Scope:** migrate the tenants that ALREADY carry `subscription.tier == "free"`.
**Prevention (already shipped, prompt 048):** no new code path mints a free
tenant — every self-service signup now starts a 14-day **trial** that the
scheduler suspends on expiry. This runbook is the one-time **cleanup** of the
free tenants that predate that change.

> **Do NOT** run any mutation from a prompt/agent. This document + the
> `task retire-free` task are the deliverable; a human operator runs them.
> The dry-run path mutates nothing.

---

## TL;DR

```bash
# 1. See what's out there (lists every free tenant, classifies litter vs real). MUTATES NOTHING.
task retire-free                 # DRY=true is the default

# 2. When you're happy with the classification, apply:
task retire-free DRY=false       # archives litter, comps the real ones to enterprise
```

Everything below explains what those two commands do, and how to run the steps
by hand if you'd rather not use the task.

---

## Why

The marketing site and `/api/v1/billing/catalog` already say "no free plan"
(the pricing page ships a "Why no free plan?" FAQ). But before prompt 048 the
backend handed out **permanent** `tier:"free"` on every self-service signup, so
free tenants kept appearing (`mc-logs`, `mantis-crab-logs`, …). Prompt 048
stopped NEW free at the source; this runbook reconciles the EXISTING ones.

The migration has two halves, in order:

1. **Reap the test litter** — duplicate / 0-event / epoch-suffix-named free
   tenants that were never real customers. Removed via the existing **guarded,
   dry-run-first** admin bulk action, with an `event_count < 50` safety backstop
   so a real tenant is never touched.
2. **Comp the survivors to enterprise** — the remaining free tenants are real
   users; move them to **comped enterprise** (`manual_override:true`) so they
   keep working (and aren't put on a trial clock they never agreed to), via
   `PUT /api/v1/tenants/:id`, preserving their existing quotas/billing linkage.

**Grandfathering invariant:** until you run this, existing free tenants keep
working unchanged. Core still defines `free_tier()` quotas and resolves them for
any tenant explicitly on `quota_preset:"free"`; the Control Plane's read-time
`defaultPlan` fallback still reads a tier-less legacy tenant as "free" (it does
NOT relabel them "trial"). Nothing in prompt 048 instantly breaks or relabels a
current free tenant — this runbook is the only thing that migrates them, and only
when you run it.

---

## Prerequisites

Same toolchain as the other operator tasks (`reap-demo`, `backfill-usage`):

- `fly`, `openssl`, `jq`, `curl`, `go-task` (`brew install go-task`).
- Read access to the fleet `JWT_SECRET` (pulled from the QS container by the
  task — admin is allowlist-gated, so we mint a short-lived admin JWT the same
  way every other operator task does).
- Your email is in the Control Plane `ADMIN_EMAILS` allowlist (the minted token
  is `role:"admin"` with your `EMAIL` as `sub`/`email`).

No new endpoint is introduced. The migration reuses:

| Step | Endpoint | Guard |
|------|----------|-------|
| list free tenants | `GET /api/v1/admin/tenants?plan=free&per_page=100` | admin, read-only |
| classify litter | (client-side, by `event_count` + name) | — |
| archive litter | `POST /api/v1/admin/tenants/bulk` `{action:"archive"}` | admin; reversible; blast-radius cap 100/run |
| comp survivor → enterprise | `PUT /api/v1/tenants/:id` | admin; merge-then-PUT (preserves other metadata) |

> **Archive, not delete.** `bulk archive` flips the tenant to `archived`
> (reversible) — the same status family as `suspend`. We deliberately do NOT
> hard-delete arbitrary tenants from this runbook: there is no admin "delete any
> tenant" route (only Core's admin `DELETE /api/v1/tenants/:id`), and archiving
> is enough to get litter out of the active fleet without destroying data. If you
> later confirm an archived tenant is genuinely empty and want it gone, delete it
> deliberately via Core `DELETE /api/v1/tenants/:id` — that's an explicit,
> out-of-band decision, not part of the bulk sweep.

---

## Litter classification (the safety model)

A free tenant is treated as **litter** (safe to archive) only when BOTH hold:

- `event_count < 50` — the **hard safety backstop**. Any tenant with ≥ 50 events
  is treated as REAL no matter what its name looks like, and is never archived.
- its name matches a test-litter shape — duplicate base name, an epoch/uuid
  suffix (e.g. `-1718000000`, `-a1b2c3d4`), or a known throwaway prefix
  (`onboard-`, `agent-`, `trial-`, `demo-`, `test`, `mc-`, `mantis-`).

Everything else — including any free tenant with ≥ 50 events — is **real** and
goes to the **comp-to-enterprise** step.

> The `event_count < 50` backstop is the line that makes this safe: even if the
> name heuristic is wrong, a tenant with real usage is never archived.

---

## Step-by-step (manual, if not using the task)

All commands assume you've minted a 1h admin token into `$TOKEN` exactly like
`reap-demo`/`backfill-usage` do (the task does this for you):

```bash
CP="https://api.all-source.xyz"
# $TOKEN = 1h admin JWT minted from the fleet JWT_SECRET (see reap-demo task)
```

### 1. List the free tenants (read-only)

```bash
curl -sS "$CP/api/v1/admin/tenants?plan=free&per_page=100" \
  -H "Authorization: Bearer $TOKEN" | jq '.tenants[] | {id, name, event_count, status, created_at}'
```

### 2. Classify (read-only)

For each tenant from step 1:

- `event_count >= 50` → **REAL** (→ step 4, comp to enterprise).
- `event_count < 50` AND name looks like litter → **LITTER** (→ step 3, archive).
- `event_count < 50` AND name looks real → **REAL** (be conservative; → step 4).

### 3. Archive the litter (dry-run first)

`bulk archive` takes up to 100 tenant ids per call. Preview, then apply:

```bash
# build the litter id list from step 2 into LITTER_IDS (JSON array), then:
curl -sS -X POST "$CP/api/v1/admin/tenants/bulk" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "{\"action\":\"archive\",\"tenant_ids\":${LITTER_IDS}}" | jq .
```

Archiving is reversible: `POST /api/v1/admin/tenants/:id/unsuspend` or a
`bulk` re-activate path restores an archived tenant if you misclassified one.

### 4. Comp the survivors to enterprise (merge, then PUT)

For each REAL free tenant, **read its current tenant first**, merge the comped
subscription into its existing metadata, then PUT the whole thing back (PUT
replaces the metadata blob, so you must merge to preserve quotas/billing linkage):

```bash
ID="<tenant-id>"
CUR=$(curl -sS "$CP/api/v1/tenants/$ID" -H "Authorization: Bearer $TOKEN")
NEW_META=$(printf '%s' "$CUR" | jq '.metadata * {
  subscription: ((.metadata.subscription // {}) * {
    tier: "enterprise",
    status: "active",
    plan_name: "Custom Enterprise",
    manual_override: true
  })
}')
curl -sS -X PUT "$CP/api/v1/tenants/$ID" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "$(jq -nc --argjson m "$NEW_META" '{metadata:$m}')" | jq '.metadata.subscription'
```

`jq`'s `*` deep-merges, so existing `metadata.quotas`, `metadata.subscriptions`
(LemonSqueezy linkage), and any other keys are preserved; only the
`subscription` tier/flags change.

---

## Verification

After `DRY=false` (or the manual steps):

```bash
# No free tenants should remain ACTIVE (the survivors are now enterprise,
# the litter is archived).
curl -sS "$CP/api/v1/admin/tenants?plan=free&status=active&per_page=100" \
  -H "Authorization: Bearer $TOKEN" | jq '.total'   # expect 0

# Spot-check a comped survivor reads as enterprise + manual_override.
curl -sS "$CP/api/v1/tenants/<survivor-id>" -H "Authorization: Bearer $TOKEN" \
  | jq '.metadata.subscription | {tier, manual_override, plan_name}'
```

Audit trail: archiving writes `tenant.bulk_archived` Core audit events; the comp
PUT is attributable to your admin token. Trial expiries (the ongoing prevention
side) write `billing.trial.expired` + `tenant.trial_expired`.

---

## Rollback

- **Archived a real tenant by mistake:** re-activate it
  (`POST /api/v1/admin/tenants/:id/unsuspend`); archiving never deletes data.
- **Comped the wrong tenant:** PUT it back to its prior subscription (you have
  the pre-merge `$CUR` from step 4; re-PUT `$CUR.metadata`).

Nothing in this runbook deletes data, so every step is reversible.
