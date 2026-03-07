"use client";

import { BlurFade, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { BarChart3, Database, Hash, TrendingUp } from "lucide-react";
import { useState } from "react";
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { useUsageAnalytics } from "@/hooks/use-usage-analytics";

const TIME_RANGES = [
  { label: "24h", value: "24h" },
  { label: "7d", value: "7d" },
  { label: "30d", value: "30d" },
  { label: "90d", value: "90d" },
] as const;

const CHART_COLORS = [
  "hsl(221, 83%, 53%)",
  "hsl(262, 83%, 58%)",
  "hsl(330, 81%, 60%)",
  "hsl(24, 94%, 50%)",
  "hsl(142, 71%, 45%)",
  "hsl(47, 96%, 53%)",
  "hsl(199, 89%, 48%)",
  "hsl(346, 77%, 49%)",
  "hsl(173, 58%, 39%)",
  "hsl(27, 87%, 67%)",
];

function formatTimestamp(ts: string, range: string): string {
  try {
    const date = new Date(ts);
    if (range === "24h") {
      return date.toLocaleTimeString(undefined, {
        hour: "2-digit",
        minute: "2-digit",
      });
    }
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return ts;
  }
}

export default function AnalyticsPage() {
  const [range, setRange] = useState("7d");
  const { eventTypeDistribution, topEntityIds, ingestionRate, isLoading, error } =
    useUsageAnalytics(range);

  const totalEvents = eventTypeDistribution.reduce((sum, d) => sum + d.count, 0);
  const uniqueTypes = eventTypeDistribution.length;
  const uniqueEntities = topEntityIds.length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
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
      </BlurFade>

      {/* Summary cards */}
      <BlurFade delay={0.15} inView>
        <div className="grid gap-4 sm:grid-cols-3">
          <Card>
            <CardContent className="flex items-center gap-4 p-4">
              <div className="rounded-lg bg-blue-500/10 p-2.5">
                <Database className="h-5 w-5 text-blue-500" />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">Total Events</p>
                <p className="text-2xl font-bold tabular-nums">{totalEvents.toLocaleString()}</p>
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
                <p className="text-2xl font-bold tabular-nums">{uniqueTypes}</p>
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
                <p className="text-2xl font-bold tabular-nums">{uniqueEntities}</p>
              </div>
            </CardContent>
          </Card>
        </div>
      </BlurFade>

      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        </div>
      ) : error ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-destructive">{error}</CardContent>
        </Card>
      ) : (
        <>
          {/* Ingestion Rate Chart */}
          <BlurFade delay={0.2} inView>
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <TrendingUp className="h-5 w-5" />
                  Ingestion Rate
                </CardTitle>
              </CardHeader>
              <CardContent>
                {ingestionRate.length === 0 ? (
                  <div className="py-12 text-center text-sm text-muted-foreground">
                    No ingestion data for this time range.
                  </div>
                ) : (
                  <div className="h-72">
                    <ResponsiveContainer width="100%" height="100%">
                      <AreaChart data={ingestionRate}>
                        <defs>
                          <linearGradient id="ingestionGradient" x1="0" y1="0" x2="0" y2="1">
                            <stop offset="5%" stopColor="hsl(221, 83%, 53%)" stopOpacity={0.3} />
                            <stop offset="95%" stopColor="hsl(221, 83%, 53%)" stopOpacity={0} />
                          </linearGradient>
                        </defs>
                        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
                        <XAxis
                          dataKey="timestamp"
                          tickFormatter={(ts) => formatTimestamp(ts, range)}
                          fontSize={12}
                          tick={{ fill: "var(--color-muted-foreground)" }}
                        />
                        <YAxis
                          fontSize={12}
                          tick={{ fill: "var(--color-muted-foreground)" }}
                        />
                        <Tooltip
                          labelFormatter={(ts) => formatTimestamp(String(ts), range)}
                          contentStyle={{
                            backgroundColor: "var(--color-card)",
                            border: "1px solid var(--color-border)",
                            borderRadius: "8px",
                            color: "var(--color-card-foreground)",
                          }}
                        />
                        <Area
                          type="monotone"
                          dataKey="count"
                          stroke="hsl(221, 83%, 53%)"
                          fill="url(#ingestionGradient)"
                          strokeWidth={2}
                          name="Events"
                        />
                      </AreaChart>
                    </ResponsiveContainer>
                  </div>
                )}
              </CardContent>
            </Card>
          </BlurFade>

          {/* Event Type Distribution + Top Entities */}
          <div className="grid gap-6 lg:grid-cols-2">
            <BlurFade delay={0.25} inView>
              <Card className="h-full">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Hash className="h-5 w-5" />
                    Event Type Distribution
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {eventTypeDistribution.length === 0 ? (
                    <div className="py-12 text-center text-sm text-muted-foreground">
                      No event types found for this time range.
                    </div>
                  ) : (
                    <div className="space-y-6">
                      <div className="mx-auto h-48 w-48">
                        <ResponsiveContainer width="100%" height="100%">
                          <PieChart>
                            <Pie
                              data={eventTypeDistribution.slice(0, 10)}
                              dataKey="count"
                              nameKey="event_type"
                              cx="50%"
                              cy="50%"
                              innerRadius={40}
                              outerRadius={80}
                              strokeWidth={2}
                              stroke="var(--color-card)"
                            >
                              {eventTypeDistribution.slice(0, 10).map((_, i) => (
                                <Cell
                                  key={`cell-${eventTypeDistribution[i]?.event_type}`}
                                  fill={CHART_COLORS[i % CHART_COLORS.length]}
                                />
                              ))}
                            </Pie>
                            <Tooltip
                              contentStyle={{
                                backgroundColor: "var(--color-card)",
                                border: "1px solid var(--color-border)",
                                borderRadius: "8px",
                                color: "var(--color-card-foreground)",
                              }}
                            />
                          </PieChart>
                        </ResponsiveContainer>
                      </div>
                      <div className="space-y-2">
                        {eventTypeDistribution.slice(0, 10).map((item, i) => (
                          <div
                            key={item.event_type}
                            className="flex items-center justify-between text-sm"
                          >
                            <div className="flex items-center gap-2">
                              <span
                                className="inline-block h-3 w-3 rounded-full"
                                style={{
                                  backgroundColor: CHART_COLORS[i % CHART_COLORS.length],
                                }}
                              />
                              <span className="truncate font-mono text-xs">{item.event_type}</span>
                            </div>
                            <span className="tabular-nums text-muted-foreground">
                              {item.count.toLocaleString()}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>
            </BlurFade>

            <BlurFade delay={0.3} inView>
              <Card className="h-full">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <BarChart3 className="h-5 w-5" />
                    Top Entity IDs
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {topEntityIds.length === 0 ? (
                    <div className="py-12 text-center text-sm text-muted-foreground">
                      No entities found for this time range.
                    </div>
                  ) : (
                    <div className="h-80">
                      <ResponsiveContainer width="100%" height="100%">
                        <BarChart
                          data={topEntityIds.slice(0, 10)}
                          layout="vertical"
                          margin={{ left: 8, right: 16 }}
                        >
                          <CartesianGrid
                            strokeDasharray="3 3"
                            className="stroke-border"
                            horizontal={false}
                          />
                          <XAxis
                            type="number"
                            fontSize={12}
                            tick={{
                              fill: "var(--color-muted-foreground)",
                            }}
                          />
                          <YAxis
                            type="category"
                            dataKey="entity_id"
                            width={120}
                            fontSize={12}
                            tick={{
                              fill: "var(--color-muted-foreground)",
                            }}
                            tickFormatter={(v: string) =>
                              v.length > 16 ? `${v.slice(0, 14)}...` : v
                            }
                          />
                          <Tooltip
                            contentStyle={{
                              backgroundColor: "var(--color-card)",
                              border: "1px solid var(--color-border)",
                              borderRadius: "8px",
                              color: "var(--color-card-foreground)",
                            }}
                          />
                          <Bar
                            dataKey="event_count"
                            fill="hsl(262, 83%, 58%)"
                            radius={[0, 4, 4, 0]}
                            name="Events"
                          />
                        </BarChart>
                      </ResponsiveContainer>
                    </div>
                  )}
                </CardContent>
              </Card>
            </BlurFade>
          </div>
        </>
      )}
    </div>
  );
}
