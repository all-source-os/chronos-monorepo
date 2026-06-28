# Hosted Hound Extraction — Metering & Hard-Gate (PARKED)

**Epic:** `t-57fca0` (billing) under parent `t-c9d2ef` · **Status: code-complete, NOT live.**
The full pipeline is built, tested, and merged to `main`. It does **nothing in
production yet** because the hosted LLM is not wired (two Fly secrets unset) and
the per-tier token amounts are unconfirmed placeholders. Picking this back up =
the "What's left" checklist below; no further engineering is required to turn it
on, only operator config + a billing-owner sign-off.

Design rationale: `docs/proposals/PRIME_HOUND_GRAPHIFY_ANALYSIS.md`.
Related: `docs/runbooks/PRICING_BILLING_CUTOVER.md`, `docs/proposals/PRICING_EXPOSURE_PLAN.md`.

---

## What this is

"Hosted Hound extraction" = AllSource provides the LLM that turns a tenant's
prose/PDF/image docs into graph nodes+edges (the `doc_extract` path in
`apps/prime-mcp`). The tenant brings **no** model key. Because AllSource pays for
those tokens, each pricing tier includes a capped **extraction-token allowance**,
metered per tenant, and **hard-gated**: once a tenant is over its allowance,
hosted extraction is blocked (HTTP 402). No money changes hands — pure
enforcement (this was the chosen model over $/token overage charging).

Tenants who don't want the cap can **bring their own LLM** (BYO): point
`PRIME_LLM_ENDPOINT` at their own OpenAI-compatible endpoint, which never touches
AllSource's proxy or the gate.

---

## End-to-end flow

```
                    PRIME_LLM_ENDPOINT                 EXTRACTION_LLM_URL
                    PRIME_LLM_API_KEY=ask_<key>        EXTRACTION_LLM_API_KEY
                          │                                  │
  prime-mcp ──POST chat───┼──▶ control-plane ──gate pass?──┬─┴─▶ real LLM (AllSource's)
  (doc_extract)           │    ProxyExtraction             │      OpenAI-compatible
       ▲                  │    HasExtractionQuota          │
       │                  │      │ over → 402               │
       │ choices+usage ◀──┴──────┴──────────────◀──────────┘  (response copied back)
       │
       └─ emits prime.extraction.usage event (total_tokens) ──▶ Core
                                                                  │
                          control-plane reconciler (5 min) ◀──────┘
                          sums tokens → tenant meter quotas.extraction_tokens_used
                                                                  │
                          feeds the NEXT HasExtractionQuota decision ◀┘  (loop closed)
```

Two URLs, two hops — do not conflate:
- **`PRIME_LLM_ENDPOINT`** (prime-mcp, client side) → where prime-mcp sends = the proxy.
- **`EXTRACTION_LLM_URL`** (control-plane, server side) → where the proxy forwards = AllSource's real model.

Metering is **single-sourced in prime-mcp** (it sees `usage` in the LLM response
and emits the event). The proxy deliberately does NOT emit usage, so there is no
double count.

---

## What's built (all on `main`)

| Piece | Commit | Where |
|---|---|---|
| prime-mcp emits `prime.extraction.usage` (token totals) per extraction | `d0cbdfa` | `apps/prime-mcp/src/doc_extract.rs` (`emit_usage`, `Usage{prompt,completion,total}`) |
| Reconciler sums tokens → `quotas.extraction_tokens_used` (5-min task, mirrors events/x402 reconcilers) | `346b27e` | `apps/control-plane/internal/application/usecases/billing/sync_extraction_usage.go` + scheduler task `extraction_usage_sync` |
| Per-tier allowance `ExtractionTokensQuota` on every tier + provisioning | `9f4b59c` | `internal/domain/entities/subscription.go` (`TierQuotaMap`), `update_subscription_metadata.go` |
| Gate primitive `HasExtractionQuota(tenantID)` (fail-open, like events gate) | `8d96524` | `internal/infrastructure/x402/quota_gate.go` |
| **Hosted-extraction proxy route** `POST /api/v1/extraction/chat/completions` (wires the gate) | `6c8e6e9` | `apps/control-plane/delegation.go` (`ProxyExtraction`, `forwardExtraction`) + route in `main.go` |

**Security properties (tested):** the server-side provider key
(`EXTRACTION_LLM_API_KEY`) is never returned to the tenant; the tenant's inbound
`ask_` key is never forwarded upstream (the proxy substitutes the provider key).

**Verification:** `gofmt` clean, `go build ./...` + `go vet` clean, **289 tests
green** across the `main` + `x402` packages. New `apps/control-plane/extraction_proxy_test.go`
proves: 401 without a tenant; 402 + **upstream LLM never hit** when over quota;
under-quota forward with provider-key substitution + OpenAI response copied back;
503 when `EXTRACTION_LLM_URL` is unset. Gate logic itself: 7 table cases in
`core_quota_checker_test.go`.

---

## Per-tier allowances (PLACEHOLDERS — confirm before launch)

`TierQuotaMap` in `internal/domain/entities/subscription.go`:

| Tier | `ExtractionTokensQuota` | Meaning |
|---|---|---|
| free | `0` | BYO only — hosted extraction blocked (402) |
| indie | `1_000_000` | 1M tokens / period |
| studio | `10_000_000` | 10M |
| scale | `100_000_000` | 100M |
| enterprise | `-1` | unlimited |

Amounts are sensible placeholders scaled to the existing event/query tiers. They
have **not** been validated against real LLM cost-per-token or target margins.

---

## What's left to finish (the un-parking checklist)

### A. Operator — turn it on (no code)
1. **Set the hosted LLM** on `allsource-control-plane` (Fly secrets):
   - `EXTRACTION_LLM_URL` = full OpenAI-compatible chat-completions URL of
     AllSource's model (e.g. `https://api.openai.com/v1/chat/completions`, or a
     self-hosted vLLM/gateway).
   - `EXTRACTION_LLM_API_KEY` = the provider bearer key for that endpoint.
   - `fly secrets set EXTRACTION_LLM_URL=... EXTRACTION_LLM_API_KEY=... -a allsource-control-plane`
   - Until both are set the route returns **503** (BYO still works).
2. **Document the tenant-facing config** (hosted mode) wherever Hound usage is
   documented:
   - `PRIME_LLM_ENDPOINT=https://api.all-source.xyz/api/v1/extraction/chat/completions`
   - `PRIME_LLM_API_KEY=ask_<tenant key>`
   - `PRIME_LLM_MODEL=<model>` (see decision B2).
3. **Smoke test in prod:** with a known tenant, drive one hosted extraction;
   confirm a `prime.extraction.usage` event lands in Core and the reconciler
   advances `quotas.extraction_tokens_used` within ~5 min. Then drive a tenant
   over a deliberately-tiny allowance and confirm the 402.

### B. Billing-owner — decisions (engineering is 1-line each, deferred on purpose)
1. **Confirm per-tier amounts** (table above) against real token economics. Edit
   `TierQuotaMap` if they change; `subscription_test.go` asserts the values.
2. **Model pinning (cost control):** today the proxy passes the tenant's
   `PRIME_LLM_MODEL` through verbatim — a tenant could request an expensive
   model on AllSource's dime. If hosted extraction should force a specific model,
   override `model` in `forwardExtraction` (`delegation.go`) before forwarding.
   One line; not done because it's a pricing/policy call, not a bug.
3. **Overage charging (explicitly NOT built):** current model is hard-gate (block
   at allowance, no charge). If the product instead wants to *charge* beyond the
   allowance, wire a `$/token` rate + `lsClient.ReportUsage` (the plumbing exists
   for events-overage). This was deliberately deferred — the chosen model is the
   hard gate.

### C. Nice-to-have follow-ups (not blocking)
- Surface remaining extraction allowance in the dashboard / `billing` summary
  (the meter + entitlement are already in tenant metadata, same shape as events).
- Alert when a tenant crosses ~80% of its extraction allowance.

---

## Files to reopen when un-parking

- `apps/control-plane/delegation.go` — `ProxyExtraction`, `forwardExtraction` (the proxy + model-pin point).
- `apps/control-plane/main.go` — route registration + `extractionGate` wiring.
- `apps/control-plane/internal/infrastructure/x402/quota_gate.go` — `HasExtractionQuota`.
- `apps/control-plane/internal/domain/entities/subscription.go` — `TierQuotaMap` amounts.
- `apps/control-plane/internal/application/usecases/billing/sync_extraction_usage.go` — the reconciler.
- `apps/prime-mcp/src/doc_extract.rs` — token emit (the metering source of truth).
