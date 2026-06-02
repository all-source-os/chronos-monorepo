"use client";

import { Badge, BlurFade, buttonVariants, Card, CardContent, cn, Skeleton } from "@allsource/ui";
import { formatDistanceToNow } from "date-fns";
import {
  Brain,
  Compass,
  GitBranch,
  Layers,
  List as ListIcon,
  Network,
  RefreshCw,
  Share2,
  Sparkles,
  Trash2,
} from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { type ReactNode, Suspense, useMemo } from "react";
import { PrimeGraph } from "@/components/memory/PrimeGraph";
import { PrimeGraphList } from "@/components/memory/PrimeGraphList";
import { useEvents } from "@/hooks/use-events";
import { useGraph } from "@/hooks/use-graph";
import type { Event } from "@/lib/api/client";

// Cap the bubble graph's server-side fetch so a huge tenant graph stays
// responsive in the browser. The list/overview views are unaffected; we surface
// a "showing X of Y" notice and a refine-by-type hint when the cap bites.
const GRAPH_LIMIT = 1500;

type MemoryView = "overview" | "graph" | "list";

function isView(v: string | null): v is MemoryView {
  return v === "overview" || v === "graph" || v === "list";
}

// Prime event types — emitted by allsource-prime to its EmbeddedCore.
// Tenants see these in their AllSource Core feed once they enable sync.
const PRIME_EVENT_PREFIX = "prime.";

const EVENT_META: Record<string, { label: string; icon: typeof Brain; color: string }> = {
  "prime.node.created": { label: "Node created", icon: Sparkles, color: "text-emerald-500" },
  "prime.node.updated": { label: "Node updated", icon: Network, color: "text-blue-500" },
  "prime.node.deleted": { label: "Node forgotten", icon: Trash2, color: "text-muted-foreground" },
  "prime.edge.created": { label: "Edge created", icon: GitBranch, color: "text-purple-500" },
  "prime.edge.deleted": { label: "Edge deleted", icon: Trash2, color: "text-muted-foreground" },
  "prime.vector.stored": { label: "Vector stored", icon: Compass, color: "text-cyan-500" },
  "prime.vector.deleted": { label: "Vector deleted", icon: Trash2, color: "text-muted-foreground" },
};

function eventMeta(type: string) {
  return (
    EVENT_META[type] ?? {
      label: type.replace(/^prime\./, ""),
      icon: Layers,
      color: "text-muted-foreground",
    }
  );
}

function nodeTypeFromPayload(payload: Record<string, unknown>): string {
  const t = payload["node_type"];
  return typeof t === "string" ? t : "unknown";
}

function deriveStats(events: Event[]) {
  let nodesCreated = 0;
  let nodesDeleted = 0;
  let edgesCreated = 0;
  let edgesDeleted = 0;
  let vectorsStored = 0;
  let vectorsDeleted = 0;
  const nodesByType = new Map<string, number>();

  for (const ev of events) {
    switch (ev.event_type) {
      case "prime.node.created":
        nodesCreated += 1;
        nodesByType.set(
          nodeTypeFromPayload(ev.payload),
          (nodesByType.get(nodeTypeFromPayload(ev.payload)) ?? 0) + 1
        );
        break;
      case "prime.node.deleted":
        nodesDeleted += 1;
        break;
      case "prime.edge.created":
        edgesCreated += 1;
        break;
      case "prime.edge.deleted":
        edgesDeleted += 1;
        break;
      case "prime.vector.stored":
        vectorsStored += 1;
        break;
      case "prime.vector.deleted":
        vectorsDeleted += 1;
        break;
    }
  }

  const liveNodes = Math.max(0, nodesCreated - nodesDeleted);
  const liveEdges = Math.max(0, edgesCreated - edgesDeleted);
  const liveVectors = Math.max(0, vectorsStored - vectorsDeleted);
  const eventCount = events.length;

  // Sort node types by count, descending
  const sortedNodeTypes = Array.from(nodesByType.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, 6);

  return {
    liveNodes,
    liveEdges,
    liveVectors,
    eventCount,
    sortedNodeTypes,
  };
}

export default function MemoryPage() {
  return (
    <Suspense fallback={<MemoryPageSkeleton />}>
      <MemoryPageInner />
    </Suspense>
  );
}

function MemoryPageInner() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const viewParam = searchParams.get("view");
  const view: MemoryView = isView(viewParam) ? viewParam : "overview";

  const setView = (next: MemoryView) => {
    const params = new URLSearchParams(searchParams.toString());
    if (next === "overview") params.delete("view");
    else params.set("view", next);
    const qs = params.toString();
    router.replace(qs ? `/dashboard/memory?${qs}` : "/dashboard/memory", { scroll: false });
  };

  // Server-side prefix filter so prime.* events can't be drowned out by other
  // event types under a flat limit. The Query Service forwards
  // event_type_prefix to Core, which filters before the limit is applied — so
  // a high-traffic tenant with thousands of non-prime events still sees its
  // memory. We keep the defensive client-side filter below in case the
  // deployed gateway predates event_type_prefix support (graceful fallback).
  const {
    events,
    isLoading: eventsLoading,
    refresh: refreshEvents,
  } = useEvents({
    event_type_prefix: PRIME_EVENT_PREFIX,
    limit: 500,
  });

  // Graph/List read the materialized graph straight from /api/v1/prime/graph
  // (full node+edge set with stored properties) rather than re-deriving from
  // the event stream. Fetched once; filtered/searched client-side.
  const {
    nodes,
    edges,
    stats: graphStats,
    hasMore,
    isLoading: graphLoading,
    error: graphError,
    refresh: refreshGraph,
  } = useGraph(view === "overview" ? undefined : { limit: GRAPH_LIMIT });

  const primeEvents = useMemo(
    () => events.filter((e) => e.event_type.startsWith(PRIME_EVENT_PREFIX)),
    [events]
  );

  const stats = useMemo(() => deriveStats(primeEvents), [primeEvents]);

  const isLoading = view === "overview" ? eventsLoading : graphLoading;
  const refresh = view === "overview" ? refreshEvents : refreshGraph;

  // Overview empties on no prime events; graph/list empty on no nodes. Both fall
  // back to the same honest "sync isn't configured" panel below.
  const empty =
    view === "overview"
      ? !eventsLoading && primeEvents.length === 0
      : !graphLoading && !graphError && nodes.length === 0;

  return (
    <div className="mx-auto w-full max-w-screen-xl px-4 py-8 lg:px-8">
      <BlurFade delay={0.1} inView>
        <div className="mb-8 flex items-start justify-between gap-4">
          <div>
            <div className="mb-2 flex items-center gap-2">
              <Brain className="h-5 w-5 text-purple-400" />
              <h1 className="text-2xl font-bold tracking-tight text-foreground">Memory</h1>
              <Badge variant="outline" className="font-mono text-[10px]">
                allsource-prime
              </Badge>
            </div>
            <p className="max-w-2xl text-sm text-muted-foreground">
              Live view of the knowledge graph your agents are writing through
              <code className="mx-1 rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                allsource-prime
              </code>
              . Derived from <code className="font-mono">prime.*</code> events in your AllSource
              Core — same audit trail, same WAL, same Parquet durability.
            </p>
          </div>
          <button
            type="button"
            onClick={() => refresh()}
            className={cn(buttonVariants({ variant: "outline", size: "sm" }), "gap-1.5")}
          >
            <RefreshCw className={cn("h-3.5 w-3.5", isLoading && "animate-spin")} />
            Refresh
          </button>
        </div>
      </BlurFade>

      {/* View toggle — persisted in ?view= so it's shareable/reloadable. */}
      <BlurFade delay={0.12} inView>
        <div className="mb-6 inline-flex rounded-md border p-0.5">
          <ViewTab active={view === "overview"} onClick={() => setView("overview")} icon={Layers}>
            Overview
          </ViewTab>
          <ViewTab active={view === "graph"} onClick={() => setView("graph")} icon={Share2}>
            Graph
          </ViewTab>
          <ViewTab active={view === "list"} onClick={() => setView("list")} icon={ListIcon}>
            List
          </ViewTab>
        </div>
      </BlurFade>

      {graphError && view !== "overview" && (
        <Card className="mb-6 border-destructive/40">
          <CardContent className="pt-6 text-sm text-destructive">
            Couldn&apos;t load the graph: {graphError}
          </CardContent>
        </Card>
      )}

      {graphLoading && view !== "overview" && <GraphLoadingSkeleton />}

      {!graphLoading && !graphError && view === "graph" && nodes.length > 0 && (
        <BlurFade delay={0.15} inView>
          {hasMore && (
            <p className="mb-3 text-xs text-muted-foreground">
              Showing {nodes.length.toLocaleString()} of {graphStats.node_count.toLocaleString()}{" "}
              nodes — refine by type (legend chips) to focus the view.
            </p>
          )}
          <PrimeGraph nodes={nodes} edges={edges} nodesByType={graphStats.nodes_by_type} />
        </BlurFade>
      )}

      {!graphLoading && !graphError && view === "list" && nodes.length > 0 && (
        <BlurFade delay={0.15} inView>
          {hasMore && (
            <p className="mb-3 text-xs text-muted-foreground">
              Showing {nodes.length.toLocaleString()} of {graphStats.node_count.toLocaleString()}{" "}
              nodes — filter by type to narrow.
            </p>
          )}
          <PrimeGraphList nodes={nodes} edges={edges} nodesByType={graphStats.nodes_by_type} />
        </BlurFade>
      )}

      {view === "overview" &&
        (empty ? <EmptyMemoryState /> : <OverviewBody stats={stats} primeEvents={primeEvents} />)}

      {/* Graph/List share the overview's honest empty-state. */}
      {view !== "overview" && empty && <EmptyMemoryState />}
    </div>
  );
}

function EmptyMemoryState() {
  return (
    <BlurFade delay={0.15} inView>
      <Card className="border-dashed">
        <CardContent className="space-y-4 pt-6">
          <div className="flex items-start gap-3">
            <div className="rounded-md bg-purple-500/10 p-2 text-purple-400">
              <Brain className="h-5 w-5" />
            </div>
            <div className="flex-1">
              <h2 className="mb-1 font-semibold text-foreground">
                No memory is reaching this tenant yet
              </h2>
              <p className="text-sm text-muted-foreground">
                Prime stores everything as <code className="font-mono">prime.*</code> events. An
                empty panel almost always means{" "}
                <strong className="text-foreground">sync isn&apos;t configured</strong>, not that
                you have no memory: if <code className="font-mono">allsource-prime</code> is running
                without <code className="font-mono">--sync-to</code> and{" "}
                <code className="font-mono">--api-key</code>, your writes succeed <em>locally</em>{" "}
                but never ship here. Run <code className="font-mono">prime_stats</code> in your
                agent — its <code className="font-mono">sync</code> field tells you whether sync is
                enabled. Once it is, existing nodes, edges, and vectors backfill automatically.
              </p>
            </div>
          </div>

          <div className="rounded-md border bg-muted/20 p-4">
            <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Connect Prime to this tenant
            </div>
            <p className="mb-3 text-sm text-muted-foreground">
              Install <code className="font-mono">allsource-prime</code>, then launch it with the
              sync flags pointed at your tenant&apos;s Core URL and API key. Events show up here
              within a second.
            </p>
            <pre className="overflow-x-auto rounded border bg-background/60 p-3 text-xs font-mono leading-relaxed">
              {`# 1. install (0.21.4+)
cargo install allsource-prime

# 2. launch with sync to this tenant
allsource-prime \\
  --data-dir ~/.prime/memory \\
  --sync-to https://api.all-source.xyz \\
  --api-key <your tenant API key>`}
            </pre>
            <div className="mt-3 flex flex-wrap gap-2">
              <Link
                href="/prime"
                className={cn(buttonVariants({ variant: "default", size: "sm" }))}
              >
                Install guide
              </Link>
              <Link
                href="/dashboard/api-keys"
                className={cn(buttonVariants({ variant: "outline", size: "sm" }))}
              >
                Get an API key
              </Link>
              <Link
                href="/docs/prime/mcp"
                className={cn(buttonVariants({ variant: "outline", size: "sm" }))}
              >
                MCP setup docs
              </Link>
            </div>
          </div>
        </CardContent>
      </Card>
    </BlurFade>
  );
}

function OverviewBody({
  stats,
  primeEvents,
}: {
  stats: ReturnType<typeof deriveStats>;
  primeEvents: Event[];
}) {
  return (
    <>
      {/* Stats cards */}
      <BlurFade delay={0.15} inView>
        <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <StatCard
            icon={Sparkles}
            color="text-emerald-500"
            value={stats.liveNodes}
            label="Live nodes"
          />
          <StatCard
            icon={GitBranch}
            color="text-purple-500"
            value={stats.liveEdges}
            label="Live edges"
          />
          <StatCard
            icon={Compass}
            color="text-cyan-500"
            value={stats.liveVectors}
            label="Live vectors"
          />
          <StatCard
            icon={Layers}
            color="text-blue-500"
            value={stats.eventCount}
            label="Prime events"
          />
        </div>
      </BlurFade>

      {/* Nodes by type */}
      {stats.sortedNodeTypes.length > 0 && (
        <BlurFade delay={0.2} inView>
          <Card className="mb-6">
            <CardContent className="pt-6">
              <h3 className="mb-3 text-sm font-medium text-foreground">Nodes by type</h3>
              <div className="flex flex-wrap gap-2">
                {stats.sortedNodeTypes.map(([type, count]) => (
                  <Badge key={type} variant="outline" className="font-mono text-xs">
                    {type}
                    <span className="ml-1.5 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                      {count}
                    </span>
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        </BlurFade>
      )}

      {/* Recent events */}
      <BlurFade delay={0.25} inView>
        <Card>
          <CardContent className="pt-6">
            <h3 className="mb-3 text-sm font-medium text-foreground">Recent activity</h3>
            <div className="space-y-2">
              {primeEvents.slice(0, 50).map((ev) => {
                const meta = eventMeta(ev.event_type);
                const Icon = meta.icon;
                return (
                  <div
                    key={ev.id}
                    className="flex items-start gap-3 rounded-md border bg-muted/10 px-3 py-2"
                  >
                    <Icon className={cn("mt-0.5 h-3.5 w-3.5 shrink-0", meta.color)} />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-baseline gap-2">
                        <span className="text-sm font-medium text-foreground">{meta.label}</span>
                        <code className="truncate font-mono text-xs text-muted-foreground">
                          {ev.entity_id}
                        </code>
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {formatDistanceToNow(new Date(ev.timestamp), { addSuffix: true })}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      </BlurFade>
    </>
  );
}

function ViewTab({
  active,
  onClick,
  icon: Icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon: typeof Brain;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1.5 rounded px-3 py-1.5 text-sm font-medium transition-colors",
        active ? "bg-muted text-foreground" : "text-muted-foreground hover:text-foreground"
      )}
    >
      <Icon className="h-3.5 w-3.5" />
      {children}
    </button>
  );
}

function MemoryPageSkeleton() {
  return (
    <div className="mx-auto w-full max-w-screen-xl px-4 py-8 lg:px-8">
      <Skeleton className="mb-8 h-8 w-40" />
      <Skeleton className="mb-6 h-9 w-72" />
      <Skeleton className="h-[560px] w-full" />
    </div>
  );
}

function GraphLoadingSkeleton() {
  return (
    <BlurFade delay={0.12} inView>
      <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
        <Skeleton className="h-[560px] w-full rounded-lg" />
        <Skeleton className="h-[280px] w-full rounded-lg" />
      </div>
    </BlurFade>
  );
}

function StatCard({
  icon: Icon,
  color,
  value,
  label,
}: {
  icon: typeof Brain;
  color: string;
  value: number;
  label: string;
}) {
  return (
    <Card>
      <CardContent className="pt-6">
        <Icon className={cn("mb-2 h-4 w-4", color)} />
        <div className="text-2xl font-bold tabular-nums text-foreground">
          {value.toLocaleString()}
        </div>
        <div className="text-xs text-muted-foreground">{label}</div>
      </CardContent>
    </Card>
  );
}
