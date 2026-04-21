"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  ChevronRight,
  Clock,
  Database,
  FileCheck,
  Fingerprint,
  Lock,
  ScrollText,
  Shield,
  Terminal,
  Users,
} from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

const features = [
  {
    title: "Immutable Append-Only Log",
    description:
      "Every event is permanently recorded. No updates, no deletes, no tampering. The WAL ensures every write survives crashes with CRC32 checksums and configurable fsync.",
    icon: Database,
    color: "from-blue-500/20 to-blue-500/5",
  },
  {
    title: "CRC32 Cryptographic Checksums",
    description:
      "Every WAL entry is checksummed at write time. Detect any bit-level corruption or tampering during recovery. Auditors can verify data integrity independently.",
    icon: Fingerprint,
    color: "from-indigo-500/20 to-indigo-500/5",
  },
  {
    title: "Time-Travel Reconstruction",
    description:
      "Reconstruct the exact state of any entity at any past timestamp with as_of queries. Answer regulator questions in seconds, not weeks of manual log trawling.",
    icon: Clock,
    color: "from-cyan-500/20 to-cyan-500/5",
  },
  {
    title: "RBAC: 4 Roles, 7 Permissions",
    description:
      "Admin, Developer, ReadOnly, and ServiceAccount roles with fine-grained permissions. Control who can ingest, query, manage schemas, and access projections.",
    icon: Users,
    color: "from-green-500/20 to-green-500/5",
  },
  {
    title: "Policy Enforcement Engine",
    description:
      "Define custom authorization policies beyond RBAC. Enforce data retention rules, access windows, IP restrictions, and tenant-specific compliance requirements.",
    icon: Shield,
    color: "from-purple-500/20 to-purple-500/5",
  },
  {
    title: "Full Event Provenance",
    description:
      "Every event carries metadata: who created it, when, from which service, with what API key. Complete chain of custody from ingestion to query.",
    icon: ScrollText,
    color: "from-amber-500/20 to-amber-500/5",
  },
];

export default function AuditCompliancePage() {
  return (
    <>
      <Header />
      <main className="relative overflow-hidden">
        {/* Hero */}
        <Section className="relative pt-24 pb-16 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <span className="inline-flex items-center gap-2 rounded-full border bg-background/50 px-4 py-1.5 text-sm backdrop-blur-sm">
              <FileCheck className="h-4 w-4 text-blue-400" />
              Audit & Compliance
            </span>
            <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
              Audit trails that
              <br />
              <span className="bg-gradient-to-r from-blue-400 to-indigo-400 bg-clip-text text-transparent">
                regulators actually trust
              </span>
            </h1>
            <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
              Immutable event history with cryptographic integrity. Reconstruct
              any past state in seconds, not days. SOC2-ready event sourcing with
              RBAC, policy enforcement, and full provenance.
            </p>
            <div className="mt-8 flex items-center justify-center gap-4">
              <Link
                href="/signup"
                className={cn(buttonVariants({ size: "lg" }))}
              >
                Start free
                <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link
                href="/docs"
                className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
              >
                Read the docs
              </Link>
            </div>
          </motion.div>
        </Section>

        {/* Features */}
        <Section className="pb-16">
          <h2 className="mb-12 text-center text-3xl font-bold">
            Built for compliance from day one
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
                    feature.color,
                  )}
                >
                  <feature.icon className="h-5 w-5" />
                </div>
                <h3 className="mb-2 font-semibold">{feature.title}</h3>
                <p className="text-sm text-muted-foreground">
                  {feature.description}
                </p>
              </motion.div>
            ))}
          </div>
        </Section>

        {/* Code Example */}
        <Section className="pb-16">
          <h2 className="mb-4 text-center text-3xl font-bold">
            Reconstruct any past state
          </h2>
          <p className="mb-8 text-center text-muted-foreground">
            One API call to answer &quot;what was the state at time X?&quot;
          </p>
          <div className="mx-auto max-w-3xl">
            <div className="overflow-hidden rounded-xl border">
              <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
                <div className="h-3 w-3 rounded-full bg-red-500" />
                <div className="h-3 w-3 rounded-full bg-yellow-500" />
                <div className="h-3 w-3 rounded-full bg-green-500" />
                <span className="ml-4 font-mono text-sm text-neutral-400">
                  compliance-audit.sh
                </span>
              </div>
              <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
{`# Reconstruct account state at the time of the audit
curl -s https://api.all-source.xyz/api/v1/events/query \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "entity_id": "account-7291",
    "as_of": "2026-01-15T09:30:00Z",
    "event_type": "compliance.*"
  }' | jq '.events | length'

# Response: 1,247 events — full history up to audit timestamp
# Every event has: who, when, what, from which service, CRC32 checksum

# Verify data integrity across the entire event log
curl -s https://api.all-source.xyz/api/v1/events/query \\
  -H "Authorization: Bearer $API_KEY" \\
  -d '{
    "entity_id": "account-7291",
    "include_checksums": true
  }' | jq '.events[] | .checksum' | wc -l

# 1,247 checksums — every single event is independently verifiable`}
              </pre>
            </div>
          </div>
        </Section>

        {/* CTA */}
        <Section className="pb-24 text-center">
          <Lock className="mx-auto mb-4 h-12 w-12 text-blue-400" />
          <h2 className="mb-4 text-3xl font-bold">
            Stop dreading audit season
          </h2>
          <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
            With immutable events, cryptographic checksums, and instant time-travel,
            your next audit takes hours instead of weeks.
          </p>
          <div className="flex items-center justify-center gap-4">
            <Link
              href="/signup"
              className={cn(buttonVariants({ size: "lg" }))}
            >
              Start free
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
            <Link href="/docs/api" className="underline">
              API Reference
            </Link>
            <Link
              href="https://github.com/all-source-os/all-source"
              className="underline"
            >
              GitHub
            </Link>
          </div>
        </Section>
      </main>
      <Footer />
    </>
  );
}
