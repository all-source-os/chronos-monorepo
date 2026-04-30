import { BlurFade, buttonVariants, Card, CardContent, cn } from "@allsource/ui";
import { ArrowRight, Bot, ClipboardList, Code2, Layers, Radio, Terminal } from "lucide-react";
import Link from "next/link";
import { FaGithub } from "react-icons/fa";
import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Examples",
  description: `Working examples and reference projects built on ${siteConfig.name}.`,
});

const codeExamples = [
  {
    title: "Asset projection (Rust)",
    icon: Code2,
    href: "https://github.com/all-source-os/all-source/blob/main/sdks/rust/examples/asset_projection.rs",
    blurb:
      "End-to-end Rust example: ingest events, fold them into a projection, query the projected state. Used as the canonical sample in the Rust SDK and exercised by the Docker build.",
    badge: "sdks/rust",
  },
  {
    title: "MCP server quickstart",
    icon: Terminal,
    href: "/docs/mcp",
    blurb:
      "Wire a Claude Desktop or any MCP-compatible client to AllSource in under five minutes. Config snippet, the 43 available tools, and what each one returns.",
    badge: "docs",
  },
  {
    title: "Tenant onboarding flow",
    icon: ClipboardList,
    href: "/docs/tenant-setup",
    blurb:
      "Provision a tenant, mint an API key, and write your first event using `curl`. The exact sequence the onboarding endpoint runs server-side, but spelled out so you can integrate it into your own setup script.",
    badge: "docs",
  },
  {
    title: "Custom projections",
    icon: Layers,
    href: "https://github.com/all-source-os/all-source/blob/main/docs/use-cases/custom-projections.md",
    blurb:
      "Walks through writing a projection that derives running totals from a stream of order events — including incremental updates, replay, and snapshots. Vendor-neutral, applicable to any SDK.",
    badge: "docs",
  },
];

const referenceProjects = [
  {
    title: "chronis — agent-native task CLI",
    icon: Terminal,
    href: "https://github.com/all-source-os/all-source/tree/main/apps/chronis",
    blurb:
      "A working CLI built directly on top of AllSource. Every action is an immutable event, state is a projection, the binary syncs to the hosted gateway. The cleanest example of what an agent-friendly tool looks like when its substrate is an event store.",
  },
  {
    title: "Prime — graph + vector + recall",
    icon: Bot,
    href: "https://github.com/all-source-os/all-source/tree/main/apps/prime-mcp",
    blurb:
      "Prime is the AI-workload service: a graph store, a vector index, and a temporal recall layer, all served as MCP tools and over HTTP. Source for both the engine and the MCP wrapper that LLM agents talk to.",
  },
  {
    title: "Phoenix Channels streaming",
    icon: Radio,
    href: "https://github.com/all-source-os/all-source/tree/main/apps/query-service",
    blurb:
      "The Elixir gateway's WebSocket layer. Real-time projection subscriptions, per-tenant channel scoping, and reconnection semantics — useful as a reference if you're wiring AllSource into a live UI.",
  },
];

const useCases = [
  { title: "Audit & compliance", href: "/solutions/audit-compliance" },
  { title: "Agent memory", href: "/solutions/agent-memory" },
  { title: "Financial services", href: "/solutions/financial-services" },
  { title: "IoT telemetry", href: "/solutions/iot-telemetry" },
  { title: "Multi-tenant SaaS", href: "/solutions/multi-tenant-saas" },
  { title: "Quant intelligence", href: "/solutions/quant-intelligence" },
  { title: "Real-time analytics", href: "/solutions/real-time-analytics" },
];

export default function ExamplesPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <BlurFade delay={0.1} inView>
        <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">Examples</h1>
        <p className="text-lg text-muted-foreground">
          Working code, reference projects, and walkthroughs for the most common AllSource shapes.
        </p>
      </BlurFade>

      <BlurFade delay={0.2} inView>
        <h2 className="text-2xl font-semibold text-foreground mt-12 mb-4">Code snippets</h2>
      </BlurFade>

      <div className="space-y-3">
        {codeExamples.map((ex, i) => {
          const isExternal = ex.href.startsWith("http");
          return (
            <BlurFade key={ex.title} delay={0.25 + i * 0.05} inView>
              <Link
                href={ex.href}
                {...(isExternal ? { target: "_blank", rel: "noopener noreferrer" } : {})}
                className="block group"
              >
                <Card className="transition-colors hover:border-foreground/20">
                  <CardContent className="pt-6">
                    <div className="flex items-start gap-4">
                      <ex.icon className="h-5 w-5 text-foreground mt-0.5 shrink-0" />
                      <div className="flex-1">
                        <div className="flex items-baseline justify-between gap-2 mb-1">
                          <h3 className="font-semibold text-foreground">{ex.title}</h3>
                          <span className="text-xs text-muted-foreground font-mono">
                            {ex.badge}
                          </span>
                        </div>
                        <p className="text-sm text-muted-foreground">{ex.blurb}</p>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              </Link>
            </BlurFade>
          );
        })}
      </div>

      <BlurFade delay={0.45} inView>
        <h2 className="text-2xl font-semibold text-foreground mt-16 mb-4">Reference projects</h2>
        <p className="text-sm text-muted-foreground mb-4">
          Real services in the monorepo that consume AllSource as their data layer. Read the source
          to see how the pieces fit together end-to-end.
        </p>
      </BlurFade>

      <div className="space-y-3">
        {referenceProjects.map((p, i) => (
          <BlurFade key={p.title} delay={0.5 + i * 0.05} inView>
            <Link href={p.href} target="_blank" rel="noopener noreferrer" className="block group">
              <Card className="transition-colors hover:border-foreground/20">
                <CardContent className="pt-6">
                  <div className="flex items-start gap-4">
                    <p.icon className="h-5 w-5 text-foreground mt-0.5 shrink-0" />
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <h3 className="font-semibold text-foreground">{p.title}</h3>
                        <FaGithub className="h-3.5 w-3.5 text-muted-foreground" />
                      </div>
                      <p className="text-sm text-muted-foreground">{p.blurb}</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </Link>
          </BlurFade>
        ))}
      </div>

      <BlurFade delay={0.7} inView>
        <h2 className="text-2xl font-semibold text-foreground mt-16 mb-4">Solution walkthroughs</h2>
        <p className="text-sm text-muted-foreground mb-4">
          Long-form pages on how AllSource maps to specific problem shapes — closer to a tutorial
          than a snippet.
        </p>
      </BlurFade>

      <BlurFade delay={0.75} inView>
        <div className="grid sm:grid-cols-2 gap-2">
          {useCases.map((uc) => (
            <Link
              key={uc.href}
              href={uc.href}
              className="group inline-flex items-center justify-between rounded-md border px-4 py-3 text-sm text-muted-foreground hover:text-foreground hover:border-foreground/20 transition-colors"
            >
              <span>{uc.title}</span>
              <ArrowRight className="h-4 w-4 opacity-0 -translate-x-1 transition-all group-hover:opacity-100 group-hover:translate-x-0" />
            </Link>
          ))}
        </div>
      </BlurFade>

      <BlurFade delay={0.85} inView>
        <div className="mt-12 flex flex-wrap gap-3">
          <Link href="/sdks" className={cn(buttonVariants({ variant: "default" }))}>
            Pick an SDK <ArrowRight className="ml-2 h-4 w-4" />
          </Link>
          <Link href="/docs" className={cn(buttonVariants({ variant: "outline" }))}>
            Read the docs
          </Link>
        </div>
      </BlurFade>
    </div>
  );
}
