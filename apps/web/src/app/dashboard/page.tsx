"use client";

import { Badge, Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, ArrowRight, Plus, Sparkles } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { CreateKeyDialog } from "@/components/api-keys/create-key-dialog";
import { KeyTable } from "@/components/api-keys/key-table";
import { UsageChart } from "@/components/billing/usage-chart";
import { LiveMetrics } from "@/components/dashboard/live-metrics";
import { QuickActions } from "@/components/dashboard/quick-actions";
import { RecentEvents } from "@/components/dashboard/recent-events";
import { StatsCards } from "@/components/dashboard/stats-cards";
import { FadeIn } from "@/components/ui/fade-in";
import { useApiKeys } from "@/hooks/use-api-keys";
import { useDashboardStats } from "@/hooks/use-dashboard-stats";
import type { ApiKeyWithSecret } from "@/lib/api/client";
import { apiClient } from "@/lib/api/client";
import { useAuthStore } from "@/lib/stores/auth-store";
import { canonicalTier } from "@/lib/tier";

export default function DashboardPage() {
  const { user, tenant } = useAuthStore();
  const { stats } = useDashboardStats();
  const { keys, isLoading: keysLoading, createKey, rotateKey, revokeKey } = useApiKeys();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState<string | null>(null);

  const currentPlan = canonicalTier(tenant?.subscription_tier);
  // Period usage (billing meter) → quota bars only.
  const eventsUsed = stats.events.used || tenant?.events_used || 0;
  const eventsQuota = stats.events.quota || tenant?.events_quota || 10000;
  const queriesUsed = stats.queries.used || tenant?.queries_used || 0;
  const queriesQuota = stats.queries.quota || tenant?.queries_quota || 10000;
  // Real tenant-scoped event total (event-store count, not the meter).
  const totalEvents = stats.events.total || eventsUsed;
  // Real daily ingestion series for the 30-day chart.
  const ingestionHistory = stats.ingestion.map((p) => p.count);
  const ingestionTotal = stats.ingestion.reduce((sum, p) => sum + p.count, 0);
  // Real daily query series for the 30-day chart (per-tenant). Empty until reads
  // are event-sourced — the chart shows an honest empty state in that case.
  const queryHistory = stats.queries_series.map((p) => p.count);
  const queryTotal = stats.queries_series.reduce((sum, p) => sum + p.count, 0);

  const greeting = () => {
    const hour = new Date().getHours();
    if (hour < 12) return "Good morning";
    if (hour < 18) return "Good afternoon";
    return "Good evening";
  };

  const handleCreateKey = async (data: {
    name: string;
    description?: string;
    scopes: string[];
    expires_at?: string;
  }): Promise<ApiKeyWithSecret | undefined> => {
    try {
      return await createKey(data);
    } catch (error) {
      console.error("Failed to create API key:", error);
      return undefined;
    }
  };

  const handleRotate = async (id: string) => {
    try {
      await rotateKey(id);
    } catch (error) {
      console.error("Failed to rotate API key:", error);
    }
  };

  const handleRevoke = async (id: string) => {
    if (confirmRevoke !== id) {
      setConfirmRevoke(id);
      return;
    }
    try {
      await revokeKey(id);
    } catch (error) {
      console.error("Failed to revoke API key:", error);
    }
    setConfirmRevoke(null);
  };

  const handleUpgrade = async () => {
    try {
      const response = await apiClient.createCheckout("studio");
      if (response.data?.checkout_url) {
        window.location.href = response.data.checkout_url;
        return;
      }
    } catch (error) {
      console.error("Failed to create checkout:", error);
    }
    // Fallback: navigate to billing page if checkout fails
    window.location.href = "/dashboard/billing";
  };

  return (
    <div className="space-y-8">
      {/* Header */}
      <FadeIn delay={0.1}>
        <div>
          <h1 className="text-2xl font-bold tracking-tight md:text-3xl">
            {greeting()}, {user?.name?.split(" ")[0] || "there"}
          </h1>
          <p className="mt-1 text-muted-foreground">
            Here&apos;s what&apos;s happening with your event store today.
          </p>
        </div>
      </FadeIn>

      {/* Stats Cards */}
      <FadeIn delay={0.2}>
        <StatsCards />
      </FadeIn>

      {/* Current Plan & Quota */}
      <FadeIn delay={0.25}>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <div>
              <CardTitle className="text-base font-medium">Current Plan</CardTitle>
            </div>
            <div className="flex items-center gap-3">
              <Badge
                variant={currentPlan === "self-host" ? "secondary" : "default"}
                className="text-sm capitalize"
              >
                {currentPlan === "self-host" ? "Developer" : currentPlan}
              </Badge>
              {currentPlan === "self-host" && (
                <Button size="sm" onClick={handleUpgrade}>
                  <Sparkles className="mr-1.5 h-4 w-4" />
                  Upgrade Plan
                </Button>
              )}
              {currentPlan !== "self-host" && (
                <Button variant="outline" size="sm" asChild>
                  <Link href="/dashboard/billing">
                    Manage Plan
                    <ArrowRight className="ml-1.5 h-3.5 w-3.5" />
                  </Link>
                </Button>
              )}
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid gap-6 md:grid-cols-2">
              {/* Events quota */}
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium">Events</span>
                  <span className="text-muted-foreground">
                    {eventsUsed.toLocaleString()} / {eventsQuota.toLocaleString()}
                  </span>
                </div>
                <div className="relative h-2 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className={cn(
                      "h-full rounded-full transition-all duration-500",
                      eventsQuota > 0 && (eventsUsed / eventsQuota) * 100 >= 95
                        ? "bg-destructive"
                        : eventsQuota > 0 && (eventsUsed / eventsQuota) * 100 >= 80
                          ? "bg-yellow-500"
                          : "bg-primary"
                    )}
                    style={{
                      width: `${Math.min(eventsQuota > 0 ? (eventsUsed / eventsQuota) * 100 : 0, 100)}%`,
                    }}
                  />
                </div>
              </div>

              {/* Queries quota */}
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium">Queries</span>
                  <span className="text-muted-foreground">
                    {queriesUsed.toLocaleString()} / {queriesQuota.toLocaleString()}
                  </span>
                </div>
                <div className="relative h-2 w-full overflow-hidden rounded-full bg-muted">
                  <div
                    className={cn(
                      "h-full rounded-full transition-all duration-500",
                      queriesQuota > 0 && (queriesUsed / queriesQuota) * 100 >= 95
                        ? "bg-destructive"
                        : queriesQuota > 0 && (queriesUsed / queriesQuota) * 100 >= 80
                          ? "bg-yellow-500"
                          : "bg-primary"
                    )}
                    style={{
                      width: `${Math.min(queriesQuota > 0 ? (queriesUsed / queriesQuota) * 100 : 0, 100)}%`,
                    }}
                  />
                </div>
                {/* queries_used only counts gateway-routed reads (QS
                    UsageEnforcement); direct-to-Core reads aren't metered. */}
                <p className="text-xs text-muted-foreground">
                  Counts reads routed through the API gateway; direct-to-Core reads aren't metered
                  yet.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </FadeIn>

      {/* Usage Charts - Daily Event Ingestion (30 days) */}
      <FadeIn delay={0.3}>
        <div className="grid gap-6 md:grid-cols-2">
          {/* Real daily ingestion series from /api/tenants/me/analytics. The "used"
              figure here is the real 30-day event total, not the billing meter. */}
          <UsageChart
            title="Event Ingestion (30 days)"
            used={ingestionTotal}
            quota={eventsQuota}
            history={ingestionHistory}
            emptyLabel="No events ingested in the last 30 days"
          />
          {/* Real per-tenant daily query series from /api/tenants/me/analytics
              (query_rate). Reads aren't event-sourced yet, so the series is
              honestly empty for now and the chart shows the empty state instead
              of a fake trend; it fills in automatically once query events flow.
              When data exists, "used" reflects the real 30-day query total. */}
          <UsageChart
            title="Query Usage (30 days)"
            used={queryHistory.length > 0 ? queryTotal : queriesUsed}
            quota={queriesQuota}
            history={queryHistory}
            emptyLabel="No query activity recorded in the last 30 days"
          />
        </div>
      </FadeIn>

      {/* Main Grid */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Live Metrics */}
        <FadeIn delay={0.35} className="lg:col-span-2">
          <LiveMetrics />
        </FadeIn>

        {/* Quick Actions */}
        <FadeIn delay={0.4}>
          <QuickActions />
        </FadeIn>
      </div>

      {/* API Keys Section */}
      <FadeIn delay={0.45}>
        <div className="space-y-4">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-lg font-semibold">API Keys</h2>
              <p className="text-sm text-muted-foreground">
                Manage keys for authenticating your applications
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="outline" size="sm" asChild>
                <Link href="/dashboard/api-keys">
                  View All
                  <ArrowRight className="ml-1.5 h-3.5 w-3.5" />
                </Link>
              </Button>
              <Button size="sm" onClick={() => setShowCreateDialog(true)}>
                <Plus className="mr-1.5 h-4 w-4" />
                Create Key
              </Button>
            </div>
          </div>
          <KeyTable
            keys={keys.slice(0, 5)}
            isLoading={keysLoading}
            onRotate={handleRotate}
            onRevoke={handleRevoke}
          />
        </div>
      </FadeIn>

      {/* Recent Events */}
      <FadeIn delay={0.5}>
        <RecentEvents />
      </FadeIn>

      {/* Instance Stats Banner */}
      <FadeIn delay={0.55}>
        <div className="rounded-xl border border-border bg-gradient-to-r from-primary/5 via-primary/10 to-primary/5 p-6">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h3 className="text-lg font-semibold">Your Event Store</h3>
              <p className="text-sm text-muted-foreground">Real-time instance metrics</p>
            </div>
            <div className="flex flex-wrap gap-6 text-center">
              <div>
                <p className="text-2xl font-bold text-primary">{totalEvents.toLocaleString()}</p>
                <p className="text-xs text-muted-foreground">total events</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">{stats.latency.formatted}</p>
                {/* Core's query-duration p99 is a process-global, all-tenant
                    figure — label it platform, not this tenant's. */}
                <p className="text-xs text-muted-foreground">p99 latency (platform)</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">{stats.projections.active}</p>
                <p className="text-xs text-muted-foreground">active projections</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-primary">{stats.storage.formatted}</p>
                {/* allsource_storage_size_bytes is the whole data-dir on disk
                    (all tenants) — platform storage, not this tenant's. */}
                <p className="text-xs text-muted-foreground">storage on disk (platform)</p>
              </div>
            </div>
          </div>
        </div>
      </FadeIn>

      {/* Create Key Dialog */}
      <CreateKeyDialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onCreateKey={handleCreateKey}
      />

      {/* Revoke confirmation */}
      {confirmRevoke && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <button
            type="button"
            className="absolute inset-0 bg-background/80 backdrop-blur-sm"
            onClick={() => setConfirmRevoke(null)}
            onKeyDown={(e) => {
              if (e.key === "Escape") setConfirmRevoke(null);
            }}
            aria-label="Close dialog"
          />
          <Card className="relative z-10 w-full max-w-sm mx-4">
            <CardContent className="p-6">
              <AlertTriangle className="mx-auto mb-4 h-12 w-12 text-destructive" />
              <h3 className="mb-2 text-center text-lg font-semibold">Revoke API Key?</h3>
              <p className="mb-6 text-center text-sm text-muted-foreground">
                This action cannot be undone. Any applications using this key will stop working
                immediately.
              </p>
              <div className="flex gap-2">
                <Button variant="outline" className="flex-1" onClick={() => setConfirmRevoke(null)}>
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  className="flex-1"
                  onClick={() => handleRevoke(confirmRevoke)}
                >
                  Revoke Key
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
