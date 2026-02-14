# AllSource Post-v0.10.0: C4 Architecture Analysis

## C4 Level 1 — System Context

```
┌─────────────────────────────────────────────────────────────────────┐
│                         EXTERNAL ACTORS                             │
│                                                                     │
│  [App Developer]    [Platform Admin]    [Billing System]            │
│   Uses events API    Manages tenants     LemonSqueezy webhooks      │
│   via Query Service  via Control Plane   via Query Service          │
└────────┬───────────────────┬───────────────────┬────────────────────┘
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Query Service  │ │  Control Plane  │ │   MCP Server    │
│  (Elixir:3902)  │ │  (Go:3901)      │ │  (Rust:3904)    │
│                 │ │                 │ │                 │
│ API Gateway     │ │ Admin Console   │ │ LLM Tool Use    │
│ Auth (OAuth)    │ │ Operations      │ │                 │
│ Billing/Quotas  │ │ RBAC Policies   │ │                 │
│ Tenant Scoping  │ │ Audit Trail     │ │                 │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────┬───────┴───────────────────┘
                     ▼
          ┌─────────────────────┐
          │    AllSource Core   │
          │    (Rust:3900)      │
          │                     │
          │  Event Store (WAL)  │
          │  Parquet + DashMap  │
          │  + Tenant/Audit/    │
          │    Config metadata  │
          └─────────────────────┘
```

---

## C4 Level 2 — Container Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        AllSource Platform                               │
│                                                                         │
│  ┌──────────────────────┐          ┌──────────────────────┐            │
│  │   Query Service      │          │   Control Plane      │            │
│  │   (Elixir/Phoenix)   │          │   (Go/Gin)           │            │
│  │                      │          │                      │            │
│  │ ┌──────────────────┐ │          │ ┌──────────────────┐ │            │
│  │ │ OAuth (Google/GH)│ │          │ │ JWT Auth + RBAC  │ │            │
│  │ │ JWT (Guardian)   │ │          │ │ (4 roles)        │ │            │
│  │ └──────────────────┘ │          │ └──────────────────┘ │            │
│  │ ┌──────────────────┐ │          │ ┌──────────────────┐ │            │
│  │ │ Tenant Context   │◄┼── GAP ──┼►│ Tenant CRUD      │ │            │
│  │ │ (own PG tenants) │ │  No sync │ │ (delegates Core) │ │            │
│  │ └──────────────────┘ │          │ └──────────────────┘ │            │
│  │ ┌──────────────────┐ │          │ ┌──────────────────┐ │            │
│  │ │ Usage Metering   │ │          │ │ Policy Engine    │ │            │
│  │ │ Quota Enforce    │ │          │ │ (in-memory)      │ │            │
│  │ │ Billing (Lemon)  │ │          │ └──────────────────┘ │            │
│  │ └──────────────────┘ │          │ ┌──────────────────┐ │            │
│  │ ┌──────────────────┐ │          │ │ Operations Mgmt  │ │            │
│  │ │ Consistency      │ │          │ │ Snapshot/Compact │ │            │
│  │ │ Routing          │ │          │ │ Replay/Backup    │ │            │
│  │ │ (leader/follower)│ │          │ └──────────────────┘ │            │
│  │ └──────────────────┘ │          │ ┌──────────────────┐ │            │
│  │ ┌──────────────────┐ │          │ │ Audit Trail      │ │            │
│  │ │ Analytics Engine │ │          │ │ (delegates Core) │ │            │
│  │ └──────────────────┘ │          │ └──────────────────┘ │            │
│  └──────────┬───────────┘          └──────────┬───────────┘            │
│             │                                 │                        │
│             │  HTTP (events, queries,         │  HTTP (tenants, audit, │
│             │  projections, schemas)          │  config, operations)   │
│             │                                 │                        │
│             └────────────┬────────────────────┘                        │
│                          ▼                                             │
│             ┌──────────────────────┐                                   │
│             │    AllSource Core    │                                   │
│             │    (Rust/Axum)       │                                   │
│             │                     │                                   │
│             │  Event Data:        │    Metadata (event-sourced):      │
│             │  ├─ WAL (durability)│    ├─ /api/v1/tenants   ◄─ CP    │
│             │  ├─ Parquet (cold)  │    ├─ /api/v1/audit     ◄─ CP    │
│             │  ├─ DashMap (hot)   │    ├─ /api/v1/config    ◄─ CP    │
│             │  │                  │    ├─ /api/v1/auth      ◄─ CP    │
│             │  ├─ Events API      │    └─ /api/v1/stats            │
│             │  ├─ Projections     │                                   │
│             │  ├─ Schemas         │    Ops:                           │
│             │  ├─ Snapshots       │    ├─ /api/v1/compaction ◄─ CP   │
│             │  ├─ Pipelines       │    ├─ /api/v1/replay     ◄─ CP   │
│             │  └─ Analytics       │    └─ /api/v1/snapshots  ◄─ CP   │
│             └──────────────────────┘                                   │
│                                                                         │
│  ┌──────────────────────┐                                              │
│  │   PostgreSQL         │  ◄── Query Service ONLY                     │
│  │   (users, tenants,   │      Control Plane removed PG in v0.10.0   │
│  │    subscriptions,    │                                              │
│  │    billing, usage)   │                                              │
│  └──────────────────────┘                                              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## C4 Level 3 — Component Diagram: Responsibility Overlap & Gaps

```
                    TENANT LIFECYCLE (THE BIG GAP)
    ╔═══════════════════════════════════════════════════════════╗
    ║                                                           ║
    ║   Query Service (PG)         Control Plane (→Core)       ║
    ║   ─────────────────          ──────────────────────      ║
    ║   Tenant model:              Tenant model:               ║
    ║   • name, slug               • ID, Name, Description    ║
    ║   • subscription_id          • Status (active/susp/del) ║
    ║   • tier (free→enterprise)   • Metadata map             ║
    ║   • usage counters           • CreatedAt, UpdatedAt     ║
    ║   • overage billing                                      ║
    ║   • trial dates              NO billing fields           ║
    ║   • LemonSqueezy IDs        NO usage tracking            ║
    ║                              NO subscription tier        ║
    ║                                                           ║
    ║   ⚠️  NO SYNC BETWEEN THESE TWO TENANT STORES            ║
    ║   ⚠️  Creating tenant in CP does NOT create in QS         ║
    ║   ⚠️  Creating tenant in QS does NOT create in CP/Core    ║
    ╚═══════════════════════════════════════════════════════════╝

                    AUTH (THE SECOND GAP)
    ╔═══════════════════════════════════════════════════════════╗
    ║                                                           ║
    ║   Query Service              Control Plane     Core      ║
    ║   ─────────────              ──────────────    ────      ║
    ║   OAuth (Google/GH)          JWT validation    Auth API  ║
    ║   Guardian JWT (HS512)       own JWT signing   register  ║
    ║   User model in PG           proxies to Core   login     ║
    ║   auto-creates tenant        RBAC (4 roles)    API keys  ║
    ║                                                           ║
    ║   ⚠️  THREE separate auth systems, no SSO                 ║
    ║   ⚠️  QS users ≠ CP users ≠ Core users                   ║
    ║   ⚠️  JWT secrets may differ between services             ║
    ╚═══════════════════════════════════════════════════════════╝
```

---

## DBaaS Function Map — Who Owns What

```
┌────────────────────────┬──────────┬──────────┬──────────┬──────────┐
│ DBaaS Function         │   Core   │ Query Svc│ Ctrl Pln │  STATUS  │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ EVENT STORAGE          │          │          │          │          │
│ ├─ Ingest events       │    ✅    │  proxy   │          │ GOOD     │
│ ├─ Query events        │    ✅    │  proxy   │          │ GOOD     │
│ ├─ Batch ingest        │    ✅    │  proxy   │          │ GOOD     │
│ ├─ Event streaming     │    ✅    │  proxy   │          │ GOOD     │
│ └─ WAL/Parquet durable │    ✅    │          │          │ GOOD     │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ DATA FEATURES          │          │          │          │          │
│ ├─ Projections         │    ✅    │  proxy   │          │ GOOD     │
│ ├─ Schemas             │    ✅    │  proxy   │  proxy   │ GOOD     │
│ ├─ Snapshots           │    ✅    │  proxy   │  proxy   │ GOOD     │
│ ├─ Analytics           │    ✅    │ extended │          │ GOOD     │
│ └─ Pipelines           │    ✅    │          │          │ GOOD     │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ MULTI-TENANCY          │          │          │          │          │
│ ├─ Tenant CRUD         │    ✅    │  ✅ PG   │  ✅→Core │ ⚠️ DUAL  │
│ ├─ Tenant isolation    │  by key  │  by ctx  │          │ GOOD     │
│ ├─ Tenant billing      │          │    ✅    │          │ QS only  │
│ ├─ Tenant quotas       │  stores  │ enforces │  proxy   │ SPLIT    │
│ ├─ Tenant stats        │    ✅    │          │  proxy   │ GOOD     │
│ └─ Tenant suspend/del  │    ✅    │          │    ✅    │ ⚠️ DUAL  │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ AUTH & ACCESS           │          │          │          │          │
│ ├─ User registration   │    ✅    │  proxy   │          │ OK       │
│ ├─ OAuth (social)      │          │    ✅    │          │ QS only  │
│ ├─ API keys            │    ✅    │          │          │ Core     │
│ ├─ JWT issuance        │          │    ✅    │    ✅    │ ⚠️ DUAL  │
│ ├─ RBAC enforcement    │   basic  │          │    ✅    │ CP only  │
│ └─ Policy engine       │          │          │    ✅    │ CP only  │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ OPERATIONS             │          │          │          │          │
│ ├─ Compaction          │    ✅    │          │  trigger │ GOOD     │
│ ├─ Snapshot creation   │    ✅    │          │  trigger │ GOOD     │
│ ├─ Replay              │    ✅    │          │  trigger │ GOOD     │
│ ├─ Backup              │          │          │  ✅(API) │ ⚠️ STUB? │
│ └─ Operation history   │          │          │  in-mem  │ ⚠️ LOST  │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ OBSERVABILITY          │          │          │          │          │
│ ├─ Health checks       │    ✅    │    ✅    │    ✅    │ GOOD     │
│ ├─ Prometheus metrics  │    ✅    │    ✅    │    ✅    │ GOOD     │
│ ├─ Audit trail         │    ✅    │          │  ✅→Core │ GOOD     │
│ ├─ OpenTelemetry       │          │          │    ✅    │ CP only  │
│ └─ Cluster health      │  /health │  checks  │  agg.   │ GOOD     │
├────────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ CLUSTER MANAGEMENT     │          │          │          │          │
│ ├─ Leader election     │ internal │          │          │ Core     │
│ ├─ Failover notify     │          │ receives │          │ QS       │
│ ├─ Read routing        │          │    ✅    │          │ QS       │
│ ├─ Consistency modes   │          │    ✅    │          │ QS       │
│ ├─ Replication lag     │          │  tracks  │          │ QS       │
│ └─ Config management   │  stores  │          │  ✅→Core │ CP→Core  │
└────────────────────────┴──────────┴──────────┴──────────┴──────────┘

Legend: ✅ = owns/implements, proxy = passes through, →Core = delegates to Core
        ⚠️ = gap or overlap needing resolution
```

---

## Critical Gaps Identified

### Gap 1: Dual Tenant Store (HIGH)

```
                   ┌─────────────┐
  OAuth signup ──► │ Query Svc   │──► PostgreSQL tenant row
                   │ (port 3902) │    (billing, usage, tier)
                   └─────────────┘
                         ✗ no notification to ──►  Core tenant store

                   ┌─────────────┐
  Admin API ────► │ Control Pln │──► Core /api/v1/tenants
                   │ (port 3901) │    (ID, status, metadata)
                   └─────────────┘
                         ✗ no notification to ──►  QS PostgreSQL

  RESULT: A tenant can exist in QS but not Core, or in Core but not QS.
  Quotas set via CP don't propagate to QS enforcement.
  Suspension via CP doesn't block QS requests.
```

### Gap 2: Auth Fragmentation (MEDIUM)

```
  Core:     /api/v1/auth/register + /login → issues token (algo?)
  QS:      OAuth → Guardian JWT (HS512, 1hr TTL) → stored in PG
  CP:      validates JWT with own secret → RBAC roles

  No shared identity provider. No token exchange.
  A QS JWT won't authenticate to CP and vice versa.
```

### Gap 3: Operation History Volatility (LOW)

```
  CP tracks operations (snapshot, compaction, replay) in MemoryOperationRepository.
  On CP restart → all operation history is lost.
  Should delegate to Core like tenant/audit/config already do.
```

### Gap 4: Missing CP <-> QS Communication (MEDIUM)

```
  Current interactions:
    QS ──► CP: cluster_health() only
    CP ──► QS: nothing

  Missing:
    CP tenant suspend → QS should block requests for that tenant
    CP quota update → QS should update enforcement limits
    CP policy change → QS should apply new policies
    QS usage data → CP should see billing/usage metrics
```

---

## Recommended Target Architecture (C4 Level 2)

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AllSource DBaaS Platform                        │
│                                                                     │
│  ┌────────────────────┐         ┌────────────────────┐             │
│  │   Query Service    │         │   Control Plane    │             │
│  │   (Data Plane)     │         │   (Mgmt Plane)     │             │
│  │                    │         │                    │             │
│  │ • Event API proxy  │         │ • Tenant lifecycle │             │
│  │ • Consistency route│◄───────►│ • RBAC + Policies  │             │
│  │ • Read-your-writes │  events │ • Operations mgmt  │             │
│  │ • Analytics        │  or     │ • Audit trail      │             │
│  │ • Rate limiting    │  shared │ • Config mgmt      │             │
│  │                    │  auth   │ • Cluster overview  │             │
│  │ REMOVES:           │         │                    │             │
│  │ ✗ own tenant store │         │ KEEPS:             │             │
│  │ ✗ own user store   │         │ • Policy engine    │             │
│  │                    │         │ • Ops scheduling   │             │
│  │ KEEPS:             │         │                    │             │
│  │ • Billing (Lemon)  │         │ ADDS:              │             │
│  │ • Usage metering   │         │ • Notifies QS on   │             │
│  │ • Quota enforce    │         │   tenant changes   │             │
│  └─────────┬──────────┘         └─────────┬──────────┘             │
│            │                              │                        │
│            └──────────┬───────────────────┘                        │
│                       ▼                                            │
│            ┌──────────────────────┐                                │
│            │    AllSource Core    │                                │
│            │  (Single Source of   │                                │
│            │   Truth for ALL)     │                                │
│            │                     │                                │
│            │ Events + Tenants +  │                                │
│            │ Users + Auth +      │                                │
│            │ Audit + Config      │                                │
│            └──────────────────────┘                                │
│                                                                     │
│  PostgreSQL ──► billing/subscription metadata ONLY                 │
│                 (LemonSqueezy IDs, overage tracking)               │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Summary of Gaps by Priority

| # | Gap | Severity | Impact |
|---|-----|----------|--------|
| 1 | **Dual tenant store** — QS (PG) and CP (Core) don't sync | HIGH | Tenant created via OAuth doesn't exist in Core; CP suspension doesn't affect QS |
| 2 | **Auth fragmentation** — 3 separate JWT/auth systems | MEDIUM | No SSO; tokens aren't portable across services |
| 3 | **No CP->QS eventing** — CP state changes don't propagate | MEDIUM | Quota changes, suspensions, policy updates invisible to QS |
| 4 | **Operation history in-memory** — CP MemoryOperationRepo | LOW | Op history lost on restart; inconsistent with audit/config being Core-backed |
| 5 | **Backup is a stub** — CP exposes `/backup` route but no implementation | LOW | Admin API promises backup but doesn't deliver |

The v0.10.0 changes correctly moved CP off PostgreSQL toward Core-as-source-of-truth, but the **Query Service still maintains a parallel tenant/user store in PostgreSQL** that isn't synchronized. The next architectural milestone should unify tenant identity through Core, with QS keeping only billing-specific columns (LemonSqueezy IDs, usage counters) as a thin cache that subscribes to Core tenant events.
