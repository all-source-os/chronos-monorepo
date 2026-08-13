"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { BarChart3, Hash, TrendingUp } from "lucide-react";
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
import { FadeIn } from "@/components/ui/fade-in";
import type { EventTypeDistribution, IngestionDataPoint, TopEntity } from "@/lib/api/client";

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

function formatTimestamp(timestamp: string, range: string): string {
  try {
    const date = new Date(timestamp);
    if (range === "24h") {
      return date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
    }
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  } catch {
    return timestamp;
  }
}

export function AnalyticsCharts({
  range,
  ingestionRate,
  eventTypeDistribution,
  topEntityIds,
}: {
  range: string;
  ingestionRate: IngestionDataPoint[];
  eventTypeDistribution: EventTypeDistribution[];
  topEntityIds: TopEntity[];
}) {
  return (
    <>
      <FadeIn delay={0.2} inView>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5" />
              Ingestion rate
            </CardTitle>
          </CardHeader>
          <CardContent>
            {ingestionRate.length === 0 ? (
              <div className="py-12 text-center text-sm text-muted-foreground">
                No ingestion data for this range.
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
                      tickFormatter={(value) => formatTimestamp(value, range)}
                      fontSize={12}
                      tick={{ fill: "var(--color-muted-foreground)" }}
                    />
                    <YAxis fontSize={12} tick={{ fill: "var(--color-muted-foreground)" }} />
                    <Tooltip
                      labelFormatter={(value) => formatTimestamp(String(value), range)}
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
      </FadeIn>

      <div className="grid gap-6 lg:grid-cols-2">
        <FadeIn delay={0.25} inView>
          <Card className="h-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Hash className="h-5 w-5" />
                Event type distribution
              </CardTitle>
            </CardHeader>
            <CardContent>
              {eventTypeDistribution.length === 0 ? (
                <div className="py-12 text-center text-sm text-muted-foreground">
                  No event types found for this range.
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
                          {eventTypeDistribution.slice(0, 10).map((item, index) => (
                            <Cell
                              key={item.event_type}
                              fill={CHART_COLORS[index % CHART_COLORS.length]}
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
                    {eventTypeDistribution.slice(0, 10).map((item, index) => (
                      <div
                        key={item.event_type}
                        className="flex items-center justify-between text-sm"
                      >
                        <div className="flex min-w-0 items-center gap-2">
                          <span
                            className="inline-block h-3 w-3 shrink-0 rounded-full"
                            style={{ backgroundColor: CHART_COLORS[index % CHART_COLORS.length] }}
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
        </FadeIn>

        <FadeIn delay={0.3} inView>
          <Card className="h-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <BarChart3 className="h-5 w-5" />
                Top entity IDs
              </CardTitle>
            </CardHeader>
            <CardContent>
              {topEntityIds.length === 0 ? (
                <div className="py-12 text-center text-sm text-muted-foreground">
                  No entities found for this range.
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
                        tick={{ fill: "var(--color-muted-foreground)" }}
                      />
                      <YAxis
                        type="category"
                        dataKey="entity_id"
                        width={120}
                        fontSize={12}
                        tick={{ fill: "var(--color-muted-foreground)" }}
                        tickFormatter={(value: string) =>
                          value.length > 16 ? `${value.slice(0, 14)}...` : value
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
        </FadeIn>
      </div>
    </>
  );
}
