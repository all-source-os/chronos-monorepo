"use client";

import { cn } from "@allsource/ui";
import {
  ArrowLeft,
  ArrowRight,
  Bot,
  Braces,
  Check,
  GitBranch,
  History,
  Network,
  Radio,
  Rows3,
} from "lucide-react";
import { useMemo, useState } from "react";
import {
  CAPABILITY_EVENTS,
  formatEventTime,
  graphAt,
  type McpToolName,
  mcpExchange,
  projectionAt,
  reconstructOrderState,
} from "@/lib/capability-demo";

const mcpTools: { name: McpToolName; description: string }[] = [
  { name: "event_timeline", description: "Read ordered history" },
  { name: "reconstruct_state", description: "Ask what was true" },
  { name: "query_events", description: "Filter raw events" },
];

const nodeTone: Record<string, string> = {
  customer: "border-sky-500 bg-sky-500/15",
  order: "border-primary bg-primary/15",
  payment: "border-violet-500 bg-violet-500/15",
  inventory: "border-amber-500 bg-amber-500/15",
  shipment: "border-emerald-500 bg-emerald-500/15",
};

function Panel({
  id,
  eyebrow,
  title,
  description,
  icon: Icon,
  className,
  children,
}: {
  id: string;
  eyebrow: string;
  title: string;
  description: string;
  icon: React.ElementType;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className={cn("scroll-mt-24 border border-border bg-card", className)}>
      <header className="border-b border-border px-5 py-4 sm:px-6">
        <div className="flex items-center gap-2 font-mono text-[0.68rem] font-semibold uppercase tracking-[0.18em] text-primary">
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
          {eyebrow}
        </div>
        <h2 className="mt-2 text-xl font-semibold tracking-tight sm:text-2xl">{title}</h2>
        <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p>
      </header>
      {children}
    </section>
  );
}

function EventRail({ cursor, onChange }: { cursor: number; onChange: (cursor: number) => void }) {
  const current = CAPABILITY_EVENTS[cursor]!;

  return (
    <div className="border-b border-border bg-muted/20 px-5 py-5 sm:px-6">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="font-mono text-[0.68rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            Shared history cursor
          </p>
          <p className="mt-1 text-sm text-foreground">
            As of{" "}
            <span className="font-mono text-primary">{formatEventTime(current.timestamp)} UTC</span>
            <span className="text-muted-foreground"> · version {current.version}</span>
          </p>
        </div>
        <p className="font-mono text-xs text-muted-foreground" aria-live="polite">
          {cursor + 1} / {CAPABILITY_EVENTS.length} events applied
        </p>
      </div>

      <div className="relative mt-5 px-2">
        <div className="absolute left-4 right-4 top-[0.44rem] h-px bg-border" aria-hidden="true" />
        <ol className="relative grid grid-cols-6 gap-1">
          {CAPABILITY_EVENTS.map((event, index) => {
            const selected = index === cursor;
            const applied = index <= cursor;

            return (
              <li key={event.id} className="min-w-0">
                <button
                  type="button"
                  onClick={() => onChange(index)}
                  className="group flex w-full flex-col items-center gap-2 text-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  aria-label={`Travel to version ${event.version}: ${event.eventType}`}
                  aria-current={selected ? "step" : undefined}
                >
                  <span
                    className={cn(
                      "z-10 h-3.5 w-3.5 rounded-full border-2 transition-colors motion-reduce:transition-none",
                      selected
                        ? "border-primary bg-primary ring-4 ring-primary/15"
                        : applied
                          ? "border-emerald-500 bg-emerald-500"
                          : "border-border bg-card group-hover:border-foreground/50"
                    )}
                  />
                  <span
                    className={cn(
                      "hidden truncate font-mono text-[0.63rem] sm:block",
                      selected ? "text-foreground" : "text-muted-foreground"
                    )}
                  >
                    v{event.version}
                  </span>
                </button>
              </li>
            );
          })}
        </ol>
        <label className="sr-only" htmlFor="history-cursor">
          Event history position
        </label>
        <input
          id="history-cursor"
          type="range"
          min={0}
          max={CAPABILITY_EVENTS.length - 1}
          step={1}
          value={cursor}
          onChange={(event) => onChange(Number(event.target.value))}
          className="mt-4 w-full accent-[var(--primary)]"
        />
      </div>
    </div>
  );
}

function TimelinePanel({
  cursor,
  onChange,
}: {
  cursor: number;
  onChange: (cursor: number) => void;
}) {
  return (
    <Panel
      id="event-timeline"
      eyebrow="Source history"
      title="Event Timeline"
      description="Inspect every fact in sequence. Select one event to move every other view to that exact point."
      icon={Rows3}
      className="lg:col-span-5"
    >
      <ol className="divide-y divide-border">
        {CAPABILITY_EVENTS.map((event, index) => {
          const selected = index === cursor;
          const future = index > cursor;

          return (
            <li key={event.id}>
              <button
                type="button"
                onClick={() => onChange(index)}
                className={cn(
                  "grid w-full grid-cols-[3.25rem_minmax(0,1fr)_auto] gap-3 px-5 py-4 text-left transition-colors hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring sm:px-6",
                  selected && "bg-primary/[0.07]",
                  future && "opacity-45"
                )}
                aria-pressed={selected}
              >
                <span className="font-mono text-xs text-muted-foreground">
                  {formatEventTime(event.timestamp)}
                </span>
                <span className="min-w-0">
                  <span className="block truncate font-mono text-xs font-semibold text-foreground">
                    {event.eventType}
                  </span>
                  <span className="mt-1 block text-xs leading-5 text-muted-foreground">
                    {event.summary}
                  </span>
                </span>
                <span
                  className={cn(
                    "mt-0.5 flex h-6 w-6 items-center justify-center rounded-full border font-mono text-[0.65rem]",
                    selected
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-border bg-background text-muted-foreground"
                  )}
                >
                  {event.version}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
    </Panel>
  );
}

function TimeTravelPanel({
  cursor,
  onChange,
}: {
  cursor: number;
  onChange: (cursor: number) => void;
}) {
  const state = reconstructOrderState(cursor);
  const event = CAPABILITY_EVENTS[cursor]!;
  const fields = [
    ["status", state.status],
    ["total", state.total],
    ["payment", state.payment],
    ["inventory", state.inventory],
    ["delivery_postcode", state.postcode],
    ["shipment", state.shipment],
  ];

  return (
    <Panel
      id="time-travel"
      eyebrow="Point-in-time read"
      title="Time travel without a second database"
      description="Replay source events through a timestamp. Current state and historical state come from the same durable stream."
      icon={History}
      className="lg:col-span-7"
    >
      <div className="grid sm:grid-cols-[minmax(0,1fr)_13rem]">
        <dl className="grid grid-cols-1 divide-y divide-border sm:grid-cols-2 sm:divide-x sm:divide-y-0">
          <div className="divide-y divide-border">
            {fields.slice(0, 3).map(([label, value]) => (
              <div key={label} className="px-5 py-4 sm:px-6">
                <dt className="font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground">
                  {label}
                </dt>
                <dd className="mt-1 text-sm font-medium text-foreground">{value}</dd>
              </div>
            ))}
          </div>
          <div className="divide-y divide-border border-t border-border sm:border-t-0">
            {fields.slice(3).map(([label, value]) => (
              <div key={label} className="px-5 py-4 sm:px-6">
                <dt className="font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground">
                  {label}
                </dt>
                <dd className="mt-1 text-sm font-medium text-foreground">{value}</dd>
              </div>
            ))}
          </div>
        </dl>
        <aside className="flex flex-col justify-between border-t border-border bg-muted/20 p-5 sm:border-l sm:border-t-0">
          <div>
            <p className="font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground">
              Reconstructed as_of
            </p>
            <p className="mt-2 break-all font-mono text-xs leading-5 text-primary">
              {event.timestamp}
            </p>
          </div>
          <div className="mt-8 flex gap-2">
            <button
              type="button"
              onClick={() => onChange(Math.max(0, cursor - 1))}
              disabled={cursor === 0}
              className="flex h-9 flex-1 items-center justify-center border border-border bg-background text-foreground hover:border-foreground/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-35"
              aria-label="Previous event"
            >
              <ArrowLeft className="h-4 w-4" />
            </button>
            <button
              type="button"
              onClick={() => onChange(Math.min(CAPABILITY_EVENTS.length - 1, cursor + 1))}
              disabled={cursor === CAPABILITY_EVENTS.length - 1}
              className="flex h-9 flex-1 items-center justify-center border border-border bg-background text-foreground hover:border-foreground/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-35"
              aria-label="Next event"
            >
              <ArrowRight className="h-4 w-4" />
            </button>
          </div>
        </aside>
      </div>
    </Panel>
  );
}

function GraphPanel({ cursor }: { cursor: number }) {
  const graph = graphAt(cursor);
  const [selectedNodeId, setSelectedNodeId] = useState("order-1042");
  const selectedNode = graph.nodes.find((node) => node.id === selectedNodeId) ?? graph.nodes[0]!;
  const nodesById = new Map(graph.nodes.map((node) => [node.id, node]));

  return (
    <Panel
      id="graph-visualisation"
      eyebrow="Prime relationship view"
      title="Graph visualisation with provenance"
      description="Relationships appear only after their source event exists. Click a node to inspect what the graph knows at this point in history."
      icon={Network}
      className="lg:col-span-7"
    >
      <div className="grid lg:grid-cols-[minmax(0,1fr)_13rem]">
        <div className="min-w-0 overflow-x-auto p-3 sm:p-5">
          <svg
            viewBox="0 0 700 310"
            className="h-auto min-h-48 w-full sm:min-h-72"
            role="img"
            aria-label={`Relationship graph with ${graph.nodes.length} nodes and ${graph.edges.length} edges`}
          >
            <title>{`Order relationship graph as of event ${cursor + 1}`}</title>
            {graph.edges.map((edge) => {
              const source = nodesById.get(edge.source)!;
              const target = nodesById.get(edge.target)!;
              const midX = (source.x + target.x) / 2;
              const midY = (source.y + target.y) / 2;
              return (
                <g key={`${edge.source}-${edge.target}`}>
                  <line
                    x1={source.x}
                    y1={source.y}
                    x2={target.x}
                    y2={target.y}
                    className="stroke-border"
                    strokeWidth="2"
                  />
                  <rect
                    x={midX - 35}
                    y={midY - 9}
                    width="70"
                    height="18"
                    rx="3"
                    className="fill-background stroke-border"
                  />
                  <text
                    x={midX}
                    y={midY + 3}
                    textAnchor="middle"
                    className="fill-muted-foreground font-mono text-[9px]"
                  >
                    {edge.label}
                  </text>
                </g>
              );
            })}
            {graph.nodes.map((node) => {
              const selected = selectedNode.id === node.id;
              return (
                <foreignObject
                  key={node.id}
                  x={node.x - 68}
                  y={node.y - 25}
                  width="136"
                  height="50"
                >
                  <button
                    type="button"
                    onClick={() => setSelectedNodeId(node.id)}
                    className={cn(
                      "flex h-full w-full flex-col items-center justify-center rounded-md border text-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                      nodeTone[node.type],
                      selected && "border-[3px]"
                    )}
                    aria-pressed={selected}
                    aria-label={`Inspect ${node.label}`}
                  >
                    <span className="text-xs font-semibold text-foreground">{node.label}</span>
                    <span className="mt-0.5 font-mono text-[9px] text-muted-foreground">
                      {node.type}
                    </span>
                  </button>
                </foreignObject>
              );
            })}
          </svg>
        </div>
        <aside className="border-t border-border bg-muted/20 p-5 lg:border-l lg:border-t-0">
          <p className="font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground">
            Selected node
          </p>
          <p className="mt-3 text-base font-semibold">{selectedNode.label}</p>
          <p className="mt-1 font-mono text-xs text-primary">{selectedNode.id}</p>
          <p className="mt-4 text-sm leading-6 text-muted-foreground">{selectedNode.detail}</p>
          <div className="mt-6 border-t border-border pt-4 font-mono text-[0.68rem] text-muted-foreground">
            {graph.nodes.length} nodes · {graph.edges.length} edges
          </div>
        </aside>
      </div>
    </Panel>
  );
}

function PipelinePanel({ cursor }: { cursor: number }) {
  const event = CAPABILITY_EVENTS[cursor]!;
  const route = event.eventType.startsWith("payment.")
    ? "finance"
    : event.eventType.startsWith("inventory.") || event.eventType.startsWith("delivery.")
      ? "fulfilment"
      : "orders";
  const stages = [
    { operator: "filter", value: "entity_id = order-1042", result: "pass" },
    { operator: "map", value: "normalize payload", result: "done" },
    { operator: "branch", value: `route → ${route}`, result: route },
  ];

  return (
    <Panel
      id="pipelines"
      eyebrow="Core inline processing"
      title="Trace one event through a pipeline"
      description="Filter, map, and branch accepted events without hiding which event produced each result."
      icon={GitBranch}
      className="lg:col-span-5"
    >
      <div className="px-5 py-5 sm:px-6">
        <div className="border border-border bg-muted/20 px-4 py-3">
          <p className="font-mono text-[0.65rem] uppercase tracking-[0.14em] text-muted-foreground">
            Pipeline input
          </p>
          <p className="mt-1 truncate font-mono text-xs text-foreground">{event.eventType}</p>
        </div>
        <ol className="mt-4 space-y-0">
          {stages.map((stage, index) => (
            <li
              key={stage.operator}
              className="relative grid grid-cols-[1.75rem_minmax(0,1fr)_auto] gap-3"
            >
              {index < stages.length - 1 && (
                <span
                  className="absolute bottom-0 left-[0.84rem] top-7 w-px bg-border"
                  aria-hidden="true"
                />
              )}
              <span className="z-10 mt-3 flex h-7 w-7 items-center justify-center rounded-full border border-emerald-500/50 bg-background text-emerald-500">
                <Check className="h-3.5 w-3.5" />
              </span>
              <div className="min-w-0 py-3">
                <p className="font-mono text-xs font-semibold text-foreground">
                  {stage.operator}()
                </p>
                <p className="mt-1 truncate font-mono text-[0.68rem] text-muted-foreground">
                  {stage.value}
                </p>
              </div>
              <span className="my-3 self-start border border-border bg-muted/30 px-2 py-1 font-mono text-[0.62rem] uppercase text-muted-foreground">
                {stage.result}
              </span>
            </li>
          ))}
        </ol>
      </div>
      <div className="border-t border-border bg-muted/20 px-5 py-4 text-xs leading-5 text-muted-foreground sm:px-6">
        Pipeline transforms run in Core. Query Service projections below fold source history into
        tenant-scoped read models; they are a separate read-plane concern.
      </div>
    </Panel>
  );
}

function ProjectionPanel({ cursor }: { cursor: number }) {
  const projection = projectionAt(cursor);
  const rows = Object.entries(projection.state);

  return (
    <Panel
      id="projections"
      eyebrow="Query Service read model"
      title="Projection state you can rebuild"
      description="Fold the same ordered history into a query-ready shape, then keep it current as new events arrive."
      icon={Braces}
      className="lg:col-span-5"
    >
      <div className="flex items-center justify-between border-b border-border bg-muted/20 px-5 py-3 sm:px-6">
        <code className="font-mono text-xs text-primary">{projection.projection}</code>
        <span className="flex items-center gap-1.5 font-mono text-[0.65rem] text-emerald-500">
          <Radio className="h-3 w-3" /> ready · v{projection.version}
        </span>
      </div>
      <dl className="divide-y divide-border">
        {rows.map(([key, value]) => (
          <div
            key={key}
            className="grid grid-cols-[minmax(7rem,0.8fr)_minmax(0,1fr)] gap-3 px-5 py-3 sm:px-6"
          >
            <dt className="font-mono text-[0.68rem] text-muted-foreground">{key}</dt>
            <dd className="truncate text-right font-mono text-xs font-medium text-foreground">
              {value}
            </dd>
          </div>
        ))}
      </dl>
      <div className="border-t border-border px-5 py-4 font-mono text-[0.65rem] text-muted-foreground sm:px-6">
        {projection.applied_events} events folded · HTTP, realtime, or analytics read paths
      </div>
    </Panel>
  );
}

function McpPanel({ cursor }: { cursor: number }) {
  const [tool, setTool] = useState<McpToolName>("event_timeline");
  const exchange = useMemo(() => mcpExchange(tool, cursor), [tool, cursor]);

  return (
    <Panel
      id="mcp-data-access"
      eyebrow="Agent tool interface"
      title="MCP data access agents can reason over"
      description="Use explicit, tenant-scoped tools instead of pasting whole histories into prompts. These are real event-store connector tool shapes."
      icon={Bot}
      className="lg:col-span-7"
    >
      <div className="border-b border-border p-2 sm:p-3">
        <fieldset className="grid gap-2 sm:grid-cols-3">
          <legend className="sr-only">Choose an MCP tool</legend>
          {mcpTools.map((item) => (
            <button
              key={item.name}
              type="button"
              onClick={() => setTool(item.name)}
              className={cn(
                "border px-3 py-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                tool === item.name
                  ? "border-primary bg-primary/[0.08]"
                  : "border-border hover:border-foreground/30"
              )}
              aria-pressed={tool === item.name}
            >
              <span className="block font-mono text-[0.7rem] font-semibold text-foreground">
                {item.name}
              </span>
              <span className="mt-1 block text-[0.68rem] text-muted-foreground">
                {item.description}
              </span>
            </button>
          ))}
        </fieldset>
      </div>
      <div className="grid min-w-0 lg:grid-cols-2">
        <div className="min-w-0 border-b border-border lg:border-b-0 lg:border-r">
          <div className="flex items-center justify-between border-b border-border bg-muted/20 px-4 py-2 font-mono text-[0.65rem] uppercase tracking-[0.12em] text-muted-foreground">
            <span>tools/call</span>
            <span>request</span>
          </div>
          <pre className="max-h-80 overflow-auto p-4 font-mono text-[0.7rem] leading-5 text-foreground">
            <code>{JSON.stringify({ name: tool, arguments: exchange.request }, null, 2)}</code>
          </pre>
        </div>
        <div className="min-w-0">
          <div className="flex items-center justify-between border-b border-border bg-muted/20 px-4 py-2 font-mono text-[0.65rem] uppercase tracking-[0.12em] text-muted-foreground">
            <span>content</span>
            <span className="text-emerald-500">success</span>
          </div>
          <pre
            className="max-h-80 overflow-auto p-4 font-mono text-[0.7rem] leading-5 text-foreground"
            aria-live="polite"
          >
            <code>{JSON.stringify(exchange.response, null, 2)}</code>
          </pre>
        </div>
      </div>
      <div className="border-t border-border bg-muted/20 px-5 py-3 text-xs text-muted-foreground sm:px-6">
        Interactive fixture · no network request · connector-compatible tool names and parameters
      </div>
    </Panel>
  );
}

export function CapabilityWorkbench() {
  const [cursor, setCursor] = useState(CAPABILITY_EVENTS.length - 1);
  const current = CAPABILITY_EVENTS[cursor]!;

  return (
    <section
      id="capability-workbench"
      className="scroll-mt-24 border border-border bg-background shadow-2xl shadow-primary/5"
    >
      <header className="grid border-b border-border lg:grid-cols-[minmax(0,1fr)_auto]">
        <div className="px-5 py-6 sm:px-6 lg:px-8">
          <div className="flex items-center gap-2 font-mono text-[0.68rem] font-semibold uppercase tracking-[0.2em] text-primary">
            <span className="h-2 w-2 rounded-full bg-emerald-500" aria-hidden="true" />
            Interactive capability lab
          </div>
          <h2 className="mt-3 text-2xl font-semibold tracking-tight sm:text-3xl">
            One event stream. Six ways to use it.
          </h2>
          <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground sm:text-base sm:leading-7">
            Follow one order from creation to dispatch. Move through its history and watch every
            AllSource surface answer from the same source events.
          </p>
        </div>
        <div className="grid grid-cols-2 border-t border-border text-xs lg:w-72 lg:border-l lg:border-t-0">
          <div className="border-r border-border px-4 py-5">
            <p className="font-mono uppercase tracking-[0.14em] text-muted-foreground">Entity</p>
            <p className="mt-2 font-mono font-semibold text-foreground">{current.entityId}</p>
          </div>
          <div className="px-4 py-5">
            <p className="font-mono uppercase tracking-[0.14em] text-muted-foreground">Fixture</p>
            <p className="mt-2 font-mono font-semibold text-foreground">local · safe</p>
          </div>
        </div>
      </header>

      <nav
        aria-label="Capability workbench sections"
        className="flex snap-x gap-px overflow-x-auto border-b border-border bg-border"
      >
        {[
          ["#event-timeline", "Timeline"],
          ["#time-travel", "Time travel"],
          ["#graph-visualisation", "Graph"],
          ["#pipelines", "Pipelines"],
          ["#projections", "Projections"],
          ["#mcp-data-access", "MCP access"],
        ].map(([href, label]) => (
          <a
            key={href}
            href={href}
            className="min-w-fit flex-1 snap-start bg-background px-4 py-3 text-center font-mono text-[0.68rem] font-semibold uppercase tracking-[0.12em] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
          >
            {label}
          </a>
        ))}
      </nav>

      <EventRail cursor={cursor} onChange={setCursor} />

      <div className="grid gap-4 p-4 lg:grid-cols-12 lg:p-6">
        <TimelinePanel cursor={cursor} onChange={setCursor} />
        <TimeTravelPanel cursor={cursor} onChange={setCursor} />
        <GraphPanel cursor={cursor} />
        <PipelinePanel cursor={cursor} />
        <ProjectionPanel cursor={cursor} />
        <McpPanel cursor={cursor} />
      </div>
    </section>
  );
}
