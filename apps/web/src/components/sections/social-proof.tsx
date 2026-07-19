"use client";

import { Section } from "@allsource/ui";
import { Check, X } from "lucide-react";
import { motion } from "motion/react";

const metrics = [
  {
    value: "12μs",
    label: "Agent recall latency",
    description: "vs. 70ms+ for cloud memory APIs",
    highlight: true,
  },
  {
    value: "80%+",
    label: "Cross-domain recall",
    description: "Compressed index doubles accuracy",
    highlight: false,
  },
  {
    value: "469K",
    label: "Events/sec ingestion",
    description: "Rust core, lock-free structures",
    highlight: false,
  },
  {
    value: "$0",
    label: "Self-hosted cost",
    description: "Apache-2.0 licensed, run it yourself",
    highlight: false,
  },
];

const comparison = [
  { feature: "Temporal queries (as_of)", allsource: true, mem0: false, letta: false, zep: true },
  { feature: "Full event provenance", allsource: true, mem0: false, letta: false, zep: false },
  { feature: "Auto compressed index", allsource: true, mem0: false, letta: false, zep: false },
  { feature: "Offline / embedded mode", allsource: true, mem0: false, letta: false, zep: false },
  { feature: "Sub-millisecond recall", allsource: true, mem0: false, letta: false, zep: false },
  { feature: "Apache-2.0 / self-host", allsource: true, mem0: true, letta: false, zep: false },
];

type CompetitorKey = "mem0" | "letta" | "zep";

const competitors: { key: CompetitorKey; label: string }[] = [
  { key: "mem0", label: "mem0" },
  { key: "letta", label: "Letta" },
  { key: "zep", label: "Zep" },
];

function BoolCell({ value }: { value: boolean }) {
  return value ? (
    <Check className="mx-auto h-4 w-4 text-primary" />
  ) : (
    <X className="mx-auto h-4 w-4 text-muted-foreground/40" />
  );
}

export default function SocialProof() {
  return (
    <Section
      id="why-allsource"
      title="Why AllSource"
      subtitle="Built for the benchmarks that matter"
      description="Event sourcing infrastructure with an optional AI agent memory module (Prime). The core event store stands on its own — Prime adds knowledge graphs and vector search for teams that need it."
    >
      {/* Metrics row */}
      <motion.div
        className="mx-auto mt-12 grid max-w-4xl grid-cols-2 gap-4 md:grid-cols-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        viewport={{ once: true }}
      >
        {metrics.map((metric, index) => (
          <motion.div
            key={metric.label}
            className={`flex flex-col items-center justify-center rounded-xl border p-5 text-center backdrop-blur-sm transition-all ${
              metric.highlight
                ? "border-primary/40 bg-primary/5 shadow-lg shadow-primary/10"
                : "border-border bg-background/50"
            }`}
            initial={{ opacity: 0, scale: 0.9 }}
            whileInView={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.3, delay: index * 0.1 }}
            viewport={{ once: true }}
          >
            <span className="text-3xl font-bold text-primary md:text-4xl">{metric.value}</span>
            <span className="mt-1 text-sm font-medium text-foreground">{metric.label}</span>
            <span className="mt-1 text-xs text-muted-foreground">{metric.description}</span>
          </motion.div>
        ))}
      </motion.div>

      {/* Comparison table */}
      <motion.div
        className="mx-auto mt-12 max-w-3xl overflow-hidden rounded-2xl border border-border bg-background/50"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay: 0.2 }}
        viewport={{ once: true }}
      >
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="px-4 py-3 text-left font-medium text-muted-foreground">Feature</th>
              <th className="px-4 py-3 text-center font-semibold text-primary">AllSource</th>
              {competitors.map((c) => (
                <th key={c.key} className="px-4 py-3 text-center font-medium text-muted-foreground">
                  {c.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {comparison.map((row, index) => (
              <tr
                key={row.feature}
                className={`border-b border-border/50 last:border-0 ${
                  index % 2 === 0 ? "bg-background/30" : ""
                }`}
              >
                <td className="px-4 py-3 text-foreground">{row.feature}</td>
                <td className="px-4 py-3 text-center">
                  <BoolCell value={row.allsource} />
                </td>
                {competitors.map((c) => (
                  <td key={c.key} className="px-4 py-3 text-center">
                    <BoolCell value={row[c.key]} />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </motion.div>

      <p className="mx-auto mt-4 max-w-lg text-center text-xs text-muted-foreground">
        Based on public documentation and independent benchmarks. Last updated April 2026.
      </p>
    </Section>
  );
}
