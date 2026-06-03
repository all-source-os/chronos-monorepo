"use client";

import { Badge, Card, CardContent, cn } from "@allsource/ui";
import { Layers, Sparkles } from "lucide-react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  C4_LEVELS,
  C4_TYPE_COLORS,
  C4_TYPE_LABELS,
  type C4ElementType,
  type C4Node,
} from "@/data/c4-model";

// react-force-graph-2d paints to a <canvas> and touches `window` at module
// scope, so it MUST be client-only — a static import crashes the Next build
// during SSR. Dynamic import with ssr:false keeps it off the server. This is
// the same pattern the Memory PrimeGraph uses.
const ForceGraph2D = dynamic(() => import("react-force-graph-2d"), {
  ssr: false,
  loading: () => (
    <div className="flex h-[600px] items-center justify-center text-sm text-muted-foreground">
      Loading diagram…
    </div>
  ),
});

type LevelId = "context" | "container";

// react-force-graph mutates link.source/target into node refs at runtime.
interface ForceNode extends C4Node {
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

const LEVEL_ORDER: { id: LevelId; label: string }[] = [
  { id: "context", label: "L1 · System Context" },
  { id: "container", label: "L2 · Containers" },
];

export function C4Graph() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const levelParam = searchParams.get("level");
  const level: LevelId = levelParam === "container" ? "container" : "context";

  const [selected, setSelected] = useState<ForceNode | null>(null);

  // react-force-graph-2d defaults its canvas to the WINDOW size, which makes it
  // overflow the card. Measure the wrapper and pass explicit width/height so the
  // canvas is clipped to the 600px graph box.
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

  // Give the bubbles a bit more room than the library defaults so labels don't
  // overlap. The ForceGraph is a dynamic (ssr:false) import, so its ref may be
  // null on the first effect pass — poll briefly until d3Force is available.
  // We set the forces BEFORE the simulation settles and do NOT reheat, so the
  // single initial cooldown produces a stable layout that zoomToFit can frame.
  useEffect(() => {
    let tries = 0;
    const apply = () => {
      const fg = fgRef.current;
      if (!fg?.d3Force) {
        if (tries++ < 40) setTimeout(apply, 50);
        return;
      }
      fg.d3Force("charge")?.strength?.(-280);
      fg.d3Force("link")?.distance?.(70);
    };
    apply();
  }, [level]);

  const setLevel = useCallback(
    (next: LevelId) => {
      setSelected(null);
      const params = new URLSearchParams(searchParams.toString());
      params.set("level", next);
      router.replace(`/architecture?${params.toString()}`, { scroll: false });
    },
    [router, searchParams]
  );

  const model = C4_LEVELS[level];

  const degree = useMemo(() => {
    const d = new Map<string, number>();
    for (const e of model.edges) {
      d.set(e.source, (d.get(e.source) ?? 0) + 1);
      d.set(e.target, (d.get(e.target) ?? 0) + 1);
    }
    return d;
  }, [model.edges]);

  const graphData = useMemo(() => {
    const nodes: ForceNode[] = model.nodes.map((n) => ({
      ...n,
      degree: degree.get(n.id) ?? 0,
      color: C4_TYPE_COLORS[n.type],
    }));
    const present = new Set(nodes.map((n) => n.id));
    const links: ForceLink[] = model.edges
      .filter((e) => present.has(e.source) && present.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, label: e.label }));
    return { nodes, links };
  }, [model.nodes, model.edges, degree]);

  const nodeById = useMemo(() => {
    const m = new Map<string, C4Node>();
    for (const n of model.nodes) m.set(n.id, n);
    return m;
  }, [model.nodes]);

  const edgesForNode = useCallback(
    (id: string) => ({
      incoming: model.edges.filter((e) => e.target === id),
      outgoing: model.edges.filter((e) => e.source === id),
    }),
    [model.edges]
  );

  // Element types actually present in this level (for the legend).
  const typesPresent = useMemo(() => {
    const counts = new Map<C4ElementType, number>();
    for (const n of model.nodes) counts.set(n.type, (counts.get(n.type) ?? 0) + 1);
    return Array.from(counts.entries());
  }, [model.nodes]);

  const handleNodeClick = useCallback(
    (node: ForceNode) => {
      // In L1, clicking the AllSource system drills down to L2.
      if (level === "context" && node.id === "allsource") {
        setLevel("container");
        return;
      }
      setSelected(node);
    },
    [level, setLevel]
  );

  const paintNode = useCallback(
    (node: ForceNode, ctx: CanvasRenderingContext2D, globalScale: number) => {
      const radius = 7 + Math.sqrt(node.degree) * 3 + (node.type === "system" ? 6 : 0);

      // Agent-suite ring highlight.
      if (node.agentSuite) {
        ctx.beginPath();
        ctx.arc(node.x!, node.y!, radius + 3, 0, 2 * Math.PI);
        ctx.strokeStyle = "#a855f7";
        ctx.lineWidth = 2 / globalScale;
        ctx.stroke();
      }

      ctx.beginPath();
      ctx.arc(node.x!, node.y!, radius, 0, 2 * Math.PI);
      ctx.fillStyle = node.color;
      ctx.fill();

      if (selected?.id === node.id) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 2 / globalScale;
        ctx.stroke();
      }

      // Always-on label — this is a small curated graph, clutter is fine.
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
    <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
      <Card className="overflow-hidden">
        <CardContent className="p-0">
          {/* Toggle */}
          <div className="flex flex-wrap items-center gap-2 border-b p-3">
            <Layers className="h-4 w-4 text-muted-foreground" />
            <div className="inline-flex rounded-md border p-0.5">
              {LEVEL_ORDER.map((lvl) => (
                <button
                  key={lvl.id}
                  type="button"
                  onClick={() => setLevel(lvl.id)}
                  className={cn(
                    "rounded px-3 py-1 text-xs font-medium transition-colors",
                    level === lvl.id
                      ? "bg-foreground text-background"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                >
                  {lvl.label}
                </button>
              ))}
            </div>
            <span className="ml-auto text-xs text-muted-foreground">{model.subtitle}</span>
          </div>

          {/* Legend */}
          <div className="flex flex-wrap items-center gap-2 border-b px-3 py-2">
            {typesPresent.map(([type, count]) => (
              <span
                key={type}
                className="inline-flex items-center gap-1.5 rounded-full border border-transparent px-2 py-0.5 text-[11px]"
              >
                <span
                  className="h-2.5 w-2.5 rounded-full"
                  style={{ backgroundColor: C4_TYPE_COLORS[type] }}
                />
                <span className="text-foreground">{C4_TYPE_LABELS[type]}</span>
                <span className="text-muted-foreground">{count}</span>
              </span>
            ))}
            <span className="ml-auto inline-flex items-center gap-1 text-[11px] text-muted-foreground">
              <span className="h-2.5 w-2.5 rounded-full ring-2 ring-purple-500" />
              agent-suite product
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
                  const radius = 7 + Math.sqrt(node.degree) * 3 + (node.type === "system" ? 6 : 0);
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
              linkDirectionalParticles={0}
              onNodeClick={handleNodeClick as never}
              onBackgroundClick={() => setSelected(null)}
              cooldownTicks={150}
              onEngineStop={(() => fgRef.current?.zoomToFit?.(400, 55)) as never}
              nodeLabel={((n: ForceNode) => `${n.name} — ${n.description}`) as never}
            />
          </div>
        </CardContent>
      </Card>

      {/* Detail panel */}
      <Card className="h-fit lg:sticky lg:top-4">
        <CardContent className="pt-6">
          {!selected ? (
            <div className="text-sm text-muted-foreground">
              <p className="mb-2 font-medium text-foreground">Inspect the architecture</p>
              <p>
                Click any bubble to see its tech, responsibilities, and how it connects. Use the
                toggle to switch between Level 1 (System Context) and Level 2 (Containers).
              </p>
              {level === "context" && (
                <p className="mt-3">
                  Tip: click the <strong className="text-foreground">AllSource Platform</strong>{" "}
                  bubble to drill into the containers.
                </p>
              )}
              <p className="mt-3 text-xs">
                Edges are directed and labelled — hover a connection to read the relationship.
              </p>
            </div>
          ) : (
            <NodeDetail
              node={selected}
              edges={edgesForNode(selected.id)}
              nodeById={nodeById}
              onPick={(id) => {
                const n = graphData.nodes.find((x) => x.id === id);
                if (n) setSelected(n);
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
  nodeById: Map<string, C4Node>;
  onPick: (id: string) => void;
}) {
  const labelFor = (id: string) => nodeById.get(id)?.name ?? id;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <span className="h-3 w-3 rounded-full" style={{ backgroundColor: node.color }} />
        <Badge variant="outline" className="font-mono text-[10px]">
          {C4_TYPE_LABELS[node.type]}
        </Badge>
        {node.agentSuite && (
          <Badge variant="outline" className="gap-1 font-mono text-[10px] text-purple-400">
            <Sparkles className="h-3 w-3" />
            agent suite
          </Badge>
        )}
        {node.fly && (
          <Badge variant="outline" className="font-mono text-[10px] text-sky-400">
            Fly.io
          </Badge>
        )}
        {node.vercel && (
          <Badge variant="outline" className="font-mono text-[10px] text-emerald-400">
            Vercel
          </Badge>
        )}
      </div>

      <div>
        <div className="text-base font-semibold text-foreground">{node.name}</div>
        {node.tech && (
          <code className="font-mono text-[11px] text-muted-foreground">{node.tech}</code>
        )}
      </div>

      <p className="text-sm text-muted-foreground">{node.description}</p>

      {node.responsibilities && node.responsibilities.length > 0 && (
        <div>
          <div className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Responsibilities
          </div>
          <ul className="space-y-1.5">
            {node.responsibilities.map((r) => (
              <li key={r} className="flex gap-2 text-xs text-foreground">
                <span className="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-muted-foreground" />
                <span>{r}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <EdgeGroup
        title={`Outgoing (${edges.outgoing.length})`}
        rows={edges.outgoing.map((e) => ({ label: e.label, other: e.target }))}
        labelFor={labelFor}
        onPick={onPick}
      />
      <EdgeGroup
        title={`Incoming (${edges.incoming.length})`}
        rows={edges.incoming.map((e) => ({ label: e.label, other: e.source }))}
        labelFor={labelFor}
        onPick={onPick}
      />
    </div>
  );
}

function EdgeGroup({
  title,
  rows,
  labelFor,
  onPick,
}: {
  title: string;
  rows: { label: string; other: string }[];
  labelFor: (id: string) => string;
  onPick: (id: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <div>
      <div className="mb-1.5 text-xs font-medium text-foreground">{title}</div>
      <div className="space-y-1">
        {rows.map((r) => (
          <button
            key={`${r.label}-${r.other}`}
            type="button"
            onClick={() => onPick(r.other)}
            className="flex w-full flex-col gap-0.5 rounded border bg-muted/10 px-2 py-1.5 text-left text-xs hover:bg-muted/30"
          >
            <span className="text-muted-foreground">{r.label || "connects to"}</span>
            <span className="truncate font-medium text-foreground">{labelFor(r.other)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
