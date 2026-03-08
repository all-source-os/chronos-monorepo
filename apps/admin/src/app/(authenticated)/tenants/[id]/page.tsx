"use client";

import { useCallback, useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import {
  ArrowLeft,
  Calendar,
  CreditCard,
  Database,
  Pencil,
  Search,
  ShieldAlert,
  ShieldCheck,
  Users,
  Zap,
} from "lucide-react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
  fetchTenantDetail,
  fetchTenantUsage,
  updateTenantQuotas,
  suspendTenant,
  unsuspendTenant,
  type TenantDetail,
  type TenantUsage,
  type TenantStatus,
} from "@/lib/tenants-api";
import { EditQuotasDialog } from "@/components/tenants/edit-quotas-dialog";
import { SuspendDialog } from "@/components/tenants/suspend-dialog";
import { UsageBar } from "@/components/tenants/usage-bar";

function statusVariant(
  status: TenantStatus
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "active":
      return "default";
    case "suspended":
      return "destructive";
    case "archived":
      return "secondary";
    default:
      return "outline";
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function TenantDetailPage() {
  const params = useParams();
  const router = useRouter();
  const tenantId = params.id as string;

  const [tenant, setTenant] = useState<TenantDetail | null>(null);
  const [usage, setUsage] = useState<TenantUsage | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [editQuotasOpen, setEditQuotasOpen] = useState(false);
  const [suspendOpen, setSuspendOpen] = useState(false);

  const loadData = useCallback(async () => {
    setIsLoading(true);
    try {
      const [tenantData, usageData] = await Promise.all([
        fetchTenantDetail(tenantId),
        fetchTenantUsage(tenantId),
      ]);
      setTenant(tenantData);
      setUsage(usageData);
    } catch (err) {
      console.error("Failed to load tenant detail:", err);
    } finally {
      setIsLoading(false);
    }
  }, [tenantId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSaveQuotas = async (quotas: {
    event_limit: number;
    query_limit: number;
    storage_limit_mb: number;
  }) => {
    await updateTenantQuotas(tenantId, quotas);
    await loadData();
  };

  const handleToggleSuspend = async () => {
    if (!tenant) return;
    if (tenant.status === "suspended") {
      await unsuspendTenant(tenantId);
    } else {
      await suspendTenant(tenantId);
    }
    await loadData();
  };

  // Chart config
  const chartConfig: ChartConfig = {
    events: {
      label: "Events",
      color: "hsl(var(--primary))",
    },
  };

  if (isLoading) {
    return (
      <div className="space-y-6" data-testid="tenant-detail-loading">
        <Skeleton className="h-8 w-48" />
        <div className="grid gap-4 md:grid-cols-2">
          <Skeleton className="h-48" />
          <Skeleton className="h-48" />
        </div>
        <Skeleton className="h-64" />
      </div>
    );
  }

  if (!tenant || !usage) {
    return (
      <div className="space-y-4" data-testid="tenant-detail-error">
        <p className="text-muted-foreground">Tenant not found.</p>
        <Button variant="outline" onClick={() => router.push("/tenants")}>
          Back to Tenants
        </Button>
      </div>
    );
  }

  const chartData = usage.daily.map((d) => ({
    date: new Date(d.date).toLocaleDateString("en-US", { month: "short", day: "numeric" }),
    events: d.events,
  }));

  return (
    <div className="space-y-6" data-testid="tenant-detail-page">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-2 text-sm text-muted-foreground" data-testid="tenant-breadcrumb">
        <Link href="/tenants" className="hover:text-foreground transition-colors flex items-center gap-1">
          <ArrowLeft className="h-4 w-4" />
          Tenants
        </Link>
        <span>/</span>
        <span className="text-foreground font-medium">{tenant.name}</span>
      </nav>

      {/* Header with actions */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{tenant.name}</h1>
          {tenant.description && (
            <p className="text-muted-foreground mt-1">{tenant.description}</p>
          )}
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            onClick={() => setEditQuotasOpen(true)}
            data-testid="edit-quotas-btn"
          >
            <Pencil className="mr-2 h-4 w-4" />
            Edit Quotas
          </Button>
          <Button
            variant={tenant.status === "suspended" ? "default" : "destructive"}
            onClick={() => setSuspendOpen(true)}
            data-testid="suspend-btn"
          >
            {tenant.status === "suspended" ? (
              <>
                <ShieldCheck className="mr-2 h-4 w-4" />
                Unsuspend
              </>
            ) : (
              <>
                <ShieldAlert className="mr-2 h-4 w-4" />
                Suspend
              </>
            )}
          </Button>
        </div>
      </div>

      {/* Tenant Info + Subscription */}
      <div className="grid gap-4 md:grid-cols-2">
        <Card data-testid="tenant-info-card">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="h-5 w-5" />
              Tenant Info
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">ID</span>
              <span className="text-sm font-mono">{tenant.id}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Status</span>
              <Badge variant={statusVariant(tenant.status)} data-testid="tenant-status-badge">
                {tenant.status}
              </Badge>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Plan</span>
              <Badge variant="outline">{tenant.plan}</Badge>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Members</span>
              <span className="text-sm">{tenant.members_count}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Created</span>
              <span className="text-sm">{formatDate(tenant.created_at)}</span>
            </div>
          </CardContent>
        </Card>

        <Card data-testid="tenant-subscription-card">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CreditCard className="h-5 w-5" />
              Subscription
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Plan</span>
              <span className="text-sm font-medium capitalize">{tenant.subscription.plan}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Started</span>
              <span className="text-sm">{formatDate(tenant.subscription.started_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-muted-foreground">Current Period End</span>
              <span className="text-sm">{formatDate(tenant.subscription.current_period_end)}</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Usage Stats */}
      <Card data-testid="tenant-usage-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Zap className="h-5 w-5" />
            Usage
          </CardTitle>
          <CardDescription>Current billing period usage and quotas</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Stat tiles */}
          <div className="grid gap-4 sm:grid-cols-3">
            <div className="rounded-lg border p-4 space-y-1" data-testid="stat-events-ingested">
              <p className="text-sm text-muted-foreground">Events Ingested</p>
              <p className="text-2xl font-bold tabular-nums">{usage.events_ingested.toLocaleString()}</p>
            </div>
            <div className="rounded-lg border p-4 space-y-1" data-testid="stat-queries-run">
              <p className="text-sm text-muted-foreground">Queries Run</p>
              <p className="text-2xl font-bold tabular-nums">{usage.queries_run.toLocaleString()}</p>
            </div>
            <div className="rounded-lg border p-4 space-y-1" data-testid="stat-storage-used">
              <p className="text-sm text-muted-foreground">Storage Used</p>
              <p className="text-2xl font-bold tabular-nums">{usage.storage_used_mb.toLocaleString()} MB</p>
            </div>
          </div>

          {/* Quota bars */}
          <div className="space-y-4">
            <UsageBar
              label="Events"
              current={usage.events_ingested}
              limit={usage.event_limit}
              testId="usage-bar-events"
            />
            <UsageBar
              label="Queries"
              current={usage.queries_run}
              limit={usage.query_limit}
              testId="usage-bar-queries"
            />
            <UsageBar
              label="Storage"
              current={usage.storage_used_mb}
              limit={usage.storage_limit_mb}
              unit=" MB"
              testId="usage-bar-storage"
            />
          </div>

          {/* Daily events sparkline */}
          {chartData.length > 0 && (
            <div data-testid="daily-events-chart">
              <p className="text-sm font-medium mb-2">Daily Events (Last 30 Days)</p>
              <ChartContainer config={chartConfig} className="h-[200px] w-full">
                <AreaChart data={chartData}>
                  <defs>
                    <linearGradient id="fill-events" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="hsl(var(--primary))" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="hsl(var(--primary))" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" vertical={false} />
                  <XAxis
                    dataKey="date"
                    tickLine={false}
                    axisLine={false}
                    tickMargin={8}
                    fontSize={12}
                  />
                  <YAxis
                    tickLine={false}
                    axisLine={false}
                    tickMargin={8}
                    fontSize={12}
                  />
                  <ChartTooltip
                    content={
                      <ChartTooltipContent
                        formatter={(value) => (
                          <span className="font-mono font-medium">
                            {typeof value === "number" ? value.toLocaleString() : value}
                          </span>
                        )}
                      />
                    }
                  />
                  <Area
                    type="monotone"
                    dataKey="events"
                    stroke="hsl(var(--primary))"
                    fill="url(#fill-events)"
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Members List */}
      <Card data-testid="tenant-members-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="h-5 w-5" />
            Members
          </CardTitle>
          <CardDescription>{tenant.members.length} member{tenant.members.length !== 1 ? "s" : ""}</CardDescription>
        </CardHeader>
        <CardContent>
          {tenant.members.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4 text-center">No members found.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Email</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Joined</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tenant.members.map((member) => (
                  <TableRow key={member.id} data-testid={`member-row-${member.id}`}>
                    <TableCell className="font-medium">{member.email}</TableCell>
                    <TableCell>
                      <Badge variant="outline">{member.role}</Badge>
                    </TableCell>
                    <TableCell>{formatDate(member.joined_at)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Audit Log */}
      <Card data-testid="tenant-audit-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Calendar className="h-5 w-5" />
            Recent Audit Log
          </CardTitle>
        </CardHeader>
        <CardContent>
          {tenant.audit_log.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4 text-center">No audit entries.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Action</TableHead>
                  <TableHead>Actor</TableHead>
                  <TableHead>Details</TableHead>
                  <TableHead>Timestamp</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {tenant.audit_log.map((entry) => (
                  <TableRow key={entry.id} data-testid={`audit-row-${entry.id}`}>
                    <TableCell className="font-medium">{entry.action}</TableCell>
                    <TableCell>{entry.actor}</TableCell>
                    <TableCell className="text-muted-foreground">{entry.details || "-"}</TableCell>
                    <TableCell>{formatDateTime(entry.timestamp)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Dialogs */}
      <EditQuotasDialog
        open={editQuotasOpen}
        onClose={() => setEditQuotasOpen(false)}
        quotas={tenant.quotas}
        onSave={handleSaveQuotas}
      />
      <SuspendDialog
        open={suspendOpen}
        onClose={() => setSuspendOpen(false)}
        tenantName={tenant.name}
        isSuspended={tenant.status === "suspended"}
        onConfirm={handleToggleSuspend}
      />
    </div>
  );
}
