import { Badge, BlurFade } from "@allsource/ui";
import { Suspense } from "react";
import { C4Graph } from "@/components/architecture/C4Graph";

// Fully static, public page — no auth, no tenant, no fetch. The C4 model is a
// hand-authored static file; the graph hydrates client-side (react-force-graph
// touches `window`, so it's a dynamic ssr:false import inside C4Graph).

function GraphSkeleton() {
  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
      <div className="h-[680px] animate-pulse rounded-xl border bg-muted/20" />
      <div className="hidden h-[360px] animate-pulse rounded-xl border bg-muted/20 lg:block" />
    </div>
  );
}

export default function ArchitecturePage() {
  return (
    <div className="mx-auto w-full max-w-screen-xl px-4 py-24 lg:px-8">
      <BlurFade delay={0.1} inView>
        <Badge variant="outline" className="mb-4 font-mono text-xs">
          C4 · System Context + Containers
        </Badge>
        <h1 className="mb-3 max-w-3xl text-3xl font-bold tracking-tight text-foreground sm:text-5xl">
          The AI-agent suite on a durable event store.
        </h1>
        <p className="max-w-3xl text-lg text-muted-foreground">
          AllSource Core is a purpose-built Rust event store — WAL + Parquet + DashMap, durable
          across restarts. On top of that one source of truth sits a suite of AI-agent products:
          Prime agent memory, the prime-mcp server, the event-sourced chronis task CLI, and the
          language SDKs. Explore it as an interactive C4 diagram — toggle between the System Context
          and the deployable Containers, and click any bubble for details.
        </p>
      </BlurFade>

      <BlurFade delay={0.2} inView>
        <div className="mt-10">
          <Suspense fallback={<GraphSkeleton />}>
            <C4Graph />
          </Suspense>
        </div>
      </BlurFade>

      <BlurFade delay={0.25} inView>
        <div className="mt-10 grid gap-4 sm:grid-cols-3">
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">Core is the database</div>
            <p className="text-xs text-muted-foreground">
              A durable event store: write-ahead log with CRC32 + fsync, Snappy-compressed Parquet,
              and a DashMap index for ~12µs reads. Event data survives restarts.
            </p>
          </div>
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">One gateway, one path</div>
            <p className="text-xs text-muted-foreground">
              Clients go through the Query Service (auth, billing, routing) to Core. The Control
              Plane owns public auth and billing. PostgreSQL holds operational metadata only — never
              events.
            </p>
          </div>
          <div className="rounded-lg border bg-muted/10 p-4">
            <div className="mb-1 text-sm font-semibold text-foreground">
              Backend on Fly, web on Vercel
            </div>
            <p className="text-xs text-muted-foreground">
              Six backend apps run on Fly.io (core, query, control-plane, auth, prime, registry).
              The Next.js dashboard runs on Vercel at www.all-source.xyz.
            </p>
          </div>
        </div>
      </BlurFade>
    </div>
  );
}
