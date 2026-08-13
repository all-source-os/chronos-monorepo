"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  ArrowLeftRight,
  Banknote,
  ChevronRight,
  Clock,
  CreditCard,
  Database,
  Landmark,
  Scale,
  Users,
} from "lucide-react";
import Link from "next/link";
import { staticMotion as motion } from "@/components/ui/static-motion";

const features = [
  {
    title: "Immutable Transaction Log",
    description:
      "Every debit, credit, transfer, and fee is an append-only event. No record can be altered after the fact. The WAL with CRC32 checksums guarantees bit-level integrity.",
    icon: Database,
    color: "from-emerald-500/20 to-emerald-500/5",
  },
  {
    title: "Point-in-Time Balance Reconstruction",
    description:
      "Replay events up to any timestamp to reconstruct exact account balances. Answer 'what was the balance at 3:47 PM on March 12th?' in milliseconds, not hours.",
    icon: Clock,
    color: "from-green-500/20 to-green-500/5",
  },
  {
    title: "Regulatory Compliance (MiFID II, SOX)",
    description:
      "Full audit trail with provenance metadata satisfies MiFID II transaction reporting, SOX internal controls, and SOC2 data integrity requirements out of the box.",
    icon: Scale,
    color: "from-teal-500/20 to-teal-500/5",
  },
  {
    title: "Double-Entry Verification via Projections",
    description:
      "Define projections that enforce double-entry bookkeeping invariants. Every credit has a matching debit. Projections catch imbalances in real-time, not in end-of-day reconciliation.",
    icon: ArrowLeftRight,
    color: "from-lime-500/20 to-lime-500/5",
  },
  {
    title: "Sub-Microsecond Reconciliation",
    description:
      "Reconcile accounts across systems with published 11.9us p99 read latency. Cross-reference transaction logs from multiple sources and flag discrepancies as new events arrive.",
    icon: CreditCard,
    color: "from-cyan-500/20 to-cyan-500/5",
  },
  {
    title: "Multi-Tenant Isolation for Client Accounts",
    description:
      "Tenant IDs scope event streams, while role-based API permissions limit which client accounts a user or service can query.",
    icon: Users,
    color: "from-sky-500/20 to-sky-500/5",
  },
];

export default function FinancialServicesPage() {
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
            <Landmark className="h-4 w-4 text-emerald-400" />
            Financial Services
          </span>
          <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
            Reconstruct transaction and account history
          </h1>
          <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
            Append transaction changes as ordered events, rebuild an account balance at a past
            sequence, and trace a result back to the events that produced it.
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
            <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
              Start 14-day trial
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="/solutions/audit-compliance"
              className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
            >
              Audit & compliance details
            </Link>
          </div>
        </motion.div>
      </Section>

      {/* Features */}
      <Section className="pb-16">
        <h2 className="mb-12 text-center text-3xl font-bold">
          The transaction log your regulators expect
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
        <h2 className="mb-4 text-center text-3xl font-bold">Time-travel any account balance</h2>
        <p className="mb-8 text-center text-muted-foreground">
          Reconstruct the exact balance at any point in history
        </p>
        <div className="mx-auto max-w-3xl">
          <div className="overflow-hidden rounded-xl border">
            <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500" />
              <div className="h-3 w-3 rounded-full bg-yellow-500" />
              <div className="h-3 w-3 rounded-full bg-green-500" />
              <span className="ml-4 font-mono text-sm text-neutral-400">balance-timetravel.sh</span>
            </div>
            <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
              {`# What was account balance at market close on March 15th?
curl -s https://api.all-source.xyz/api/v1/events/query \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "entity_id": "acct-001-trading",
    "as_of": "2026-03-15T16:00:00Z",
    "event_type": "transaction.*"
  }'

# {"events": [...], "count": 2341}
# Replay all 2,341 transactions to get exact balance: $1,247,892.43

# Compare with end-of-day balance from counterparty system
curl -s https://api.all-source.xyz/api/v1/projections/account-balance \\
  -H "Authorization: Bearer $API_KEY" \\
  -G -d "entity_id=acct-001-trading" \\
     -d "as_of=2026-03-15T16:00:00Z"

# {"projection": {"balance": 1247892.43, "currency": "USD",
#   "last_tx": "2026-03-15T15:59:47Z", "tx_count": 2341}}
# Reconciliation complete — balances match to the cent`}
            </pre>
          </div>
        </div>
      </Section>

      {/* CTA */}
      <Section className="pb-24 text-center">
        <Banknote className="mx-auto mb-4 h-12 w-12 text-emerald-400" />
        <h2 className="mb-4 text-3xl font-bold">Your ledger of record</h2>
        <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
          Immutable transactions, instant time-travel, and regulatory compliance built into the
          storage engine. Not bolted on after.
        </p>
        <div className="flex flex-col items-stretch justify-center gap-3 sm:flex-row sm:items-center">
          <Link href="/signup" className={cn(buttonVariants({ size: "lg" }))}>
            Start 14-day trial
            <ChevronRight className="ml-1 h-4 w-4" />
          </Link>
          <Link href="/docs/api" className={cn(buttonVariants({ variant: "outline", size: "lg" }))}>
            API reference
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
