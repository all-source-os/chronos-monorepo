"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertCircle,
  ArrowRight,
  Check,
  CheckCircle2,
  Clock,
  Database,
  Layers3,
  Loader2,
  Play,
  RotateCcw,
  ShieldCheck,
  Square,
  Trash2,
  XCircle,
} from "lucide-react";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { LoadError } from "@/components/dashboard/load-error";
import { useReplays } from "@/hooks/use-replay";
import {
  apiClient,
  type Projection,
  type ReplayProgress,
  type ReplayStatus,
} from "@/lib/api/client";

const STATUS_CONFIG: Record<
  ReplayStatus,
  {
    label: string;
    variant: "default" | "secondary" | "destructive" | "outline";
    icon: typeof Clock;
  }
> = {
  pending: { label: "Pending", variant: "secondary", icon: Clock },
  running: { label: "Rebuilding", variant: "default", icon: Loader2 },
  completed: { label: "Ready", variant: "outline", icon: CheckCircle2 },
  failed: { label: "Failed safely", variant: "destructive", icon: XCircle },
  cancelled: { label: "Cancelled", variant: "secondary", icon: Square },
  unknown: { label: "Status unavailable", variant: "secondary", icon: AlertCircle },
};

function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

function formatTimestamp(ts: string): string {
  const date = new Date(ts);
  return Number.isNaN(date.getTime()) ? ts : date.toLocaleString();
}

export default function ReplayPage() {
  const { replays, isLoading, error, startReplay, cancelReplay, deleteReplay, refresh } =
    useReplays();
  const [projections, setProjections] = useState<Projection[]>([]);
  const [projectionsLoading, setProjectionsLoading] = useState(true);
  const [projectionError, setProjectionError] = useState<string | null>(null);
  const [selectedProjection, setSelectedProjection] = useState("");
  const [isStarting, setIsStarting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    apiClient.listProjections().then((response) => {
      if (!active) return;
      setProjectionsLoading(false);
      if (response.error) {
        setProjectionError(response.error.message);
        return;
      }

      const available = response.data?.projections ?? [];
      setProjections(available);
      setSelectedProjection(
        (current) => current || available.find((item) => item.status === "ready")?.name || ""
      );
    });

    return () => {
      active = false;
    };
  }, []);

  const target = useMemo(
    () => projections.find((projection) => projection.name === selectedProjection),
    [projections, selectedProjection]
  );
  const targetBusy = target?.status === "building";

  const handleStart = async () => {
    if (!selectedProjection) return;
    setFormError(null);
    setIsStarting(true);

    try {
      await startReplay({ projection_name: selectedProjection });
    } catch (err) {
      setFormError(err instanceof Error ? err.message : "Replay could not start");
    } finally {
      setIsStarting(false);
    }
  };

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      <header className="flex flex-col gap-4 border-b border-border pb-6 lg:flex-row lg:items-end lg:justify-between">
        <div className="max-w-3xl">
          <div className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-primary">
            <RotateCcw className="h-4 w-4" />
            Replay Studio
          </div>
          <h1 className="text-3xl font-bold tracking-tight md:text-4xl">Rebuild from truth.</h1>
          <p className="mt-2 max-w-2xl text-base leading-7 text-muted-foreground">
            Fold your immutable event history into one read-model. Current state stays live until a
            successful replacement is ready.
          </p>
        </div>
        <Badge variant="outline" className="w-fit gap-2 px-3 py-1.5 text-xs">
          <ShieldCheck className="h-3.5 w-3.5 text-emerald-500" />
          Tenant-scoped · source preserved
        </Badge>
      </header>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.6fr)_minmax(280px,0.7fr)]">
        <Card className="overflow-hidden border-border/80">
          <CardHeader className="border-b border-border bg-muted/20">
            <CardTitle>Build plan</CardTitle>
            <CardDescription>
              Choose one enabled projection. All tenant events replay in timestamp order.
            </CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="grid items-stretch md:grid-cols-[1fr_auto_1fr_auto_1fr]">
              <PlanStep
                icon={Database}
                eyebrow="Source"
                title="Event history"
                detail="Immutable · current tenant"
              />
              <PlanArrow />
              <div className="space-y-3 border-y border-border p-5 md:border-x md:border-y-0">
                <div className="flex items-center gap-3">
                  <span className="flex h-9 w-9 items-center justify-center rounded-md bg-primary/10 text-primary">
                    <Layers3 className="h-4 w-4" />
                  </span>
                  <div>
                    <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
                      Target
                    </p>
                    <p className="font-semibold">Read-model</p>
                  </div>
                </div>
                <label htmlFor="projection" className="sr-only">
                  Target projection
                </label>
                <select
                  id="projection"
                  value={selectedProjection}
                  onChange={(event) => setSelectedProjection(event.target.value)}
                  className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm font-medium outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <option value="">Choose projection</option>
                  {projections.map((projection) => (
                    <option
                      key={projection.name}
                      value={projection.name}
                      disabled={projection.status === "building"}
                    >
                      {projection.title || projection.name}
                      {projection.status === "building" ? " · building" : ""}
                    </option>
                  ))}
                </select>
                {target?.description && (
                  <p className="text-xs leading-5 text-muted-foreground">{target.description}</p>
                )}
              </div>
              <PlanArrow />
              <PlanStep
                icon={Check}
                eyebrow="Commit"
                title="Atomic swap"
                detail="Only after full success"
              />
            </div>

            <div className="flex flex-col gap-3 border-t border-border bg-muted/10 p-5 sm:flex-row sm:items-center sm:justify-between">
              <div className="min-h-5 text-sm">
                {projectionsLoading ? (
                  <span className="text-muted-foreground">Loading enabled projections…</span>
                ) : projectionError ? (
                  <span className="text-destructive">{projectionError}</span>
                ) : projections.length === 0 ? (
                  <span className="text-muted-foreground">
                    Enable a projection in{" "}
                    <Link
                      href="/dashboard/pipelines"
                      className="font-medium text-primary underline underline-offset-4"
                    >
                      Read Models
                    </Link>{" "}
                    first.
                  </span>
                ) : target ? (
                  <span className="text-muted-foreground">
                    Ready to rebuild{" "}
                    <strong className="text-foreground">{target.title || target.name}</strong>.
                  </span>
                ) : (
                  <span className="text-muted-foreground">Select a target to continue.</span>
                )}
              </div>
              <Button
                type="button"
                onClick={handleStart}
                disabled={projectionsLoading || !target || targetBusy || isStarting}
                className="shrink-0"
              >
                {isStarting ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  <Play className="mr-2 h-4 w-4" />
                )}
                Rebuild projection
              </Button>
            </div>
            {formError && (
              <div
                className="flex items-center gap-2 border-t border-destructive/30 bg-destructive/10 px-5 py-3 text-sm text-destructive"
                role="alert"
              >
                <AlertCircle className="h-4 w-4 shrink-0" />
                {formError}
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="border-emerald-500/20 bg-emerald-500/[0.03]">
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <ShieldCheck className="h-4 w-4 text-emerald-500" /> Safety contract
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-4 text-sm">
              <SafetyItem
                title="No downtime"
                detail="Existing read-model serves traffic during fold."
              />
              <SafetyItem title="No source mutation" detail="Stored events remain unchanged." />
              <SafetyItem
                title="Failure is isolated"
                detail="Partial output is discarded, never published."
              />
              <SafetyItem
                title="One tenant, one target"
                detail="Replay cannot reach another workspace."
              />
            </ul>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader className="grid grid-cols-[1fr_auto] items-start">
          <div>
            <CardTitle>Replay runs</CardTitle>
            <CardDescription className="mt-1">
              Progress and outcomes for this tenant.
            </CardDescription>
          </div>
          {!isLoading && (
            <Badge variant="secondary" className="w-fit justify-self-end">
              {replays.length} {replays.length === 1 ? "run" : "runs"}
            </Badge>
          )}
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div
              className="flex items-center justify-center py-12"
              role="status"
              aria-label="Loading replay runs"
            >
              <Loader2 className="h-6 w-6 animate-spin text-primary" />
            </div>
          ) : error ? (
            <LoadError title="Replay runs could not be loaded" message={error} onRetry={refresh} />
          ) : replays.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border py-10 text-center">
              <RotateCcw className="mx-auto h-5 w-5 text-muted-foreground" />
              <p className="mt-3 text-sm font-medium">No replay runs yet</p>
              <p className="mt-1 text-sm text-muted-foreground">
                Your first safe rebuild will appear here.
              </p>
            </div>
          ) : (
            <div className="divide-y divide-border rounded-lg border border-border">
              {replays.map((replay) => (
                <ReplayRow
                  key={replay.replay_id}
                  replay={replay}
                  onCancel={cancelReplay}
                  onDelete={deleteReplay}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function PlanStep({
  icon: Icon,
  eyebrow,
  title,
  detail,
}: {
  icon: typeof Database;
  eyebrow: string;
  title: string;
  detail: string;
}) {
  return (
    <div className="flex min-h-36 flex-col justify-between gap-6 p-5">
      <span className="flex h-9 w-9 items-center justify-center rounded-md bg-primary/10 text-primary">
        <Icon className="h-4 w-4" />
      </span>
      <div>
        <p className="text-[11px] font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          {eyebrow}
        </p>
        <p className="mt-1 font-semibold">{title}</p>
        <p className="mt-1 text-xs text-muted-foreground">{detail}</p>
      </div>
    </div>
  );
}

function PlanArrow() {
  return (
    <div className="hidden items-center text-muted-foreground/50 md:flex">
      <ArrowRight className="h-4 w-4" />
    </div>
  );
}

function SafetyItem({ title, detail }: { title: string; detail: string }) {
  return (
    <li className="flex gap-3">
      <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
      <span>
        <strong className="block font-medium text-foreground">{title}</strong>
        <span className="mt-0.5 block leading-5 text-muted-foreground">{detail}</span>
      </span>
    </li>
  );
}

function ReplayRow({
  replay,
  onCancel,
  onDelete,
}: {
  replay: ReplayProgress;
  onCancel: (id: string) => Promise<unknown>;
  onDelete: (id: string) => Promise<unknown>;
}) {
  const config = STATUS_CONFIG[replay.status] ?? STATUS_CONFIG.unknown;
  const StatusIcon = config.icon;
  const running = replay.status === "running" || replay.status === "pending";
  const done = ["completed", "failed", "cancelled", "unknown"].includes(replay.status);
  const elapsed =
    (replay.completed_at ? new Date(replay.completed_at) : new Date(replay.updated_at)).getTime() -
    new Date(replay.started_at).getTime();
  const progress = Number.isFinite(replay.progress_percentage)
    ? Math.min(Math.max(replay.progress_percentage, 0), 100)
    : 0;

  return (
    <article className="p-4 sm:p-5">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex min-w-0 items-center gap-3">
          <span
            className={cn(
              "flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-muted",
              replay.status === "completed" && "bg-emerald-500/10 text-emerald-500",
              replay.status === "failed" && "bg-destructive/10 text-destructive"
            )}
          >
            <StatusIcon className={cn("h-4 w-4", running && "animate-spin")} />
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <p className="truncate font-medium">
                {replay.projection_name || "Projection replay"}
              </p>
              <Badge variant={config.variant}>{config.label}</Badge>
            </div>
            <p className="mt-1 font-mono text-xs text-muted-foreground">
              {replay.replay_id.slice(0, 10)} · {formatTimestamp(replay.started_at)}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2 self-end sm:self-auto">
          <span className="mr-1 text-xs tabular-nums text-muted-foreground">
            {formatDuration(elapsed)}
          </span>
          {running && (
            <Button
              variant="outline"
              size="sm"
              type="button"
              onClick={() => void onCancel(replay.replay_id).catch(() => undefined)}
            >
              <Square className="mr-1.5 h-3 w-3" />
              Cancel
            </Button>
          )}
          {done && (
            <Button
              variant="ghost"
              size="sm"
              type="button"
              onClick={() => void onDelete(replay.replay_id).catch(() => undefined)}
              aria-label={`Remove replay ${replay.replay_id}`}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </div>

      <div
        className="mt-4 h-1.5 overflow-hidden rounded-full bg-muted"
        role="progressbar"
        aria-label="Replay progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(progress)}
      >
        <div
          className={cn(
            "h-full rounded-full bg-primary transition-[width] duration-300",
            replay.status === "completed" && "bg-emerald-500",
            replay.status === "failed" && "bg-destructive"
          )}
          style={{ width: `${progress}%` }}
        />
      </div>
      <div className="mt-2 flex justify-between text-xs tabular-nums text-muted-foreground">
        <span>{replay.processed_events.toLocaleString()} events folded</span>
        <span>
          {replay.total_events > 0 ? `${progress.toFixed(0)}%` : running ? "Scanning history" : "—"}
        </span>
      </div>
      {replay.error_message && (
        <div
          className="mt-3 flex items-start gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive"
          role="alert"
        >
          <AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
          {replay.error_message}
        </div>
      )}
    </article>
  );
}
