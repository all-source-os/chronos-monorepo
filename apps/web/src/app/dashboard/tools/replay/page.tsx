"use client";

import { Badge, BlurFade, Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import {
  AlertCircle,
  CheckCircle2,
  Clock,
  Loader2,
  Play,
  RotateCcw,
  Square,
  Trash2,
  XCircle,
} from "lucide-react";
import { useState } from "react";

import { useReplays } from "@/hooks/use-replay";
import type { ReplayProgress, ReplayStatus } from "@/lib/api/client";

const STATUS_CONFIG: Record<
  ReplayStatus,
  {
    label: string;
    variant: "default" | "secondary" | "destructive" | "outline";
    icon: typeof Clock;
  }
> = {
  Pending: { label: "Pending", variant: "secondary", icon: Clock },
  Running: { label: "Running", variant: "default", icon: Loader2 },
  Completed: { label: "Completed", variant: "outline", icon: CheckCircle2 },
  Failed: { label: "Failed", variant: "destructive", icon: XCircle },
  Cancelled: { label: "Cancelled", variant: "secondary", icon: Square },
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

function formatTimestamp(ts: string): string {
  try {
    return new Date(ts).toLocaleString();
  } catch {
    return ts;
  }
}

export default function ReplayPage() {
  const { replays, isLoading, error, startReplay, cancelReplay, deleteReplay } = useReplays();

  const [fromTimestamp, setFromTimestamp] = useState("");
  const [fromTime, setFromTime] = useState("");
  const [toTimestamp, setToTimestamp] = useState("");
  const [toTime, setToTime] = useState("");
  const [eventType, setEventType] = useState("");
  const [entityId, setEntityId] = useState("");
  const [projectionName, setProjectionName] = useState("");
  const [isStarting, setIsStarting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const handleStart = async () => {
    setFormError(null);
    setIsStarting(true);

    try {
      const params: Record<string, string> = {};
      if (fromTimestamp) {
        params.from_timestamp = new Date(`${fromTimestamp}T${fromTime || "00:00"}`).toISOString();
      }
      if (toTimestamp) {
        params.to_timestamp = new Date(`${toTimestamp}T${toTime || "23:59"}`).toISOString();
      }
      if (eventType.trim()) params.event_type = eventType.trim();
      if (entityId.trim()) params.entity_id = entityId.trim();
      if (projectionName.trim()) params.projection_name = projectionName.trim();

      await startReplay(params);

      setFromTimestamp("");
      setFromTime("");
      setToTimestamp("");
      setToTime("");
      setEventType("");
      setEntityId("");
      setProjectionName("");
    } catch (err) {
      setFormError(err instanceof Error ? err.message : "Failed to start replay");
    } finally {
      setIsStarting(false);
    }
  };

  const handleCancel = async (replayId: string) => {
    try {
      await cancelReplay(replayId);
    } catch {
      // Swallow — UI will refresh via SWR
    }
  };

  const handleDelete = async (replayId: string) => {
    try {
      await deleteReplay(replayId);
    } catch {
      // Swallow
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div>
          <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Event Replay</h1>
          <p className="mt-1 text-muted-foreground">
            Replay events from a specific point in time to rebuild projections or re-process events
          </p>
        </div>
      </BlurFade>

      {/* Start Replay Form */}
      <BlurFade delay={0.15} inView>
        <Card className="max-w-3xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <RotateCcw className="h-5 w-5" />
              Start New Replay
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {/* Time Range */}
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <label htmlFor="from-date" className="text-sm font-medium">
                  From Date
                </label>
                <div className="flex gap-2">
                  <input
                    id="from-date"
                    type="date"
                    value={fromTimestamp}
                    onChange={(e) => setFromTimestamp(e.target.value)}
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  />
                  <input
                    type="time"
                    value={fromTime}
                    onChange={(e) => setFromTime(e.target.value)}
                    aria-label="From time"
                    className="flex h-9 w-28 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>
              </div>
              <div className="space-y-2">
                <label htmlFor="to-date" className="text-sm font-medium">
                  To Date
                </label>
                <div className="flex gap-2">
                  <input
                    id="to-date"
                    type="date"
                    value={toTimestamp}
                    onChange={(e) => setToTimestamp(e.target.value)}
                    className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  />
                  <input
                    type="time"
                    value={toTime}
                    onChange={(e) => setToTime(e.target.value)}
                    aria-label="To time"
                    className="flex h-9 w-28 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>
              </div>
            </div>

            {/* Filters */}
            <div className="grid gap-4 sm:grid-cols-3">
              <div className="space-y-2">
                <label htmlFor="event-type" className="text-sm font-medium">
                  Event Type Filter
                </label>
                <input
                  id="event-type"
                  type="text"
                  placeholder="e.g. user.created"
                  value={eventType}
                  onChange={(e) => setEventType(e.target.value)}
                  className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="entity-id" className="text-sm font-medium">
                  Entity ID Filter
                </label>
                <input
                  id="entity-id"
                  type="text"
                  placeholder="e.g. user-123"
                  value={entityId}
                  onChange={(e) => setEntityId(e.target.value)}
                  className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="projection" className="text-sm font-medium">
                  Target Projection
                </label>
                <input
                  id="projection"
                  type="text"
                  placeholder="All projections"
                  value={projectionName}
                  onChange={(e) => setProjectionName(e.target.value)}
                  className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>
            </div>

            {formError && (
              <div className="flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-sm text-destructive">
                <AlertCircle className="h-4 w-4 shrink-0" />
                {formError}
              </div>
            )}

            <Button type="button" onClick={handleStart} disabled={isStarting}>
              {isStarting ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Play className="mr-2 h-4 w-4" />
              )}
              Start Replay
            </Button>
          </CardContent>
        </Card>
      </BlurFade>

      {/* Replay History */}
      <BlurFade delay={0.2} inView>
        <Card className="max-w-3xl">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Clock className="h-5 w-5" />
              Replay History
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <div className="flex items-center justify-center py-12">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent" />
              </div>
            ) : error ? (
              <div className="py-12 text-center text-sm text-destructive">{error}</div>
            ) : replays.length === 0 ? (
              <div className="py-12 text-center text-sm text-muted-foreground">
                No replays yet. Start a new replay above.
              </div>
            ) : (
              <div className="space-y-3">
                {replays.map((replay: ReplayProgress) => (
                  <ReplayRow
                    key={replay.replay_id}
                    replay={replay}
                    onCancel={handleCancel}
                    onDelete={handleDelete}
                  />
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </BlurFade>
    </div>
  );
}

function ReplayRow({
  replay,
  onCancel,
  onDelete,
}: {
  replay: ReplayProgress;
  onCancel: (id: string) => void;
  onDelete: (id: string) => void;
}) {
  const config = STATUS_CONFIG[replay.status];
  const StatusIcon = config.icon;
  const isRunning = replay.status === "Running" || replay.status === "Pending";
  const isDone =
    replay.status === "Completed" || replay.status === "Failed" || replay.status === "Cancelled";

  const elapsed = replay.completed_at
    ? new Date(replay.completed_at).getTime() - new Date(replay.started_at).getTime()
    : new Date(replay.updated_at).getTime() - new Date(replay.started_at).getTime();

  return (
    <div className="rounded-lg border border-border p-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <StatusIcon
            className={`h-5 w-5 shrink-0 ${
              replay.status === "Running" ? "animate-spin text-primary" : ""
            } ${replay.status === "Completed" ? "text-green-500" : ""} ${
              replay.status === "Failed" ? "text-destructive" : ""
            }`}
          />
          <div>
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs text-muted-foreground">
                {replay.replay_id.slice(0, 8)}
              </span>
              <Badge variant={config.variant}>{config.label}</Badge>
            </div>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Started {formatTimestamp(replay.started_at)}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {isRunning && (
            <Button
              variant="outline"
              size="sm"
              type="button"
              onClick={() => onCancel(replay.replay_id)}
            >
              <Square className="mr-1 h-3 w-3" />
              Cancel
            </Button>
          )}
          {isDone && (
            <Button
              variant="ghost"
              size="sm"
              type="button"
              onClick={() => onDelete(replay.replay_id)}
            >
              <Trash2 className="mr-1 h-3 w-3" />
              Remove
            </Button>
          )}
        </div>
      </div>

      {/* Progress bar */}
      {replay.total_events > 0 && (
        <div className="mt-3 space-y-1.5">
          <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
            <div
              className={`h-full rounded-full transition-all duration-500 ${
                replay.status === "Failed"
                  ? "bg-destructive"
                  : replay.status === "Completed"
                    ? "bg-green-500"
                    : "bg-primary"
              }`}
              style={{ width: `${Math.min(replay.progress_percentage, 100)}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>
              {replay.processed_events.toLocaleString()} / {replay.total_events.toLocaleString()}{" "}
              events
              {replay.failed_events > 0 && (
                <span className="text-destructive"> ({replay.failed_events} failed)</span>
              )}
            </span>
            <span className="flex items-center gap-3">
              {replay.events_per_second > 0 && (
                <span>{Math.round(replay.events_per_second).toLocaleString()} events/s</span>
              )}
              <span>{formatDuration(elapsed)}</span>
              <span>{replay.progress_percentage.toFixed(1)}%</span>
            </span>
          </div>
        </div>
      )}

      {replay.error_message && (
        <div className="mt-2 flex items-start gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
          {replay.error_message}
        </div>
      )}
    </div>
  );
}
