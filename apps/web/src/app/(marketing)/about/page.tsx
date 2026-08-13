import { buttonVariants, Card, CardContent, cn } from "@allsource/ui";
import { ArrowRight, Database, GitBranch, Zap } from "lucide-react";
import Link from "next/link";
import { FadeIn } from "@/components/ui/fade-in";
import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "About",
  description: `Why we're building ${siteConfig.name} — a purpose-built event store for the AI era.`,
});

const principles = [
  {
    title: "Events, not rows",
    icon: GitBranch,
    body: "Mutable rows lose history. AllSource records every state change as an immutable event, derives current state by projection, and lets you reconstruct any past moment with a single `as_of` query.",
  },
  {
    title: "Core IS the database",
    icon: Database,
    body: "No second store, no caching tier to keep in sync. The Rust core owns durability (WAL with CRC32 + Parquet on disk) and reads (DashMap in memory). Postgres is for billing metadata, never event data.",
  },
  {
    title: "Built for agents",
    icon: Zap,
    body: "AI agents need persistent memory beyond a context window. A tenant connector exposes 55+ MCP tools for reading and writing durable history; fleet operators can enable 73 with administrative controls.",
  },
];

export default function AboutPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <FadeIn delay={0.1} inView>
        <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">About AllSource</h1>
        <p className="text-lg text-muted-foreground">
          Open-source event storage for durable system history and AI agent memory.
        </p>
      </FadeIn>

      <FadeIn delay={0.2} inView>
        <div className="prose prose-invert mt-10 max-w-none text-muted-foreground">
          <p>
            AllSource exists because the data layer underneath modern applications — and especially
            agentic ones — has been wrong for a long time. Production systems lose history every
            time a row is updated. Audit trails are bolted on after the fact. Time-travel debugging
            requires forensic log analysis. Agents have to re-explain themselves on every
            invocation.
          </p>
          <p>
            We thought the answer was a single, focused engine: append-only events as the source of
            truth, projections as the only way to read derived state, and a query layer fast enough
            that "give me the world as it was on Tuesday" is a sub-millisecond operation rather than
            a research project. That's AllSource Core — written in Rust, durable by default, and
            built to scale through replication rather than by stitching another database underneath.
          </p>
          <p>
            Around it we ship a small constellation: a gateway in Elixir that handles auth, billing,
            and rate limiting; a graph-and-vector layer (Prime) for AI workloads; SDKs in four
            languages; and a CLI (chronis) that shows what an agent-native task tracker looks like
            when its substrate is an event store. All of it is open source, all of it ships from one
            monorepo, and the whole thing is designed to be embedded as easily as it's deployed.
          </p>
        </div>
      </FadeIn>

      <FadeIn delay={0.3} inView>
        <h2 className="text-2xl font-semibold text-foreground mt-16 mb-4">What we believe</h2>
      </FadeIn>

      <div className="grid gap-4 md:grid-cols-3 mt-2">
        {principles.map((p, i) => (
          <FadeIn key={p.title} delay={0.35 + i * 0.05} inView>
            <Card className="h-full">
              <CardContent className="pt-6">
                <p.icon className="h-6 w-6 text-foreground mb-3" />
                <h3 className="font-semibold text-foreground mb-2">{p.title}</h3>
                <p className="text-sm text-muted-foreground">{p.body}</p>
              </CardContent>
            </Card>
          </FadeIn>
        ))}
      </div>

      <FadeIn delay={0.5} inView>
        <div className="mt-16 flex flex-wrap gap-3">
          <Link
            href="https://github.com/all-source-os/all-source"
            className={cn(buttonVariants({ variant: "default" }))}
            target="_blank"
            rel="noopener noreferrer"
          >
            Source on GitHub <ArrowRight className="ml-2 h-4 w-4" />
          </Link>
          <Link href="/docs" className={cn(buttonVariants({ variant: "outline" }))}>
            Read the docs
          </Link>
          <Link href="/blog" className={cn(buttonVariants({ variant: "ghost" }))}>
            Latest writing
          </Link>
        </div>
      </FadeIn>
    </div>
  );
}
