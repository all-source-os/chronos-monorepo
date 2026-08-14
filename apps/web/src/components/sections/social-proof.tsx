import { buttonVariants, cn, Section } from "@allsource/ui";
import { ArrowRight } from "lucide-react";
import Link from "next/link";
import { siteConfig } from "@/lib/config";

const metrics = [
  {
    value: siteConfig.stats[1]?.display ?? "11.9μs",
    label: "Core indexed-read p99",
    description: "Published in-memory reference benchmark",
  },
  {
    value: siteConfig.stats[0]?.display ?? "469K",
    label: "events/sec",
    description: "Published batch-ingestion benchmark",
  },
  {
    value: "WAL + Parquet",
    label: "durable storage",
    description: "Write-ahead log with columnar persistence",
  },
  {
    value: "Apache-2.0",
    label: "core licence",
    description: "Inspect, run, and modify the event-store core",
  },
];

export default function TechnicalProof() {
  return (
    <Section
      id="technical-proof"
      title="Claims you can inspect"
      subtitle="Published proof"
      description="Numbers link back to reproducible commands and source code. Architecture pages explain which service owns each part of the data path."
    >
      <div className="mx-auto grid max-w-5xl gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {metrics.map((metric) => (
          <div key={metric.label} className="border border-border bg-card p-5">
            <p className="text-2xl font-semibold text-primary">{metric.value}</p>
            <p className="mt-2 font-medium text-foreground">{metric.label}</p>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">{metric.description}</p>
          </div>
        ))}
      </div>

      <div className="mt-8 flex flex-col justify-center gap-3 sm:flex-row">
        <Link href="/architecture" className={cn(buttonVariants({ variant: "outline" }), "gap-2")}>
          Read architecture
          <ArrowRight className="h-4 w-4" aria-hidden="true" />
        </Link>
        <Link
          href={`${siteConfig.links.github}#benchmarks`}
          className={cn(buttonVariants({ variant: "outline" }), "gap-2")}
        >
          Reproduce benchmarks
          <ArrowRight className="h-4 w-4" aria-hidden="true" />
        </Link>
      </div>
    </Section>
  );
}
