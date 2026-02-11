# Chronos SaaS Launch Roadmap - Indie Hacker Edition

**Date**: 2026-02-10
**Status**: PROPOSED
**Goal**: Get to first paying customers fast, iterate based on feedback

---

## What You Already Have (Don't Rebuild This)

| Component | Status | Notes |
|-----------|--------|-------|
| Multi-tenant isolation | ✅ Complete | Tenant entity, quotas, isolation |
| LemonSqueezy billing | ✅ Complete | Checkout, portal, webhooks, overage |
| Usage metering | ✅ Complete | Events/queries tracking, quotas |
| JWT auth | ✅ Complete | Control plane handles this |
| OAuth ready | ✅ Scaffolded | Google/GitHub in docker-compose |
| 726K events/sec | ✅ Complete | Performance is a moat |

---

## Phase 0: Ship What You Have (Week 1)

**Goal: Get to "I can take payments" state**

| Item | Priority | Effort | Why |
|------|----------|--------|-----|
| Deploy to Fly.io | P0 | 1 day | Stop theorizing, ship |
| Create LemonSqueezy products | P0 | 2 hrs | Free, Pro ($29), Team ($99) |
| Landing page on `/` | P0 | 1 day | Static HTML, no React. Hero + pricing + signup |
| Waitlist → OAuth flow | P0 | 1 day | Collect emails now, convert later |

**Anti-pattern to avoid:** Don't build a dashboard yet. API-first, dashboard later.

---

## Phase 1: First 10 Customers (Weeks 2-4)

**Goal: Someone pays you money**

| Item | Priority | Effort | Rationale |
|------|----------|--------|-----------|
| **Onboarding wizard API** | P0 | 2 days | `/api/onboard/start` → create tenant + API key + sample stream |
| **Quick start curl examples** | P0 | 4 hrs | `curl -X POST` to send first event. Put in welcome email |
| **Usage warning emails** | P1 | 1 day | 80% quota → email. Drives upgrades |
| **Stripe as backup** | P1 | 2 days | Some customers hate LS. Have both |
| **SDK: JavaScript** | P1 | 2 days | `npm install @chronos/client` with 3 methods |
| **API docs on `/docs`** | P1 | 1 day | Mintlify or Docusaurus. Devs judge by docs |

**What NOT to do yet:**
- Admin dashboard (your users have their own dashboards)
- Multi-region (premature optimization)
- SSO/SAML (enterprise, not MVP)
- GraphQL (REST is fine)

---

## Phase 2: Product-Market Fit (Months 2-3)

**Goal: 10 customers → 100 customers, understand churn**

| Item | Priority | Effort | Rationale |
|------|----------|--------|-----------|
| **Simple dashboard** | P1 | 1 week | Usage chart, API keys, upgrade button. That's it |
| **Webhook delivery** | P0 | 3 days | Push events to customer URLs. This is the killer feature |
| **Python SDK** | P1 | 2 days | 2nd most requested after JS |
| **Status page** | P1 | 2 hrs | Instatus.com free tier. Builds trust |
| **Changelog** | P1 | Setup | `/changelog` - shows you're shipping |
| **Feedback widget** | P1 | 2 hrs | Canny or simple email link |

---

## Phase 3: Scale Revenue (Months 3-6)

**Goal: $1K → $10K MRR**

| Item | Priority | Effort | Rationale |
|------|----------|--------|-----------|
| **Annual billing discount** | P1 | 1 day | 2 months free = better retention |
| **Team seats** | P2 | 3 days | Team tier: 5 seats included |
| **Audit logs** | P2 | 2 days | Enterprise asks for this |
| **Usage analytics for customers** | P2 | 3 days | Show them their event patterns |
| **Replay from UI** | P2 | 2 days | Event replay is a differentiator |
| **Go SDK** | P2 | 2 days | Cover the 3 main languages |

---

## What Makes Chronos Defensible (Your Moat)

1. **Performance** - 726K events/sec is enterprise-grade at indie prices
2. **Event sourcing native** - Not just a queue, actual projections + replay
3. **AI-native MCP** - Claude/GPT can query your event store directly
4. **Polyglot backend** - Rust speed, Elixir real-time, Go auth - right tool for each job

---

## Pricing Suggestion (Validated Pattern)

| Tier | Price | Events/mo | Target |
|------|-------|-----------|--------|
| **Free** | $0 | 10K | Hobbyists, eval |
| **Pro** | $29/mo | 500K | Solo devs, small apps |
| **Team** | $99/mo | 5M + 5 seats | Startups |
| **Scale** | $299/mo | 50M | Growing companies |
| **Enterprise** | Custom | Unlimited | Call us |

Overage: $1 per 100K events over quota (already built!)

---

## Immediate Actions (This Week)

1. **Create LemonSqueezy store** - Add 3 products matching your tiers
2. **Deploy query-service to Fly.io** - Use the `fly.toml` from the PRD
3. **Landing page** - Single `index.html` with: headline, 3 benefits, pricing table, "Get API Key" CTA
4. **Tweet about it** - "Building an event store in public. 726K events/sec. Sign up for early access."

---

## What to Explicitly NOT Build

| Don't Build | Why |
|-------------|-----|
| Custom auth system | OAuth is enough. No password reset emails to maintain |
| Admin super-dashboard | You have 0 customers. Use `psql` to manage |
| Multi-region | Fly.io can do this later with 1 config change |
| Kubernetes | Fly.io handles orchestration |
| Custom docs platform | Use Mintlify/GitBook |
| Mobile SDKs | Server-side events only for now |

---

## Success Metrics (Indie Hacker Focus)

| Metric | Week 1 | Month 1 | Month 3 |
|--------|--------|---------|---------|
| Signups | 50 | 200 | 500 |
| Activated (sent 1 event) | 10 | 50 | 150 |
| Paid | 0 | 5 | 25 |
| MRR | $0 | $150 | $1,500 |

---

## References

- Existing SaaS PRD: `tasks/prd-chronos-saas-mvp-production-ready-with-billing.md`
- Billing implementation: `apps/query-service/lib/query_service_ex_web/controllers/billing_controller.ex`
- Tenant management: `apps/query-service/lib/query_service_ex/tenants.ex`
- Consolidated roadmap: `docs/roadmaps/2026-02-02_CONSOLIDATED_ROADMAP.md`
