"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  ChevronRight,
  CreditCard,
  Key,
  Layers,
  Lock,
  Scale,
  Settings,
  Shield,
  Users,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const features = [
  {
    title: "Tenant-Level Event Isolation",
    description:
      "Every event belongs to exactly one tenant. Queries are scoped by tenant ID at the storage engine level — not filtered after the fact. Cross-tenant data access is architecturally impossible.",
    icon: Lock,
    color: "from-violet-500/20 to-violet-500/5",
  },
  {
    title: "RBAC: Admin / Developer / ReadOnly / ServiceAccount",
    description:
      "Four built-in roles with seven granular permissions covering event ingestion, querying, schema management, projection access, and tenant administration.",
    icon: Users,
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    title: "Policy Engine with Custom Rules",
    description:
      "Go beyond role-based access with attribute-based policies. Restrict access by IP range, time window, event type pattern, or custom claims. Policies evaluate at request time with zero latency overhead.",
    icon: Scale,
    color: "from-fuchsia-500/20 to-fuchsia-500/5",
  },
  {
    title: "Per-Tenant Quota Enforcement",
    description:
      "Set ingestion rate limits, storage caps, and query budgets per tenant. Projections track usage in real-time. Tenants that hit their quota get HTTP 429 — other tenants are unaffected.",
    icon: Settings,
    color: "from-pink-500/20 to-pink-500/5",
  },
  {
    title: "x402 Pay-Per-Call Monetization",
    description:
      "Built-in HTTP 402 payment protocol support. Charge tenants per API call, per event ingested, or per query executed. Metering is a projection — no external billing system needed for simple models.",
    icon: CreditCard,
    color: "from-indigo-500/20 to-indigo-500/5",
  },
  {
    title: "API Key Management per Tenant",
    description:
      "Each tenant gets isolated API keys with configurable scopes and expiration. Rotate keys without downtime. Revoke compromised keys instantly across all services.",
    icon: Key,
    color: "from-blue-500/20 to-blue-500/5",
  },
];

export default function MultiTenantSaasPage() {
  return (
    <div className="relative overflow-hidden">
      {/* Hero */}
      <Section className="relative pt-24 pb-16 text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6 }}
        >
          <span className="inline-flex items-center gap-2 rounded-full border bg-background/50 px-4 py-1.5 text-sm backdrop-blur-sm">
            <Layers className="h-4 w-4 text-violet-400" />
            Multi-Tenant SaaS
          </span>
          <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
            Tenant-scoped event streams with role-based access
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            Event sourcing for SaaS platforms. Tenant isolation at the storage engine level. RBAC
            with 4 roles and 7 permissions. Policy engine for custom authorization. Per-tenant
            quotas and billing.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
              Start 14-day trial
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link href="/docs" className={cn(buttonVariants({ variant: "outline", size: "lg" }))}>
              Read the docs
            </Link>
          </div>
        </motion.div>
      </Section>

      {/* Features */}
      <Section className="pb-16">
        <h2 className="mb-12 text-center text-3xl font-bold">
          Multi-tenancy is not an afterthought
        </h2>
        <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, i) => (
            <motion.div
              key={feature.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.08 }}
              viewport={{ once: true }}
              className="rounded-xl border p-6"
            >
              <div
                className={cn(
                  "mb-4 flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br",
                  feature.color
                )}
              >
                <feature.icon className="h-5 w-5" />
              </div>
              <h3 className="mb-2 font-semibold">{feature.title}</h3>
              <p className="text-sm text-muted-foreground">{feature.description}</p>
            </motion.div>
          ))}
        </div>
      </Section>

      {/* Code Example */}
      <Section className="pb-16">
        <h2 className="mb-4 text-center text-3xl font-bold">
          Tenant provisioning in two API calls
        </h2>
        <p className="mb-8 text-center text-muted-foreground">
          Create a tenant, generate an API key, start ingesting — all in seconds
        </p>
        <div className="mx-auto max-w-3xl">
          <div className="overflow-hidden rounded-xl border">
            <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500" />
              <div className="h-3 w-3 rounded-full bg-yellow-500" />
              <div className="h-3 w-3 rounded-full bg-green-500" />
              <span className="ml-4 font-mono text-sm text-neutral-400">tenant-setup.sh</span>
            </div>
            <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
              {`# 1. Create a new tenant with quota limits
curl -s -X POST https://api.all-source.xyz/api/v1/tenants \\
  -H "Authorization: Bearer $ADMIN_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "Acme Corp",
    "plan": "growth",
    "quotas": {
      "events_per_second": 10000,
      "storage_gb": 50,
      "queries_per_day": 100000
    }
  }'

# {"tenant": {"id": "tnt_acme_29x", "name": "Acme Corp", ...}}

# 2. Generate a scoped API key for the tenant
curl -s -X POST https://api.all-source.xyz/api/v1/tenants/tnt_acme_29x/keys \\
  -H "Authorization: Bearer $ADMIN_KEY" \\
  -d '{
    "name": "acme-production",
    "role": "developer",
    "permissions": ["events:write", "events:read", "projections:read"],
    "expires_in": "90d"
  }'

# {"key": "ask_live_acme_...", "expires_at": "2026-07-15T00:00:00Z"}

# 3. Tenant ingests events — fully isolated from all other tenants
curl -s -X POST https://api.all-source.xyz/api/v1/events \\
  -H "Authorization: Bearer ask_live_acme_..." \\
  -d '{
    "event_type": "order.placed",
    "entity_id": "order-4821",
    "data": {"amount": 149.99, "currency": "USD"}
  }'

# {"id": "evt_...", "tenant_id": "tnt_acme_29x"}
# This event is invisible to every other tenant`}
            </pre>
          </div>
        </div>
      </Section>

      {/* CTA */}
      <Section className="pb-24 text-center">
        <Shield className="mx-auto mb-4 h-12 w-12 text-violet-400" />
        <h2 className="mb-4 text-3xl font-bold">Ship multi-tenant faster</h2>
        <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
          Stop building tenant isolation, RBAC, quotas, and billing from scratch. AllSource handles
          the hard parts so you can focus on your product.
        </p>
        <div className="flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
            Start 14-day trial
            <ChevronRight className="ml-1 h-4 w-4" />
          </Link>
          <Link
            href="/compare/eventstoredb"
            className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
          >
            Compare to EventStoreDB
          </Link>
        </div>
        <div className="mt-6 flex items-center justify-center gap-6 text-sm text-muted-foreground">
          <Link href="/docs" className="underline">
            Documentation
          </Link>
          <Link href="/solutions/audit-compliance" className="underline">
            Audit & Compliance
          </Link>
          <Link href="https://github.com/all-source-os/all-source" className="underline">
            GitHub
          </Link>
        </div>
      </Section>
    </div>
  );
}
