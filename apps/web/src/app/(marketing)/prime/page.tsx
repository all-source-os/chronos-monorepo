import { Badge, BlurFade, buttonVariants, Card, CardContent, cn } from "@allsource/ui";
import { ArrowRight, Check, Minus } from "lucide-react";
import Link from "next/link";
import { FaGithub } from "react-icons/fa";

const PRIME_VERSION = "0.21.4";

const claudeDesktopConfig = `{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/memory",
        "--auto-inject"
      ]
    }
  }
}`;

const claudeCodeConfig = `claude mcp add prime allsource-prime \\
  --data-dir ~/.prime/memory \\
  --auto-inject`;

const tools = [
  {
    name: "prime_add_node",
    blurb: "Create a node — person, concept, project, decision, insight.",
  },
  {
    name: "prime_add_edge",
    blurb: "Connect two nodes with a directed relation (works_on, impacts, depends_on…).",
  },
  {
    name: "prime_embed",
    blurb:
      "Make a node findable by meaning. Pass text — Prime embeds it server-side via fastembed.",
  },
  {
    name: "prime_recall",
    blurb:
      "Hybrid recall: vector similarity + graph proximity + temporal recency. Pass text or a vector.",
  },
  {
    name: "prime_context",
    blurb: "Combined retrieval with tiered depth (L0 stats / L1 conversation / L2 full hybrid).",
  },
  {
    name: "prime_index",
    blurb:
      "Compressed knowledge index — token-efficient markdown of everything you know, by domain.",
  },
  {
    name: "prime_neighbors",
    blurb: "Walk the graph around a node. Multi-hop BFS with direction + relation filters.",
  },
  {
    name: "prime_search",
    blurb: "List all nodes of a given type. Cheap, broad.",
  },
  {
    name: "prime_shortest_path",
    blurb: "Find how two entities are connected — the chain of relationships between them.",
  },
  {
    name: "prime_similar",
    blurb: "Find the most semantically similar embeddings to a stored vector.",
  },
  {
    name: "prime_history",
    blurb: "Full audit trail for any entity — every creation, update, and deletion.",
  },
  {
    name: "prime_forget",
    blurb: "Soft-delete a node. Invisible to queries, preserved in history.",
  },
  {
    name: "prime_stats",
    blurb: "Graph overview: total nodes, edges, types, relations.",
  },
];

type Comparison = {
  feature: string;
  prime: "yes" | "no";
  folk: "yes" | "no";
  notion: "yes" | "no";
  mem0: "yes" | "no";
  note?: string;
};

const comparison: Comparison[] = [
  {
    feature: "Compressed index auto-injected at conversation start",
    prime: "yes",
    folk: "no",
    notion: "no",
    mem0: "no",
  },
  {
    feature: "Event-sourced — time-travel and full audit trail",
    prime: "yes",
    folk: "no",
    notion: "no",
    mem0: "no",
  },
  {
    feature: "Cross-domain hybrid recall (graph + vector + recency)",
    prime: "yes",
    folk: "no",
    notion: "no",
    mem0: "no",
    note: "Prime treats domains as first-class",
  },
  {
    feature: "In-process embeddings — no external API",
    prime: "yes",
    folk: "no",
    notion: "no",
    mem0: "no",
  },
  {
    feature: "MCP-native (Claude Desktop, Claude Code, Cursor)",
    prime: "yes",
    folk: "no",
    notion: "no",
    mem0: "no",
  },
  {
    feature: "Polished UI for human editors",
    prime: "no",
    folk: "yes",
    notion: "yes",
    mem0: "no",
    note: "Prime is for AI editors — that's the wedge",
  },
];

function Cell({ value }: { value: "yes" | "no" }) {
  return (
    <td className="p-3 text-center">
      {value === "yes" ? (
        <Check className="mx-auto h-4 w-4 text-emerald-500" aria-label="yes" />
      ) : (
        <Minus className="mx-auto h-4 w-4 text-muted-foreground/40" aria-label="no" />
      )}
    </td>
  );
}

export default function PrimeLandingPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 py-24 lg:px-8">
      {/* Hero */}
      <BlurFade delay={0.1} inView>
        <Badge variant="outline" className="mb-4 text-xs font-mono">
          allsource-prime v{PRIME_VERSION} · MCP
        </Badge>
        <h1 className="mb-3 text-3xl font-bold tracking-tight text-foreground sm:text-5xl">
          Memory for Claude, where your agents already work.
        </h1>
        <p className="text-lg text-muted-foreground">
          AllSource Prime is the AI-native memory layer for Claude Desktop, Claude Code, and any MCP
          client. Knowledge graph, vector recall, and compressed-index auto-injection — no CMS
          chrome to babysit, no embedding API keys to rotate. Your agents read and write it
          directly.
        </p>
        <div className="mt-8 flex items-center gap-3">
          <Link href="#install" className={cn(buttonVariants({ variant: "default" }), "gap-1")}>
            Install in 30 seconds <ArrowRight className="h-4 w-4" />
          </Link>
          <Link href="/docs/prime/mcp" className={cn(buttonVariants({ variant: "outline" }))}>
            MCP setup docs
          </Link>
        </div>
      </BlurFade>

      {/* Install in 30s */}
      <BlurFade delay={0.2} inView>
        <section id="install" className="mt-16 scroll-mt-24">
          <h2 className="mb-2 text-2xl font-semibold text-foreground">Install in 30 seconds</h2>
          <p className="mb-6 text-sm text-muted-foreground">
            One <code className="rounded bg-muted px-1.5 py-0.5 font-mono">cargo install</code> and
            one config edit. Restart your client. Done.
          </p>

          <Card>
            <CardContent className="space-y-4 pt-6">
              <div>
                <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  1. Install the binary
                </div>
                <pre className="overflow-x-auto rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono">
                  cargo install allsource-prime
                </pre>
              </div>

              <div>
                <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  2a. Claude Desktop —{" "}
                  <code className="font-mono">~/.claude/claude_desktop_config.json</code>
                </div>
                <pre className="overflow-x-auto rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono leading-relaxed">
                  {claudeDesktopConfig}
                </pre>
              </div>

              <div>
                <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  2b. Claude Code — one command
                </div>
                <pre className="overflow-x-auto rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono leading-relaxed">
                  {claudeCodeConfig}
                </pre>
              </div>

              <div className="rounded-md border border-dashed bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
                <strong className="text-foreground">Why this works.</strong> The{" "}
                <code className="font-mono">--auto-inject</code> flag exposes a{" "}
                <code className="font-mono">prime://auto-context</code> MCP resource. Claude reads
                it at conversation start — a compressed markdown index of everything you&apos;ve
                told it, organized by domain, in ~500–1,000 tokens. No prompt engineering required.
              </div>
            </CardContent>
          </Card>
        </section>
      </BlurFade>

      {/* What you get */}
      <BlurFade delay={0.25} inView>
        <section className="mt-16">
          <h2 className="mb-2 text-2xl font-semibold text-foreground">
            13 MCP tools Claude picks automatically
          </h2>
          <p className="mb-6 text-sm text-muted-foreground">
            You don&apos;t call these. Claude does — the tool descriptions are written for agent
            consumption. v{PRIME_VERSION} adds text-only inputs for{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono">prime_embed</code> and{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono">prime_recall</code> so the
            model never has to hand over a precomputed vector.
          </p>
          <Card>
            <CardContent className="pt-6">
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-left text-xs uppercase tracking-wide text-muted-foreground">
                      <th className="px-3 py-2 font-medium">Tool</th>
                      <th className="px-3 py-2 font-medium">When Claude calls it</th>
                    </tr>
                  </thead>
                  <tbody>
                    {tools.map((tool) => (
                      <tr key={tool.name} className="border-b border-muted last:border-0">
                        <td className="whitespace-nowrap px-3 py-2 align-top font-mono text-xs">
                          {tool.name}
                        </td>
                        <td className="px-3 py-2 text-muted-foreground">{tool.blurb}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>
        </section>
      </BlurFade>

      {/* Comparison */}
      <BlurFade delay={0.3} inView>
        <section className="mt-16">
          <h2 className="mb-2 text-2xl font-semibold text-foreground">Why not just use a CMS?</h2>
          <p className="mb-6 text-sm text-muted-foreground">
            CMSes and CRMs are built for humans editing pages. Prime is built for AI agents reading
            and writing memory. Different shape, different wedge — pick the one your primary
            &quot;editor&quot; actually is.
          </p>
          <Card>
            <CardContent className="pt-6">
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b text-xs uppercase tracking-wide text-muted-foreground">
                      <th className="px-3 py-2 text-left font-medium">Feature</th>
                      <th className="px-3 py-2 text-center font-medium">Prime</th>
                      <th className="px-3 py-2 text-center font-medium">folk</th>
                      <th className="px-3 py-2 text-center font-medium">Notion</th>
                      <th className="px-3 py-2 text-center font-medium">Mem0</th>
                    </tr>
                  </thead>
                  <tbody>
                    {comparison.map((row) => (
                      <tr key={row.feature} className="border-b border-muted last:border-0">
                        <td className="px-3 py-2 align-top">
                          <div>{row.feature}</div>
                          {row.note ? (
                            <div className="mt-0.5 text-xs text-muted-foreground/70">
                              {row.note}
                            </div>
                          ) : null}
                        </td>
                        <Cell value={row.prime} />
                        <Cell value={row.folk} />
                        <Cell value={row.notion} />
                        <Cell value={row.mem0} />
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>
          <p className="mt-3 text-xs text-muted-foreground">
            Comparison reflects features as of {PRIME_VERSION}. folk and Notion both ship polished
            editing UIs — pick them if humans are still your primary writers.
          </p>
        </section>
      </BlurFade>

      {/* How it stores */}
      <BlurFade delay={0.35} inView>
        <section className="mt-16">
          <h2 className="mb-2 text-2xl font-semibold text-foreground">
            What&apos;s under the hood
          </h2>
          <p className="text-sm text-muted-foreground">
            Prime is an AllSource event store, not a separate database. Every mutation — node, edge,
            vector, soft-delete — appends an immutable event to the WAL and is replayed by
            projections that hold the queryable state. WAL + Parquet durability with CRC32
            checksums; Snappy-compressed columnar snapshots; in-memory DashMap reads at ~12µs.
          </p>
          <p className="mt-3 text-sm text-muted-foreground">
            Embeddings are computed in-process via{" "}
            <Link
              href="https://crates.io/crates/fastembed"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline-offset-4 hover:underline"
            >
              fastembed
            </Link>{" "}
            (AllMiniLML6V2, 384 dims, ~25 MB, auto-downloaded into the fastembed cache on first
            call). No external embedding service is required. Subsequent embeddings take ~1–3 ms on
            a modern CPU.
          </p>
          <div className="mt-6 flex flex-wrap items-center gap-3">
            <Link href="/platform/prime" className={cn(buttonVariants({ variant: "outline" }))}>
              Technical deep-dive
            </Link>
            <Link
              href="https://crates.io/crates/allsource-prime"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
            >
              crates.io/allsource-prime ↗
            </Link>
            <Link
              href="https://github.com/all-source-os/all-source/tree/main/apps/prime-mcp"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
            >
              <FaGithub className="h-3.5 w-3.5" /> Source
            </Link>
          </div>
        </section>
      </BlurFade>

      {/* CTA */}
      <BlurFade delay={0.4} inView>
        <Card className="mt-16 border-dashed">
          <CardContent className="pt-6">
            <h3 className="mb-2 font-semibold text-foreground">Already running Claude?</h3>
            <p className="mb-4 text-sm text-muted-foreground">
              Then you&apos;re halfway there. Install <code>allsource-prime</code>, add the MCP
              config block, restart your client. Claude starts writing to memory on its own — no
              prompt changes needed.
            </p>
            <Link href="#install" className={cn(buttonVariants({ variant: "default" }), "gap-1")}>
              Back to install <ArrowRight className="h-4 w-4" />
            </Link>
          </CardContent>
        </Card>
      </BlurFade>
    </div>
  );
}
