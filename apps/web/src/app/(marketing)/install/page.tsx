import { ArrowRight, Sparkles, Terminal } from "lucide-react";
import Link from "next/link";
import { integrations } from "@/lib/integrations";
import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Install AllSource Prime in your tools",
  description:
    "Wire AllSource Prime — hosted agent memory over MCP — into Claude Code, Claude Desktop, Cursor, Windsurf, VS Code, ChatGPT, Codex, and OpenCode. One store, every client.",
  canonical: "/install",
});

// The hub DELEGATES to per-tool pages (mirrors Neotoma's hub-that-delegates IA).
// No per-tool config detail lives here — just the grid, plus the two shared
// onboarding entry points (hosted /connect mint, local stdio).

export default function InstallHubPage() {
  return (
    <div className="mx-auto w-full max-w-screen-lg px-4 py-24 lg:px-8">
      <h1 className="mb-3 text-3xl font-bold tracking-tight text-foreground sm:text-5xl">
        Install Prime in your tools
      </h1>
      <p className="max-w-2xl text-lg text-muted-foreground">
        {siteConfig.name} Prime is AI-native memory over the Model Context Protocol. It works with
        any MCP-compliant client — the same hosted memory across every tool your agents touch. Pick
        yours below for a copy-paste setup.
      </p>

      {/* Shared onboarding entry points — the two ways every tool connects. */}
      <div className="mt-10 grid gap-4 sm:grid-cols-2">
        <div className="rounded-xl border border-primary/30 bg-primary/5 p-5">
          <div className="mb-1 flex items-center gap-2 font-semibold text-foreground">
            <Sparkles className="h-4 w-4 text-primary" />
            Hosted memory
          </div>
          <p className="text-sm text-muted-foreground">
            Mint an API key and Prime syncs to your AllSource tenant. The same key works across every
            client — watch nodes appear live in the dashboard.
          </p>
          <Link
            href="/connect?source=install-hub"
            className="mt-3 inline-flex items-center gap-1.5 text-sm font-medium text-primary hover:underline"
          >
            Mint an API key
            <ArrowRight className="h-4 w-4" />
          </Link>
        </div>
        <div className="rounded-xl border border-border bg-muted/30 p-5">
          <div className="mb-1 flex items-center gap-2 font-semibold text-foreground">
            <Terminal className="h-4 w-4" />
            Local-only
          </div>
          <p className="text-sm text-muted-foreground">
            No account needed. Run{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
              cargo install allsource-prime
            </code>{" "}
            and keep memory on disk at{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">~/.prime/memory</code>.
          </p>
          <Link
            href="/docs/prime/mcp"
            className="mt-3 inline-flex items-center gap-1.5 text-sm font-medium text-foreground hover:underline"
          >
            MCP setup docs
            <ArrowRight className="h-4 w-4" />
          </Link>
        </div>
      </div>

      {/* Per-tool grid — each card links to its own install page. */}
      <h2 className="mb-4 mt-14 text-2xl font-semibold text-foreground">Choose your client</h2>
      <div className="grid gap-4 sm:grid-cols-2">
        {integrations.map((integration) => (
          <Link
            key={integration.slug}
            href={`/install/${integration.slug}`}
            className="group rounded-xl border border-border p-5 transition-all hover:border-primary/50 hover:bg-muted/50"
          >
            <div className="mb-1 flex items-center justify-between">
              <h3 className="font-semibold text-foreground">{integration.name}</h3>
              <ArrowRight className="h-4 w-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-foreground" />
            </div>
            <p className="text-sm leading-relaxed text-muted-foreground">{integration.blurb}</p>
          </Link>
        ))}
      </div>

      <p className="mt-10 text-sm text-muted-foreground">
        Using a client that isn&apos;t listed? Any MCP-compliant tool works — point it at the{" "}
        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">allsource-prime</code>{" "}
        stdio binary. See the{" "}
        <Link href="/docs/prime/mcp" className="underline underline-offset-4 hover:text-foreground">
          MCP setup docs
        </Link>{" "}
        for the generic config shape.
      </p>
    </div>
  );
}
