"use client";

import {
  Badge,
  BlurFade,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertTriangle,
  GitBranch,
  Layers,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  RefreshCw,
} from "lucide-react";
import { useEffect, useState } from "react";
import { apiClient, type Projection } from "@/lib/api/client";

interface Pipeline {
  id: string;
  name: string;
  description: string;
  status: "running" | "paused" | "error";
  type: "projection" | "stream";
  eventsProcessed: number;
  lastProcessed: string | null;
  createdAt: string;
}

function projectionToPipeline(proj: Projection): Pipeline {
  return {
    id: proj.id,
    name: proj.name,
    description: proj.definition || `Projection v${proj.version}`,
    status: proj.status,
    type: "projection",
    eventsProcessed: 0,
    lastProcessed: proj.updated_at,
    createdAt: proj.created_at,
  };
}

function PipelineCard({
  pipeline,
  onPause,
  onResume,
}: {
  pipeline: Pipeline;
  onPause: () => void;
  onResume: () => void;
}) {
  const [showMenu, setShowMenu] = useState(false);

  const getStatusColor = (status: string) => {
    switch (status) {
      case "running":
        return "bg-green-500";
      case "paused":
        return "bg-yellow-500";
      case "error":
        return "bg-red-500";
      default:
        return "bg-gray-500";
    }
  };

  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const formatTimestamp = (timestamp: string | null) => {
    if (!timestamp) return "Never";
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);

    if (diffSec < 60) return `${diffSec}s ago`;
    if (diffMin < 60) return `${diffMin}m ago`;
    return date.toLocaleString();
  };

  return (
    <Card
      className={cn(
        "transition-all hover:shadow-md",
        pipeline.status === "error" && "border-destructive/50"
      )}
    >
      <CardHeader className="flex flex-row items-start justify-between pb-2">
        <div className="flex items-start gap-3">
          <div
            className={cn(
              "mt-1 flex h-8 w-8 items-center justify-center rounded-lg",
              pipeline.type === "projection"
                ? "bg-blue-500/10 text-blue-600"
                : "bg-purple-500/10 text-purple-600"
            )}
          >
            {pipeline.type === "projection" ? (
              <Layers className="h-4 w-4" />
            ) : (
              <GitBranch className="h-4 w-4" />
            )}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <CardTitle className="text-base">{pipeline.name}</CardTitle>
              <span
                className={cn(
                  "h-2 w-2 rounded-full",
                  getStatusColor(pipeline.status),
                  pipeline.status === "running" && "animate-pulse"
                )}
              />
            </div>
            <CardDescription className="mt-1">{pipeline.description}</CardDescription>
          </div>
        </div>

        <div className="relative">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => setShowMenu(!showMenu)}
          >
            <MoreHorizontal className="h-4 w-4" />
          </Button>

          {showMenu && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowMenu(false)} />
              <div className="absolute right-0 top-full z-50 mt-1 w-36 rounded-lg border border-border bg-popover p-1 shadow-lg">
                {pipeline.status === "running" ? (
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      onPause();
                    }}
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted"
                  >
                    <Pause className="h-4 w-4" />
                    Pause
                  </button>
                ) : (
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      onResume();
                    }}
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted"
                  >
                    <Play className="h-4 w-4" />
                    Resume
                  </button>
                )}
                <button className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted">
                  <RefreshCw className="h-4 w-4" />
                  Reset
                </button>
              </div>
            </>
          )}
        </div>
      </CardHeader>

      <CardContent>
        {pipeline.status === "error" && (
          <div className="mb-4 flex items-center gap-2 rounded-lg bg-destructive/10 px-3 py-2 text-sm text-destructive">
            <AlertTriangle className="h-4 w-4" />
            Pipeline encountered an error. Check logs for details.
          </div>
        )}

        <div className="grid grid-cols-3 gap-4 text-sm">
          <div>
            <p className="text-muted-foreground">Status</p>
            <Badge
              variant={
                pipeline.status === "running"
                  ? "default"
                  : pipeline.status === "paused"
                    ? "secondary"
                    : "destructive"
              }
              className="mt-1 capitalize"
            >
              {pipeline.status}
            </Badge>
          </div>
          <div>
            <p className="text-muted-foreground">Processed</p>
            <p className="mt-1 font-medium">{formatNumber(pipeline.eventsProcessed)} events</p>
          </div>
          <div>
            <p className="text-muted-foreground">Last Active</p>
            <p className="mt-1 font-medium">{formatTimestamp(pipeline.lastProcessed)}</p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function PipelinesPage() {
  const [pipelines, setPipelines] = useState<Pipeline[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    async function fetchPipelines() {
      try {
        const response = await apiClient.listProjections();
        if (response.data) {
          setPipelines(response.data.map(projectionToPipeline));
        }
      } catch (error) {
        console.error("Failed to fetch pipelines:", error);
      } finally {
        setIsLoading(false);
      }
    }
    fetchPipelines();
  }, []);

  const refreshPipelines = async () => {
    try {
      const response = await apiClient.listProjections();
      if (response.data) {
        setPipelines(response.data.map(projectionToPipeline));
      }
    } catch (error) {
      console.error("Failed to refresh pipelines:", error);
    }
  };

  const handlePause = async (name: string) => {
    // Find the pipeline name from the id
    const pipeline = pipelines.find((p) => p.id === name);
    const projectionName = pipeline?.name || name;
    try {
      const response = await apiClient.pauseProjection(projectionName);
      if (response.error) {
        console.error("Failed to pause projection:", response.error);
        return;
      }
      await refreshPipelines();
    } catch (error) {
      console.error("Failed to pause projection:", error);
    }
  };

  const handleResume = async (name: string) => {
    const pipeline = pipelines.find((p) => p.id === name);
    const projectionName = pipeline?.name || name;
    try {
      const response = await apiClient.startProjection(projectionName);
      if (response.error) {
        console.error("Failed to start projection:", response.error);
        return;
      }
      await refreshPipelines();
    } catch (error) {
      console.error("Failed to start projection:", error);
    }
  };

  const runningCount = pipelines.filter((p) => p.status === "running").length;
  const pausedCount = pipelines.filter((p) => p.status === "paused").length;
  const errorCount = pipelines.filter((p) => p.status === "error").length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Pipelines</h1>
            <p className="mt-1 text-muted-foreground">
              Manage projections and stream processing pipelines
            </p>
          </div>
          <Button onClick={() => window.open("https://docs.all-source.xyz/pipelines", "_blank")}>
            <Plus className="mr-2 h-4 w-4" />
            Create Pipeline
          </Button>
        </div>
      </BlurFade>

      {/* Status Overview */}
      <BlurFade delay={0.2} inView>
        <div className="grid gap-4 sm:grid-cols-4">
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                <GitBranch className="h-5 w-5" />
              </div>
              <div>
                <p className="text-2xl font-bold">{pipelines.length}</p>
                <p className="text-sm text-muted-foreground">Total Pipelines</p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-green-500/10">
                <Play className="h-5 w-5 text-green-600" />
              </div>
              <div>
                <p className="text-2xl font-bold">{runningCount}</p>
                <p className="text-sm text-muted-foreground">Running</p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-yellow-500/10">
                <Pause className="h-5 w-5 text-yellow-600" />
              </div>
              <div>
                <p className="text-2xl font-bold">{pausedCount}</p>
                <p className="text-sm text-muted-foreground">Paused</p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-red-500/10">
                <AlertTriangle className="h-5 w-5 text-red-600" />
              </div>
              <div>
                <p className="text-2xl font-bold">{errorCount}</p>
                <p className="text-sm text-muted-foreground">Errors</p>
              </div>
            </CardContent>
          </Card>
        </div>
      </BlurFade>

      {/* Pipelines Grid */}
      <BlurFade delay={0.3} inView>
        {isLoading ? (
          <div className="grid gap-4 md:grid-cols-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <Card key={i}>
                <CardContent className="p-6 space-y-4">
                  <div className="h-6 w-40 animate-pulse rounded bg-muted" />
                  <div className="h-4 w-full animate-pulse rounded bg-muted" />
                  <div className="grid grid-cols-3 gap-4">
                    <div className="h-8 animate-pulse rounded bg-muted" />
                    <div className="h-8 animate-pulse rounded bg-muted" />
                    <div className="h-8 animate-pulse rounded bg-muted" />
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {pipelines.map((pipeline) => (
              <PipelineCard
                key={pipeline.id}
                pipeline={pipeline}
                onPause={() => handlePause(pipeline.id)}
                onResume={() => handleResume(pipeline.id)}
              />
            ))}
          </div>
        )}
      </BlurFade>

      {/* Empty state */}
      {!isLoading && pipelines.length === 0 && (
        <BlurFade delay={0.3} inView>
          <Card className="p-12 text-center">
            <GitBranch className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50" />
            <h3 className="text-lg font-medium">No pipelines yet</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Create your first pipeline to start processing events
            </p>
            <Button
              className="mt-4"
              onClick={() => window.open("https://docs.all-source.xyz/pipelines", "_blank")}
            >
              <Plus className="mr-2 h-4 w-4" />
              Create Pipeline
            </Button>
          </Card>
        </BlurFade>
      )}
    </div>
  );
}
