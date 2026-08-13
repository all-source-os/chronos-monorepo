import { Badge } from "@allsource/ui";
import { Suspense } from "react";
import { EcosystemGraph } from "@/components/ecosystem/EcosystemGraph";
import { FadeIn } from "@/components/ui/fade-in";

// Fully static, public page — no auth, no tenant, no fetch. The ecosystem model
// is a hand-authored static file; the graph hydrates client-side (react-force-
// graph touches `window`, so it's a dynamic ssr:false import inside the graph
// component). Companion to /architecture: that = how it's built; this = what
// agents can use + how to start.

function GraphSkeleton() {
  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_380px]">
      <div className="h-[680px] animate-pulse rounded-xl border bg-muted/20" />
      <div className="hidden h-[360px] animate-pulse rounded-xl border bg-muted/20 lg:block" />
    </div>
  );
}

export default function EcosystemPage() {
  return (
    <div className="mx-auto w-full max-w-screen-xl px-4 py-24 lg:px-8">
      <FadeIn delay={0.1} inView>
        <Badge variant="outline" className="mb-4 font-mono text-xs">
          Capabilities → public apps
        </Badge>
        <h1 className="mb-3 max-w-3xl text-3xl font-bold tracking-tight text-foreground sm:text-5xl">
          AllSource tools and integrations
        </h1>
        <p className="max-w-3xl text-lg text-muted-foreground">
          Map agent capabilities to the public apps and endpoints that provide them: the{" "}
          <span className="font-medium text-foreground">prime-mcp</span> server and its{" "}
          <code className="font-mono text-base">prime_*</code> tools, the one-click Claude Desktop
          DXT, the <span className="font-medium text-foreground">chronis</span> task CLI, the SDKs,
          and the public event API. Select a capability to see its setup path.
        </p>
      </FadeIn>

      <FadeIn delay={0.2} inView>
        <div className="mt-10">
          <Suspense fallback={<GraphSkeleton />}>
            <EcosystemGraph />
          </Suspense>
        </div>
      </FadeIn>

      <FadeIn delay={0.25} inView>
        <div className="mt-10 grid gap-4 sm:grid-cols-3">
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">Memory over MCP</div>
            <p className="text-xs text-muted-foreground">
              <code className="font-mono">cargo install allsource-prime</code> gives your agent
              durable memory — <code className="font-mono">prime_recall</code>,{" "}
              <code className="font-mono">prime_add_node</code>, a knowledge graph, and vector
              search — local-first, optionally synced to your tenant.
            </p>
          </div>
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">Zero-signup start</div>
            <p className="text-xs text-muted-foreground">
              An agent can mint a working API key with a single call to{" "}
              <code className="font-mono">/api/v1/agents/anonymous-trial</code> — push events
              immediately, then claim them into a real tenant via{" "}
              <code className="font-mono">/connect</code>.
            </p>
          </div>
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">
              Honest install channels
            </div>
            <p className="text-xs text-muted-foreground">
              Rust pieces ship on crates.io (<code className="font-mono">allsource-prime</code>,{" "}
              <code className="font-mono">chronis</code>,{" "}
              <code className="font-mono">allsource</code>). The TS / Python / Go SDKs install
              straight from the GitHub registry — not npm or PyPI.
            </p>
          </div>
        </div>
      </FadeIn>
    </div>
  );
}
