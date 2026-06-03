"use client";

import { Badge, Card, CardContent, cn } from "@allsource/ui";
import { Check, Copy, ExternalLink, Filter, Sparkles } from "lucide-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  APP_KINDS,
  type AppKind,
  type CodeSnippet,
  colorForNode,
  ECOSYSTEM_EDGES,
  ECOSYSTEM_NODES,
  type EcosystemNode,
  KIND_LABELS,
  NODE_COLORS,
} from "@/data/ecosystem-model";

// react-force-graph-2d paints to a <canvas> and touches `window` at module
// scope, so it MUST be client-only — a static import crashes the Next build
// during SSR. Same dynamic ssr:false pattern the C4Graph + Memory graph use.
const ForceGraph2D = dynamic(() => import("react-force-graph-2d"), {
  ssr: false,
  loading: () => (
    <div className="flex h-[600px] items-center justify-center text-sm text-muted-foreground">
      Loading ecosystem…
    </div>
  ),
});

// react-force-graph mutates link.source/target into node refs at runtime.
interface ForceNode extends EcosystemNode {
  degree: number;
  color: string;
  x?: number;
  y?: number;
}
interface ForceLink {
  source: string | ForceNode;
  target: string | ForceNode;
  label: string;
}

export function EcosystemGraph() {
  const [selected, setSelected] = useState<ForceNode | null>(null);
  // null = show all kinds; otherwise restrict app nodes to this kind.
  const [kindFilter, setKindFilter] = useState<AppKind | null>(null);

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const fgRef = useRef<{
    zoomToFit?: (ms: number, px: number) => void;
    d3Force?: (
      name: string
    ) => { distance?: (d: number) => unknown; strength?: (s: number) => unknown } | undefined;
  } | null>(null);
  const [size, setSize] = useState({ w: 800, h: 600 });

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (rect?.width && rect.height) {
        setSize({ w: Math.round(rect.width), h: Math.round(rect.height) });
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Give the bubbles room. The ForceGraph is a dynamic (ssr:false) import, so
  // its ref may be null on the first effect pass — poll briefly until d3Force
  // is available, then set forces once before the single cooldown settles.
  useEffect(() => {
    let tries = 0;
    const apply = () => {
      const fg = fgRef.current;
      if (!fg?.d3Force) {
        if (tries++ < 40) setTimeout(apply, 50);
        return;
      }
      fg.d3Force("charge")?.strength?.(-320);
      fg.d3Force("link")?.distance?.(80);
    };
    apply();
  }, []);

  // When a kind filter is active, hide app nodes of other kinds (and any
  // capability that ends up with no visible app). Capabilities + their edges to
  // visible apps stay. Distribution-registry nodes (package) always feel core,
  // but we respect the filter uniformly for predictability.
  const { nodes: modelNodes, edges: modelEdges } = useMemo(() => {
    if (!kindFilter) return { nodes: ECOSYSTEM_NODES, edges: ECOSYSTEM_EDGES };
    const keep = new Set<string>();
    for (const n of ECOSYSTEM_NODES) {
      if (n.type === "app" && n.kind === kindFilter) keep.add(n.id);
    }
    // Keep capabilities that still connect to a visible app.
    const visibleEdges = ECOSYSTEM_EDGES.filter((e) => keep.has(e.target));
    for (const e of visibleEdges) keep.add(e.source);
    return {
      nodes: ECOSYSTEM_NODES.filter((n) => keep.has(n.id)),
      edges: visibleEdges,
    };
  }, [kindFilter]);

  const degree = useMemo(() => {
    const d = new Map<string, number>();
    for (const e of modelEdges) {
      d.set(e.source, (d.get(e.source) ?? 0) + 1);
      d.set(e.target, (d.get(e.target) ?? 0) + 1);
    }
    return d;
  }, [modelEdges]);

  const graphData = useMemo(() => {
    const nodes: ForceNode[] = modelNodes.map((n) => ({
      ...n,
      degree: degree.get(n.id) ?? 0,
      color: colorForNode(n),
    }));
    const present = new Set(nodes.map((n) => n.id));
    const links: ForceLink[] = modelEdges
      .filter((e) => present.has(e.source) && present.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, label: e.label }));
    return { nodes, links };
  }, [modelNodes, modelEdges, degree]);

  const nodeById = useMemo(() => {
    const m = new Map<string, EcosystemNode>();
    for (const n of ECOSYSTEM_NODES) m.set(n.id, n);
    return m;
  }, []);

  const edgesForNode = useCallback(
    (id: string) => ({
      incoming: ECOSYSTEM_EDGES.filter((e) => e.target === id),
      outgoing: ECOSYSTEM_EDGES.filter((e) => e.source === id),
    }),
    []
  );

  // Re-fit whenever the visible set changes (filter toggled).
  useEffect(() => {
    const t = setTimeout(() => fgRef.current?.zoomToFit?.(400, 55), 250);
    return () => clearTimeout(t);
  }, []);

  const handleNodeClick = useCallback((node: ForceNode) => {
    setSelected(node);
  }, []);

  const paintNode = useCallback(
    (node: ForceNode, ctx: CanvasRenderingContext2D, globalScale: number) => {
      const radius = 7 + Math.sqrt(node.degree) * 2.6 + (node.type === "capability" ? 3 : 0);
      const isCap = node.type === "capability";

      if (isCap) {
        // Capabilities are squares — visually distinct from round apps.
        const s = radius;
        ctx.beginPath();
        ctx.rect(node.x! - s, node.y! - s, s * 2, s * 2);
        ctx.fillStyle = node.color;
        ctx.fill();
        if (selected?.id === node.id) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2 / globalScale;
          ctx.stroke();
        }
      } else {
        ctx.beginPath();
        ctx.arc(node.x!, node.y!, radius, 0, 2 * Math.PI);
        ctx.fillStyle = node.color;
        ctx.fill();
        if (selected?.id === node.id) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2 / globalScale;
          ctx.stroke();
        }
      }

      const fontSize = Math.max(11 / globalScale, 3);
      ctx.font = `${fontSize}px ui-sans-serif, system-ui`;
      ctx.fillStyle = "rgba(229,231,235,0.92)";
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      ctx.fillText(node.name, node.x!, node.y! + radius + 2 / globalScale);
    },
    [selected]
  );

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_380px]">
      <Card className="overflow-hidden">
        <CardContent className="p-0">
          {/* Kind filter */}
          <div className="flex flex-wrap items-center gap-2 border-b p-3">
            <Filter className="h-4 w-4 text-muted-foreground" />
            <button
              type="button"
              onClick={() => setKindFilter(null)}
              className={cn(
                "rounded px-2.5 py-1 text-xs font-medium transition-colors",
                kindFilter === null
                  ? "bg-foreground text-background"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              All
            </button>
            {APP_KINDS.map((k) => (
              <button
                key={k}
                type="button"
                onClick={() => setKindFilter(kindFilter === k ? null : k)}
                className={cn(
                  "inline-flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition-colors",
                  kindFilter === k
                    ? "bg-foreground text-background"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                <span
                  className="h-2 w-2 rounded-full"
                  style={{ backgroundColor: NODE_COLORS[k] }}
                />
                {KIND_LABELS[k]}
              </button>
            ))}
            <span className="ml-auto text-xs text-muted-foreground">
              click a capability to see how to wire it
            </span>
          </div>

          {/* Legend */}
          <div className="flex flex-wrap items-center gap-3 border-b px-3 py-2">
            <span className="inline-flex items-center gap-1.5 text-[11px]">
              <span className="h-2.5 w-2.5" style={{ backgroundColor: NODE_COLORS.capability }} />
              <span className="text-foreground">Agent capability</span>
              <span className="text-muted-foreground">(square)</span>
            </span>
            <span className="inline-flex items-center gap-1.5 text-[11px]">
              <span className="h-2.5 w-2.5 rounded-full bg-muted-foreground" />
              <span className="text-foreground">Public app / endpoint</span>
              <span className="text-muted-foreground">(circle)</span>
            </span>
            <span className="ml-auto text-[11px] text-muted-foreground">
              directed edges — hover to read the relation
            </span>
          </div>

          {/* Graph canvas */}
          <div ref={wrapRef} className="h-[600px] w-full overflow-hidden bg-[#0b0e14]">
            <ForceGraph2D
              ref={fgRef as never}
              width={size.w}
              height={size.h}
              graphData={graphData}
              nodeRelSize={1}
              nodeCanvasObject={paintNode as never}
              nodePointerAreaPaint={
                ((node: ForceNode, color: string, ctx: CanvasRenderingContext2D) => {
                  const radius =
                    7 + Math.sqrt(node.degree) * 2.6 + (node.type === "capability" ? 3 : 0);
                  ctx.fillStyle = color;
                  ctx.beginPath();
                  ctx.arc(node.x!, node.y!, radius + 3, 0, 2 * Math.PI);
                  ctx.fill();
                }) as never
              }
              linkColor={(() => "rgba(148,163,184,0.3)") as never}
              linkWidth={0.8}
              linkDirectionalArrowLength={4}
              linkDirectionalArrowRelPos={1}
              linkLabel={((l: ForceLink) => l.label) as never}
              onNodeClick={handleNodeClick as never}
              onBackgroundClick={() => setSelected(null)}
              cooldownTicks={150}
              onEngineStop={(() => fgRef.current?.zoomToFit?.(400, 55)) as never}
              nodeLabel={((n: ForceNode) => `${n.name} — ${n.summary}`) as never}
            />
          </div>
        </CardContent>
      </Card>

      {/* Detail panel */}
      <Card className="h-fit lg:sticky lg:top-4">
        <CardContent className="pt-6">
          {!selected ? (
            <div className="text-sm text-muted-foreground">
              <p className="mb-2 font-medium text-foreground">
                The ecosystem your agent can plug into
              </p>
              <p>
                <span className="font-medium text-foreground">Purple squares</span> are things your
                agent can <em>do</em>.{" "}
                <span className="font-medium text-foreground">Coloured circles</span> are the real,
                public apps and endpoints that give it those abilities.
              </p>
              <p className="mt-3">
                Click a capability to see exactly which app(s) provide it — with copy-paste install,
                MCP config, or curl. Click an app to see its access details and what it unlocks.
              </p>
              <p className="mt-3 text-xs">
                Use the filter to focus on MCP servers, CLIs, SDKs, API endpoints, registries, or
                hosted flows.
              </p>
            </div>
          ) : (
            <NodeDetail
              node={selected}
              edges={edgesForNode(selected.id)}
              nodeById={nodeById}
              onPick={(id) => {
                const n =
                  graphData.nodes.find((x) => x.id === id) ??
                  ({ ...nodeById.get(id), degree: 0, color: "#64748b" } as ForceNode);
                if (n.id) setSelected(n);
              }}
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function NodeDetail({
  node,
  edges,
  nodeById,
  onPick,
}: {
  node: ForceNode;
  edges: {
    incoming: { source: string; label: string }[];
    outgoing: { target: string; label: string }[];
  };
  nodeById: Map<string, EcosystemNode>;
  onPick: (id: string) => void;
}) {
  const labelFor = (id: string) => nodeById.get(id)?.name ?? id;
  const isCap = node.type === "capability";

  // For a capability, "providers" are its outgoing edges (apps). For an app,
  // "unlocks" are its incoming edges (capabilities).
  const related = isCap ? edges.outgoing.map((e) => e.target) : edges.incoming.map((e) => e.source);

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <span
          className={cn("h-3 w-3", isCap ? "" : "rounded-full")}
          style={{ backgroundColor: node.color }}
        />
        {isCap ? (
          <Badge variant="outline" className="gap-1 font-mono text-[10px] text-purple-400">
            <Sparkles className="h-3 w-3" />
            agent capability
          </Badge>
        ) : (
          node.kind && (
            <Badge variant="outline" className="font-mono text-[10px]">
              {KIND_LABELS[node.kind]}
            </Badge>
          )
        )}
      </div>

      <div>
        <div className="text-base font-semibold text-foreground">{node.name}</div>
      </div>

      <p className="text-sm text-muted-foreground">{node.summary}</p>

      {node.agentGets && (
        <div className="rounded-md border border-primary/30 bg-primary/5 p-3">
          <div className="mb-1 text-[11px] font-medium uppercase tracking-wide text-primary">
            What your agent gets
          </div>
          <p className="text-xs text-foreground">{node.agentGets}</p>
        </div>
      )}

      {node.tools && node.tools.length > 0 && (
        <div>
          <div className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Backed by MCP tools
          </div>
          <div className="flex flex-wrap gap-1.5">
            {node.tools.map((t) => (
              <code
                key={t}
                className="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px] text-foreground"
              >
                {t}
              </code>
            ))}
          </div>
        </div>
      )}

      {node.snippets && node.snippets.length > 0 && (
        <div className="space-y-3">
          {node.snippets.map((s) => (
            <SnippetBlock key={s.label} snippet={s} />
          ))}
        </div>
      )}

      {node.links && node.links.length > 0 && (
        <div>
          <div className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Links
          </div>
          <div className="flex flex-col gap-1">
            {node.links.map((l) => {
              const external = l.href.startsWith("http");
              return (
                <Link
                  key={l.href}
                  href={l.href}
                  target={external ? "_blank" : undefined}
                  rel={external ? "noopener noreferrer" : undefined}
                  className="inline-flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
                >
                  <ExternalLink className="h-3 w-3 shrink-0" />
                  <span className="truncate">{l.label}</span>
                </Link>
              );
            })}
          </div>
        </div>
      )}

      {related.length > 0 && (
        <div>
          <div className="mb-1.5 text-xs font-medium text-foreground">
            {isCap
              ? `Provided by (${edges.outgoing.length})`
              : `Unlocks (${edges.incoming.length})`}
          </div>
          <div className="space-y-1">
            {(isCap ? edges.outgoing : edges.incoming).map((e) => {
              const other = isCap
                ? (e as { target: string }).target
                : (e as { source: string }).source;
              return (
                <button
                  key={`${e.label}-${other}`}
                  type="button"
                  onClick={() => onPick(other)}
                  className="flex w-full flex-col gap-0.5 rounded border bg-muted/10 px-2 py-1.5 text-left text-xs hover:bg-muted/30"
                >
                  <span className="text-muted-foreground">{e.label || "connects to"}</span>
                  <span className="truncate font-medium text-foreground">{labelFor(other)}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

function SnippetBlock({ snippet }: { snippet: CodeSnippet }) {
  const [copied, setCopied] = useState(false);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(snippet.code);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard may be unavailable (no https / no permission) — fail quietly.
    }
  }, [snippet.code]);

  return (
    <div>
      <div className="mb-1 flex items-center justify-between gap-2">
        <span className="text-[11px] font-medium text-muted-foreground">{snippet.label}</span>
        <button
          type="button"
          onClick={copy}
          className="inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground"
          aria-label="Copy to clipboard"
        >
          {copied ? <Check className="h-3 w-3 text-emerald-400" /> : <Copy className="h-3 w-3" />}
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="overflow-x-auto rounded-md border bg-[#0b0e14] px-3 py-2 text-[11px] leading-relaxed text-foreground">
        <code className="font-mono">{snippet.code}</code>
      </pre>
    </div>
  );
}
