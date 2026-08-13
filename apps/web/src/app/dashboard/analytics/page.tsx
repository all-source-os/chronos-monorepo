"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { AlertCircle, BarChart3, Database, Hash, RefreshCw } from "lucide-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useState } from "react";

import { FadeIn } from "@/components/ui/fade-in";
import { useUsageAnalytics } from "@/hooks/use-usage-analytics";

const AnalyticsCharts = dynamic(
  () => import("@/components/dashboard/analytics-charts").then((module) => module.AnalyticsCharts),
  {
    ssr: false,
    loading: () => (
      <div className="grid gap-6" role="status" aria-label="Loading analytics charts">
        <div className="h-80 animate-pulse rounded-xl border border-border bg-card" />
        <div className="grid gap-6 lg:grid-cols-2">
          <div className="h-96 animate-pulse rounded-xl border border-border bg-card" />
          <div className="h-96 animate-pulse rounded-xl border border-border bg-card" />
        </div>
      </div>
    ),
  }
);

const TIME_RANGES = [
  { label: "24h", value: "24h" },
  { label: "7d", value: "7d" },
  { label: "30d", value: "30d" },
  { label: "90d", value: "90d" },
] as const;

export default function AnalyticsPage() {
  const [range, setRange] = useState("7d");
  const { eventTypeDistribution, topEntityIds, ingestionRate, isLoading, error, refresh } =
    useUsageAnalytics(range);

  const totalEvents = eventTypeDistribution.reduce((sum, d) => sum + d.count, 0);
  const uniqueTypes = eventTypeDistribution.length;
  const uniqueEntities = topEntityIds.length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <FadeIn delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Usage Analytics</h1>
            <p className="mt-1 text-muted-foreground">
              Detailed breakdown of your event store usage
            </p>
          </div>
          <div className="flex items-center gap-1 rounded-lg border border-border p-1">
            {TIME_RANGES.map((tr) => (
              <button
                key={tr.value}
                type="button"
                onClick={() => setRange(tr.value)}
                className={`rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
                  range === tr.value
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {tr.label}
              </button>
            ))}
          </div>
        </div>
      </FadeIn>

      {/* Summary cards */}
      <FadeIn delay={0.15} inView>
        <div className="grid gap-4 sm:grid-cols-3">
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="rounded-lg bg-blue-500/10 p-2.5">
                <Database className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Total Events</p>
                <p className="text-2xl font-bold tabular-nums">
                  {isLoading || error ? "—" : totalEvents.toLocaleString()}
                </p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="rounded-lg bg-purple-500/10 p-2.5">
                <Hash className="h-5 w-5 text-purple-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Event Types</p>
                <p className="text-2xl font-bold tabular-nums">
                  {isLoading || error ? "—" : uniqueTypes}
                </p>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="rounded-lg bg-green-500/10 p-2.5">
                <BarChart3 className="h-5 w-5 text-green-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Unique Entities</p>
                <p className="text-2xl font-bold tabular-nums">
                  {isLoading || error ? "—" : uniqueEntities}
                </p>
              </div>
            </CardContent>
          </Card>
        </div>
      </FadeIn>

      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        </div>
      ) : error ? (
        <Card className="border-amber-500/30 bg-amber-500/5">
          <CardContent className="flex flex-col items-center py-12 text-center">
            <AlertCircle className="mb-3 h-8 w-8 text-amber-500" />
            <h2 className="font-semibold">Usage analytics is unavailable</h2>
            <p className="mt-1 max-w-lg text-sm text-muted-foreground">
              Event ingestion remains available. Retry this report or inspect current tenant data in
              Event Explorer.
            </p>
            <p className="mt-2 max-w-lg text-xs text-muted-foreground">{error}</p>
            <div className="mt-5 flex gap-2">
              <Button variant="outline" size="sm" onClick={() => refresh()}>
                <RefreshCw className="mr-1.5 h-4 w-4" />
                Try again
              </Button>
              <Button size="sm" asChild>
                <Link href="/dashboard/events">Browse events</Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <AnalyticsCharts
          range={range}
          ingestionRate={ingestionRate}
          eventTypeDistribution={eventTypeDistribution}
          topEntityIds={topEntityIds}
        />
      )}
    </div>
  );
}
