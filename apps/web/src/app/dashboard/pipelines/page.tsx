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
  CheckCircle2,
  ChevronDown,
  GitBranch,
  Layers,
  Loader2,
  MoreHorizontal,
  Plus,
  Trash2,
} from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { ProjectionStateView } from "@/components/dashboard/projection-state-view";
import { useDashboardStats } from "@/hooks/use-dashboard-stats";
import { apiClient, type Projection, type ProjectionTemplate } from "@/lib/api/client";

function statusColor(status: Projection["status"]) {
  return status === "ready" ? "bg-green-500" : "bg-yellow-500";
}

// entity_table templates that fold ONE ETS row per entity_id (rather than a
// tenant-wide summary) need an entity_id filter to read state. active-entities
// is entity_table but tenant-wide, so it does NOT.
function isPerEntityTemplate(p: Projection): boolean {
  return p.kind === "entity_table" && p.name !== "active-entities";
}

function ProjectionCard({
  projection,
  onDisable,
}: {
  projection: Projection;
  onDisable: () => void;
}) {
  const [showMenu, setShowMenu] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const building = projection.status === "building";

  return (
    <Card className="transition-all hover:shadow-md">
      <CardHeader className="flex flex-row items-start justify-between pb-2">
        <div className="flex items-start gap-3">
          <div className="mt-1 flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/10 text-blue-600">
            <Layers className="h-4 w-4" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <CardTitle className="text-base">{projection.title || projection.name}</CardTitle>
              <span
                className={cn(
                  "h-2 w-2 rounded-full",
                  statusColor(projection.status),
                  building && "animate-pulse"
                )}
              />
            </div>
            <CardDescription className="mt-1">
              {projection.description || projection.name}
            </CardDescription>
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
                <button
                  type="button"
                  onClick={() => {
                    setShowMenu(false);
                    onDisable();
                  }}
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-destructive hover:bg-muted"
                >
                  <Trash2 className="h-4 w-4" />
                  Disable
                </button>
              </div>
            </>
          )}
        </div>
      </CardHeader>

      <CardContent>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <p className="text-muted-foreground">Status</p>
            <Badge variant={building ? "secondary" : "default"} className="mt-1 capitalize">
              {building ? (
                <>
                  <Loader2 className="mr-1 h-3 w-3 animate-spin" />
                  Building
                </>
              ) : (
                <>
                  <CheckCircle2 className="mr-1 h-3 w-3" />
                  Ready
                </>
              )}
            </Badge>
          </div>
          <div>
            <p className="text-muted-foreground">Template</p>
            <p className="mt-1 font-mono text-xs">{projection.name}</p>
          </div>
        </div>

        {/* Ready cards expand to show the folded read-model. Building cards are
            not expandable until the backfill finishes. */}
        {building ? (
          <p className="mt-4 text-xs text-muted-foreground">
            Folding your event history… the read-model will be viewable once ready.
          </p>
        ) : (
          <>
            <button
              type="button"
              onClick={() => setExpanded((v) => !v)}
              className="mt-4 flex items-center gap-1 text-xs font-medium text-primary hover:underline"
            >
              <ChevronDown
                className={cn("h-3.5 w-3.5 transition-transform", expanded && "rotate-180")}
              />
              {expanded ? "Hide read-model" : "View read-model"}
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

export default function PipelinesPage() {
  const [projections, setProjections] = useState<Projection[]>([]);
  const [templates, setTemplates] = useState<ProjectionTemplate[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const { stats } = useDashboardStats();
  // Reuse the dashboard's REAL tenant-scoped event total (summed from
  // /api/event-types) rather than recomputing — 0 means an honest empty tenant.
  const hasNoEvents = stats.events.total === 0;

  const refresh = useCallback(async () => {
    const response = await apiClient.listProjections();
    if (response.data) {
      setProjections(response.data.projections ?? []);
    }
  }, []);

  useEffect(() => {
    async function load() {
      try {
        const [list, catalog] = await Promise.all([
          apiClient.listProjections(),
          apiClient.listProjectionTemplates(),
        ]);
        if (list.data) setProjections(list.data.projections ?? []);
        if (catalog.data) setTemplates(catalog.data.templates ?? []);
      } catch (error) {
        console.error("Failed to load projections:", error);
      } finally {
        setIsLoading(false);
      }
    }
    load();
  }, []);

  const enabledNames = new Set(projections.map((p) => p.name));
  const available = templates.filter((t) => !enabledNames.has(t.name));

  const handleEnable = async (template: string) => {
    setShowAdd(false);
    setBusy(template);
    try {
      const response = await apiClient.enableProjection(template);
      if (response.error) {
        console.error("Failed to enable projection:", response.error);
        return;
      }
      await refresh();
    } catch (error) {
      console.error("Failed to enable projection:", error);
    } finally {
      setBusy(null);
    }
  };

  const handleDisable = async (name: string) => {
    setBusy(name);
    try {
      const response = await apiClient.disableProjection(name);
      if (response.error) {
        console.error("Failed to disable projection:", response.error);
        return;
      }
      await refresh();
    } catch (error) {
      console.error("Failed to disable projection:", error);
    } finally {
      setBusy(null);
    }
  };

  const readyCount = projections.filter((p) => p.status === "ready").length;
  const buildingCount = projections.filter((p) => p.status === "building").length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Projections</h1>
            <p className="mt-1 text-muted-foreground">
              Enable read-models built from your event stream
            </p>
          </div>
          <div className="relative">
            <Button onClick={() => setShowAdd(!showAdd)} disabled={available.length === 0}>
              <Plus className="mr-2 h-4 w-4" />
              Enable Projection
            </Button>
            {showAdd && available.length > 0 && (
              <>
                <div className="fixed inset-0 z-40" onClick={() => setShowAdd(false)} />
                <div className="absolute right-0 top-full z-50 mt-1 w-72 rounded-lg border border-border bg-popover p-1 shadow-lg">
                  {available.map((t) => (
                    <button
                      key={t.name}
                      type="button"
                      onClick={() => handleEnable(t.name)}
                      className="flex w-full flex-col items-start gap-0.5 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                    >
                      <span className="font-medium">{t.title}</span>
                      <span className="text-xs text-muted-foreground">{t.description}</span>
                    </button>
                  ))}
                </div>
              </>
            )}
          </div>
        </div>
      </BlurFade>

      {/* Zero-events hint — projections build from the event stream, so they
          stay empty until events arrive. Be honest rather than letting a user
          enable a projection and stare at a meaningless "Ready / 0". */}
      {!isLoading && hasNoEvents && (
        <BlurFade delay={0.15} inView>
          <Card className="border-yellow-500/30 bg-yellow-500/5">
            <CardContent className="flex items-start gap-3 p-4">
              <GitBranch className="mt-0.5 h-5 w-5 shrink-0 text-yellow-600" />
              <div className="text-sm">
                <p className="font-medium">You have no events yet</p>
                <p className="mt-0.5 text-muted-foreground">
                  Projections build a read-model from your event stream — they will stay empty until
                  events arrive.{" "}
                  <Link href="/dashboard/events" className="text-primary hover:underline">
                    Ingest your first events
                  </Link>{" "}
                  to see them populate.
                </p>
              </div>
            </CardContent>
          </Card>
        </BlurFade>
      )}

      {/* Status Overview */}
      <BlurFade delay={0.2} inView>
        <div className="grid gap-4 sm:grid-cols-3">
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                <Layers className="h-5 w-5" />
              </div>
              <div>
                <p className="text-2xl font-bold">{projections.length}</p>
                <p className="text-sm text-muted-foreground">Enabled</p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-green-500/10">
                <CheckCircle2 className="h-5 w-5 text-green-600" />
              </div>
              <div>
                <p className="text-2xl font-bold">{readyCount}</p>
                <p className="text-sm text-muted-foreground">Ready</p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-yellow-500/10">
                <Loader2 className="h-5 w-5 text-yellow-600" />
              </div>
              <div>
                <p className="text-2xl font-bold">{buildingCount}</p>
                <p className="text-sm text-muted-foreground">Building</p>
              </div>
            </CardContent>
          </Card>
        </div>
      </BlurFade>

      {/* Projections Grid */}
      <BlurFade delay={0.3} inView>
        {isLoading ? (
          <div className="grid gap-4 md:grid-cols-2">
            {Array.from({ length: 2 }).map((_, i) => (
              <Card key={i}>
                <CardContent className="p-6 space-y-4">
                  <div className="h-6 w-40 animate-pulse rounded bg-muted" />
                  <div className="h-4 w-full animate-pulse rounded bg-muted" />
                  <div className="grid grid-cols-2 gap-4">
                    <div className="h-8 animate-pulse rounded bg-muted" />
                    <div className="h-8 animate-pulse rounded bg-muted" />
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {projections.map((projection) => (
              <ProjectionCard
                key={projection.name}
                projection={
                  busy === projection.name ? { ...projection, status: "building" } : projection
                }
                onDisable={() => handleDisable(projection.name)}
              />
            ))}
          </div>
        )}
      </BlurFade>

      {/* Empty state */}
      {!isLoading && projections.length === 0 && (
        <BlurFade delay={0.3} inView>
          <Card className="p-12 text-center">
            <GitBranch className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50" />
            <h3 className="text-lg font-medium">No projections enabled</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              Enable a projection template to build a read-model from your events
            </p>
            {available.length > 0 && (
              <Button className="mt-4" onClick={() => setShowAdd(true)}>
                <Plus className="mr-2 h-4 w-4" />
                Enable Projection
              </Button>
            )}
          </Card>
        </BlurFade>
      )}
    </div>
  );
}
