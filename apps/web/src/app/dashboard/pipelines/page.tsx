"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  ArrowRight,
  BarChart3,
  CalendarDays,
  CheckCircle2,
  ChevronDown,
  Database,
  GitBranch,
  Hash,
  Layers,
  Loader2,
  MoreHorizontal,
  Plus,
  Table2,
  Trash2,
  X,
} from "lucide-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { LoadError } from "@/components/dashboard/load-error";
import {
  apiClient,
  type Projection,
  type ProjectionKind,
  type ProjectionTemplate,
} from "@/lib/api/client";

const ProjectionStateView = dynamic(
  () =>
    import("@/components/dashboard/projection-state-view").then(
      (module) => module.ProjectionStateView
    ),
  {
    ssr: false,
    loading: () => (
      <div
        className="h-64 animate-pulse rounded-lg border border-border bg-muted/30"
        role="status"
        aria-label="Loading projection state"
      />
    ),
  }
);

function statusColor(status: Projection["status"]) {
  return status === "ready" ? "bg-emerald-500" : "bg-amber-500";
}

function projectionKindLabel(kind: ProjectionKind | null) {
  switch (kind) {
    case "counter":
      return "Counter";
    case "breakdown":
      return "Breakdown";
    case "timeseries":
      return "Time series";
    case "entity_table":
      return "Entity view";
    default:
      return "Read model";
  }
}

function projectionKindIcon(kind: ProjectionKind | null) {
  switch (kind) {
    case "counter":
      return Hash;
    case "breakdown":
      return BarChart3;
    case "timeseries":
      return CalendarDays;
    case "entity_table":
      return Table2;
    default:
      return Layers;
  }
}

// entity_table templates that fold one row per entity_id need an entity_id
// filter to read state. active-entities is tenant-wide, so it does not.
function isPerEntityTemplate(projection: Projection): boolean {
  return projection.kind === "entity_table" && projection.name !== "active-entities";
}

function ProjectionTemplateOption({
  template,
  isBusy,
  disabled,
  onEnable,
}: {
  template: ProjectionTemplate;
  isBusy: boolean;
  disabled: boolean;
  onEnable: () => void;
}) {
  const Icon = projectionKindIcon(template.kind);

  return (
    <button
      type="button"
      onClick={onEnable}
      disabled={disabled}
      className="group flex min-h-32 w-full flex-col rounded-lg border border-border bg-background p-4 text-left transition-colors hover:border-primary/50 hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-60"
      aria-label={`Enable ${template.title} projection`}
    >
      <div className="flex w-full items-start justify-between gap-4">
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-muted/50 text-foreground">
          <Icon className="h-4 w-4" aria-hidden="true" />
        </span>
        <span className="flex items-center gap-1 text-xs font-medium text-muted-foreground transition-colors group-hover:text-primary">
          {isBusy ? "Enabling" : "Enable"}
          {isBusy ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
          ) : (
            <ArrowRight
              className="h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5"
              aria-hidden="true"
            />
          )}
        </span>
      </div>
      <span className="mt-4 font-semibold text-foreground">{template.title}</span>
      <span className="mt-1 line-clamp-2 text-sm leading-5 text-muted-foreground">
        {template.description}
      </span>
      <span className="mt-auto pt-4 text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
        {projectionKindLabel(template.kind)}
      </span>
    </button>
  );
}

function ProjectionFlow() {
  const steps = [
    { icon: Database, label: "Source", value: "Event history" },
    { icon: GitBranch, label: "Fold", value: "Projection template" },
    { icon: Table2, label: "Result", value: "Queryable state" },
  ];

  return (
    <div className="grid border-t border-border bg-muted/15 md:grid-cols-[1fr_auto_1fr_auto_1fr]">
      {steps.map((step, index) => {
        const Icon = step.icon;
        return (
          <div key={step.label} className="contents">
            <div className="flex items-center gap-3 px-5 py-4">
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                <Icon className="h-4 w-4" aria-hidden="true" />
              </span>
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                  {step.label}
                </p>
                <p className="mt-0.5 text-sm font-medium">{step.value}</p>
              </div>
            </div>
            {index < steps.length - 1 && (
              <div
                className="hidden items-center text-muted-foreground/60 md:flex"
                aria-hidden="true"
              >
                <ArrowRight className="h-4 w-4" />
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function EmptyProjectionWorkspace({
  templates,
  busy,
  onEnable,
}: {
  templates: ProjectionTemplate[];
  busy: string | null;
  onEnable: (name: string) => void;
}) {
  return (
    <Card className="overflow-hidden border-border/80 py-0">
      <div className="grid lg:grid-cols-[minmax(0,1.5fr)_minmax(280px,0.7fr)]">
        <div className="p-6 md:p-8">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="max-w-2xl">
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">
                Start with a template
              </p>
              <h2 className="mt-2 text-xl font-semibold tracking-tight md:text-2xl">
                Choose your first read model
              </h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                AllSource folds existing events, then keeps your view current as new events arrive.
                Pick the question you want answered first.
              </p>
            </div>
            {templates.length > 0 && (
              <Badge variant="outline" className="w-fit shrink-0 tabular-nums">
                {templates.length} available
              </Badge>
            )}
          </div>

          {templates.length > 0 ? (
            <div className="mt-6 grid gap-3 sm:grid-cols-2">
              {templates.map((template) => (
                <ProjectionTemplateOption
                  key={template.name}
                  template={template}
                  isBusy={busy === template.name}
                  disabled={busy !== null}
                  onEnable={() => onEnable(template.name)}
                />
              ))}
            </div>
          ) : (
            <div className="mt-6 rounded-lg border border-dashed border-border p-5 text-sm text-muted-foreground">
              Projection catalog is empty. Retry after templates become available.
            </div>
          )}
        </div>

        <aside className="border-t border-border bg-muted/20 p-6 lg:border-l lg:border-t-0 lg:p-8">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            What happens next
          </p>
          <ol className="mt-5 space-y-5">
            <li className="flex gap-3">
              <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-border bg-background text-xs font-semibold">
                1
              </span>
              <div>
                <p className="text-sm font-medium">History is folded</p>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  Existing tenant events build initial state.
                </p>
              </div>
            </li>
            <li className="flex gap-3">
              <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-border bg-background text-xs font-semibold">
                2
              </span>
              <div>
                <p className="text-sm font-medium">New events stay in sync</p>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  State updates continuously after backfill.
                </p>
              </div>
            </li>
            <li className="flex gap-3">
              <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-border bg-background text-xs font-semibold">
                3
              </span>
              <div>
                <p className="text-sm font-medium">Rebuild from source</p>
                <p className="mt-1 text-xs leading-5 text-muted-foreground">
                  Replay history without changing source events.
                </p>
              </div>
            </li>
          </ol>
          <div className="mt-6 border-t border-border pt-5">
            <Link
              href="/dashboard/events"
              className="inline-flex items-center gap-1.5 text-sm font-medium text-primary underline-offset-4 hover:underline"
            >
              View event history
              <ArrowRight className="h-3.5 w-3.5" aria-hidden="true" />
            </Link>
          </div>
        </aside>
      </div>
      <ProjectionFlow />
    </Card>
  );
}

function ProjectionCard({
  projection,
  isBusy,
  onDisable,
}: {
  projection: Projection;
  isBusy: boolean;
  onDisable: () => void;
}) {
  const [showMenu, setShowMenu] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const building = projection.status === "building";
  const Icon = projectionKindIcon(projection.kind);

  return (
    <Card className="gap-0 overflow-hidden border-border/80 py-0">
      <CardHeader className="flex flex-row items-start justify-between border-b border-border bg-muted/15 py-5">
        <div className="flex min-w-0 items-start gap-3">
          <div className="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-background text-primary">
            <Icon className="h-4 w-4" aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <CardTitle className="text-base">{projection.title || projection.name}</CardTitle>
              <Badge variant="outline" className="gap-1.5 font-normal capitalize">
                <span
                  className={cn(
                    "h-1.5 w-1.5 rounded-full",
                    statusColor(projection.status),
                    building && "animate-pulse"
                  )}
                  aria-hidden="true"
                />
                {building ? "Building" : "Ready"}
              </Badge>
            </div>
            <CardDescription className="mt-1.5 line-clamp-2 leading-5">
              {projection.description || projection.name}
            </CardDescription>
          </div>
        </div>

        <div className="relative ml-3 shrink-0">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => setShowMenu(!showMenu)}
            disabled={isBusy}
            aria-label={`Actions for ${projection.title || projection.name}`}
            aria-expanded={showMenu}
          >
            {isBusy ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <MoreHorizontal className="h-4 w-4" />
            )}
          </Button>

          {showMenu && (
            <>
              <button
                type="button"
                className="fixed inset-0 z-40 cursor-default"
                onClick={() => setShowMenu(false)}
                aria-label="Close projection actions"
              />
              <div className="absolute right-0 top-full z-50 mt-1 w-36 rounded-lg border border-border bg-popover p-1 shadow-lg">
                <button
                  type="button"
                  onClick={() => {
                    setShowMenu(false);
                    onDisable();
                  }}
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-destructive hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <Trash2 className="h-4 w-4" />
                  Disable
                </button>
              </div>
            </>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-5">
        <div className="flex flex-wrap items-center justify-between gap-3 text-xs">
          <span className="font-mono text-muted-foreground">{projection.name}</span>
          <span className="font-medium text-muted-foreground">
            {projectionKindLabel(projection.kind)}
          </span>
        </div>

        {building ? (
          <div className="mt-5 flex items-center gap-2 rounded-md border border-amber-500/20 bg-amber-500/5 px-3 py-2 text-xs text-muted-foreground">
            <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-amber-500" />
            Folding event history. State appears when backfill completes.
          </div>
        ) : (
          <>
            <button
              type="button"
              onClick={() => setExpanded((value) => !value)}
              className="mt-5 flex items-center gap-1.5 text-sm font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              aria-expanded={expanded}
            >
              <ChevronDown
                className={cn("h-3.5 w-3.5 transition-transform", expanded && "rotate-180")}
                aria-hidden="true"
              />
              {expanded ? "Hide current state" : "View current state"}
            </button>
            {expanded && (
              <ProjectionStateView
                name={projection.name}
                kind={projection.kind}
                isPerEntity={isPerEntityTemplate(projection)}
              />
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function ProjectionPicker({
  open,
  templates,
  busy,
  onOpenChange,
  onEnable,
}: {
  open: boolean;
  templates: ProjectionTemplate[];
  busy: string | null;
  onOpenChange: (open: boolean) => void;
  onEnable: (name: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="relative max-h-[85vh] max-w-2xl overflow-y-auto"
        onClose={() => onOpenChange(false)}
        aria-labelledby="projection-picker-title"
        aria-describedby="projection-picker-description"
      >
        <button
          type="button"
          onClick={() => onOpenChange(false)}
          className="absolute right-4 top-4 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="Close projection picker"
        >
          <X className="h-4 w-4" />
        </button>
        <DialogHeader className="pr-8">
          <DialogTitle id="projection-picker-title">Add a read model</DialogTitle>
          <DialogDescription id="projection-picker-description">
            Choose a curated projection. AllSource backfills history, then updates state as events
            arrive.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 sm:grid-cols-2">
          {templates.map((template) => (
            <ProjectionTemplateOption
              key={template.name}
              template={template}
              isBusy={busy === template.name}
              disabled={busy !== null}
              onEnable={() => onEnable(template.name)}
            />
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default function PipelinesPage() {
  const [projections, setProjections] = useState<Projection[]>([]);
  const [templates, setTemplates] = useState<ProjectionTemplate[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [pageError, setPageError] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const response = await apiClient.listProjections();
    if (response.error) throw new Error(response.error.message);
    setProjections(response.data?.projections ?? []);
  }, []);

  const load = useCallback(async () => {
    setIsLoading(true);
    setHasLoaded(false);
    setPageError(null);
    try {
      const [list, catalog] = await Promise.all([
        apiClient.listProjections(),
        apiClient.listProjectionTemplates(),
      ]);
      if (list.error) throw new Error(list.error.message);
      if (catalog.error) throw new Error(catalog.error.message);
      setProjections(list.data?.projections ?? []);
      setTemplates(catalog.data?.templates ?? []);
      setHasLoaded(true);
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Projection data is unavailable.");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const enabledNames = new Set(projections.map((projection) => projection.name));
  const available = templates.filter((template) => !enabledNames.has(template.name));

  const handleEnable = async (template: string) => {
    setShowAdd(false);
    setBusy(template);
    setPageError(null);
    try {
      const response = await apiClient.enableProjection(template);
      if (response.error) throw new Error(response.error.message);
      await refresh();
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Projection could not be enabled.");
    } finally {
      setBusy(null);
    }
  };

  const handleDisable = async (name: string) => {
    setBusy(name);
    setPageError(null);
    try {
      const response = await apiClient.disableProjection(name);
      if (response.error) throw new Error(response.error.message);
      await refresh();
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Projection could not be disabled.");
    } finally {
      setBusy(null);
    }
  };

  const readyCount = projections.filter((projection) => projection.status === "ready").length;
  const buildingCount = projections.filter((projection) => projection.status === "building").length;

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <header className="flex flex-col gap-4 border-b border-border pb-6 sm:flex-row sm:items-end sm:justify-between">
        <div className="max-w-3xl">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-primary">
            <GitBranch className="h-4 w-4" aria-hidden="true" />
            Read-model workspace
          </div>
          <h1 className="text-3xl font-bold tracking-tight md:text-4xl">Projections</h1>
          <p className="mt-2 max-w-2xl text-base leading-7 text-muted-foreground">
            Turn event history into queryable current state. Each projection stays updated as new
            events arrive.
          </p>
        </div>
        {hasLoaded &&
          projections.length > 0 &&
          (available.length > 0 ? (
            <Button onClick={() => setShowAdd(true)} className="w-fit shrink-0">
              <Plus className="h-4 w-4" />
              Add read model
            </Button>
          ) : (
            <Badge variant="outline" className="w-fit shrink-0 gap-2 px-3 py-1.5">
              <CheckCircle2 className="h-3.5 w-3.5 text-emerald-500" />
              All templates enabled
            </Badge>
          ))}
      </header>

      {pageError && (
        <LoadError title="Projections could not be updated" message={pageError} onRetry={load} />
      )}

      {isLoading && (
        <div className="grid gap-4 md:grid-cols-2" role="status" aria-label="Loading projections">
          {["first", "second", "third", "fourth"].map((placeholder) => (
            <Card key={placeholder} className="gap-4 p-6">
              <div className="h-9 w-9 animate-pulse rounded-md bg-muted" />
              <div className="h-5 w-40 animate-pulse rounded bg-muted" />
              <div className="h-4 w-full animate-pulse rounded bg-muted" />
              <div className="h-4 w-2/3 animate-pulse rounded bg-muted" />
            </Card>
          ))}
        </div>
      )}

      {!isLoading && hasLoaded && projections.length === 0 && (
        <EmptyProjectionWorkspace templates={available} busy={busy} onEnable={handleEnable} />
      )}

      {!isLoading && hasLoaded && projections.length > 0 && (
        <>
          <section
            aria-label="Projection status"
            className="flex flex-col gap-4 rounded-xl border border-border bg-card px-5 py-4 sm:flex-row sm:items-center sm:justify-between"
          >
            <div>
              <p className="font-semibold">Enabled read models</p>
              <p className="mt-0.5 text-sm text-muted-foreground">
                Monitor backfills and inspect current state.
              </p>
            </div>
            <dl className="flex items-center gap-6 sm:gap-8">
              <div>
                <dt className="text-xs text-muted-foreground">Enabled</dt>
                <dd className="mt-0.5 text-xl font-semibold tabular-nums">{projections.length}</dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Ready</dt>
                <dd className="mt-0.5 text-xl font-semibold tabular-nums text-emerald-500">
                  {readyCount}
                </dd>
              </div>
              <div>
                <dt className="text-xs text-muted-foreground">Building</dt>
                <dd className="mt-0.5 text-xl font-semibold tabular-nums text-amber-500">
                  {buildingCount}
                </dd>
              </div>
            </dl>
          </section>

          <div className="grid gap-4 md:grid-cols-2">
            {projections.map((projection) => (
              <ProjectionCard
                key={projection.name}
                projection={projection}
                isBusy={busy === projection.name}
                onDisable={() => handleDisable(projection.name)}
              />
            ))}
          </div>

          <div className="flex flex-col gap-3 rounded-lg border border-border bg-muted/15 px-4 py-3 text-sm sm:flex-row sm:items-center sm:justify-between">
            <span className="text-muted-foreground">
              Need to rebuild state after projection logic changes?
            </span>
            <Button asChild variant="outline" size="sm" className="w-fit">
              <Link href="/dashboard/tools/replay">
                Open Replay Studio
                <ArrowRight className="h-3.5 w-3.5" />
              </Link>
            </Button>
          </div>
        </>
      )}

      <ProjectionPicker
        open={showAdd}
        templates={available}
        busy={busy}
        onOpenChange={setShowAdd}
        onEnable={handleEnable}
      />
    </div>
  );
}
