"use client";

import { BlurFade } from "@allsource/ui";
import { StatsCards } from "@/components/dashboard/stats-cards";
import { LiveMetrics } from "@/components/dashboard/live-metrics";
import { RecentEvents } from "@/components/dashboard/recent-events";
import { QuickActions } from "@/components/dashboard/quick-actions";
import { UsageProgress } from "@/components/dashboard/usage-progress";
import { useAuthStore } from "@/lib/stores/auth-store";

export default function DashboardPage() {
  const { user, tenant } = useAuthStore();

  const greeting = () => {
    const hour = new Date().getHours();
    if (hour < 12) return "Good morning";
    if (hour < 18) return "Good afternoon";
    return "Good evening";
  };

  return (
    <div className="space-y-8">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div>
          <h1 className="text-2xl font-bold tracking-tight md:text-3xl">
            {greeting()}, {user?.name?.split(" ")[0] || "there"}
          </h1>
          <p className="mt-1 text-muted-foreground">
            Here&apos;s what&apos;s happening with your event store today.
          </p>
        </div>
      </BlurFade>

      {/* Stats Cards */}
      <BlurFade delay={0.2} inView>
        <StatsCards />
      </BlurFade>

      {/* Main Grid */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Live Metrics - WOW Feature */}
        <BlurFade delay={0.3} inView className="lg:col-span-2">
          <LiveMetrics />
        </BlurFade>

        {/* Usage Progress */}
        <BlurFade delay={0.4} inView>
          <UsageProgress />
        </BlurFade>
      </div>

      {/* Secondary Grid */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Recent Events */}
        <BlurFade delay={0.5} inView>
          <RecentEvents />
        </BlurFade>

        {/* Quick Actions */}
        <BlurFade delay={0.6} inView>
          <QuickActions />
        </BlurFade>
      </div>

      {/* Product Stats Banner */}
      <BlurFade delay={0.7} inView>
        <div className="rounded-xl border border-border bg-gradient-to-r from-primary/5 via-primary/10 to-primary/5 p-6">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h3 className="text-lg font-semibold">AllSource Event Store</h3>
              <p className="text-sm text-muted-foreground">
                Powering real-time data intelligence
              </p>
            </div>
            <div className="flex flex-wrap gap-6 text-center">
              <div>
                <p className="text-2xl font-bold text-primary">469K</p>
                <p className="text-xs text-muted-foreground">events/sec</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">11.9μs</p>
                <p className="text-xs text-muted-foreground">p99 latency</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">27</p>
                <p className="text-xs text-muted-foreground">MCP tools</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">~129MB</p>
                <p className="text-xs text-muted-foreground">footprint</p>
              </div>
            </div>
          </div>
        </div>
      </BlurFade>
    </div>
  );
}
