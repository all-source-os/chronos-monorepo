"use client";

import { Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Activity,
  Database,
  GitBranch,
  RefreshCw,
  TrendingDown,
  TrendingUp,
  Zap,
} from "lucide-react";
import { useDashboardStats } from "@/hooks/use-dashboard-stats";

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ElementType;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  description?: string;
  // Marks a process-global, all-tenant metric (e.g. p99 latency) so it isn't
  // read as the tenant's own number while sitting in the same stat row.
  platform?: boolean;
}

function StatCard({ title, value, icon: Icon, trend, description, platform }: StatCardProps) {
  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-start justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <p className="text-sm font-medium text-muted-foreground">{title}</p>
              {platform && (
                <span className="rounded-full border border-border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
                  Platform
                </span>
              )}
            </div>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold tracking-tight">
                {typeof value === "number" ? value.toLocaleString() : value}
              </span>
              {trend && (
                <span
                  className={cn(
                    "flex items-center text-xs font-medium",
                    trend.isPositive ? "text-green-600" : "text-red-600"
                  )}
                >
                  {trend.isPositive ? (
                    <TrendingUp className="mr-0.5 h-3 w-3" />
                  ) : (
                    <TrendingDown className="mr-0.5 h-3 w-3" />
                  )}
                  {trend.value}%
                </span>
              )}
            </div>
            {description && <p className="text-xs text-muted-foreground">{description}</p>}
          </div>
          <div className="rounded-lg bg-primary/10 p-3">
            <Icon className="h-5 w-5 text-primary" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export function StatsCards() {
  const { stats, isLoading, refresh } = useDashboardStats();

  const statCards = [
    {
      // REAL tenant-scoped total from the event store (summed per-type counts),
      // not the billing meter — which reads 0 for out-of-band ingestion.
      title: "Total Events",
      value: isLoading ? "—" : stats.events.total,
      icon: Activity,
      // Stream count has its own card below; don't duplicate it here. Just the
      // event-type breakdown, which nothing else shows.
      description: `${stats.store.eventTypes.toLocaleString()} event types`,
    },
    {
      title: "Active Projections",
      value: isLoading ? "—" : stats.projections.active,
      icon: GitBranch,
      description: `${stats.projections.count} total projections`,
    },
    {
      title: "Event Streams",
      value: isLoading ? "—" : stats.store.streams,
      icon: Database,
      // "Stream" = one entity's event sequence. Use the term consistently
      // (card title, here, and the API) — don't call the same number "entities".
      description: "Distinct streams in your store",
    },
    {
      // Platform/system p99 across all query types, derived from Core's
      // query-duration histogram (global, all tenants) — labelled as a platform
      // metric, not "your" latency. Honest "—" when no queries observed yet.
      title: "p99 Latency",
      value: isLoading ? "—" : stats.latency.formatted,
      icon: Zap,
      platform: true,
      description:
        stats.latency.formatted === "—" ? "Awaiting query samples" : "Across all tenants & queries",
    },
  ];

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={refresh}
          disabled={isLoading}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
        >
          <RefreshCw className={cn("h-3 w-3", isLoading && "animate-spin")} />
          {isLoading ? "Refreshing..." : "Refresh"}
        </button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {statCards.map((stat) => (
          <StatCard key={stat.title} {...stat} />
        ))}
      </div>
    </div>
  );
}
