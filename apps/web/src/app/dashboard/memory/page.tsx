"use client";

import { Badge, BlurFade, buttonVariants, Card, CardContent, cn } from "@allsource/ui";
import { formatDistanceToNow } from "date-fns";
import {
  Brain,
  Compass,
  GitBranch,
  Layers,
  Network,
  RefreshCw,
  Sparkles,
  Trash2,
} from "lucide-react";
import Link from "next/link";
import { useMemo } from "react";
import { useEvents } from "@/hooks/use-events";
import type { Event } from "@/lib/api/client";

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
  // Pull the latest 200 events; we filter client-side to prime.*.
  // Once the API exposes event_type_prefix this becomes a server filter.
  const { events, isLoading, refresh } = useEvents({ limit: 200 });

  const primeEvents = useMemo(
    () => events.filter((e) => e.event_type.startsWith(PRIME_EVENT_PREFIX)),
    [events]
  );

  const stats = useMemo(() => deriveStats(primeEvents), [primeEvents]);

  const empty = !isLoading && primeEvents.length === 0;

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

      {empty ? (
        <BlurFade delay={0.15} inView>
          <Card className="border-dashed">
            <CardContent className="space-y-4 pt-6">
              <div className="flex items-start gap-3">
                <div className="rounded-md bg-purple-500/10 p-2 text-purple-400">
                  <Brain className="h-5 w-5" />
                </div>
                <div className="flex-1">
                  <h2 className="mb-1 font-semibold text-foreground">No memory yet</h2>
                  <p className="text-sm text-muted-foreground">
                    Prime stores everything as <code className="font-mono">prime.*</code> events.
                    Once your local <code className="font-mono">allsource-prime</code> binary syncs
                    to this tenant, you&apos;ll see nodes, edges, and vectors land here in real
                    time.
                  </p>
                </div>
              </div>

              <div className="rounded-md border bg-muted/20 p-4">
                <div className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Connect Prime to this tenant
                </div>
                <p className="mb-3 text-sm text-muted-foreground">
                  Install <code className="font-mono">allsource-prime</code>, then launch it with
                  the sync flags pointed at your tenant&apos;s Core URL and API key. Events show
                  up here within a second.
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
      ) : (
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
                            <span className="text-sm font-medium text-foreground">
                              {meta.label}
                            </span>
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
      )}
    </div>
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
