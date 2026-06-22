"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
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
  CreditCard,
  ExternalLink,
  KeyRound,
  RefreshCcw,
  RotateCcw,
  ShieldAlert,
  Stethoscope,
  Wrench,
} from "lucide-react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { HealthChip } from "@/components/fleet/health-chip";
import {
  type ConfirmMode,
  RecoveryDialog,
  type RecoveryRisk,
} from "@/components/fleet/recovery-dialog";
import { UsageBar } from "@/components/tenants/usage-bar";
import {
  fetchTenantHealth,
  type HealthSignal,
  type RecoveryAction,
  type RecoveryRequest,
  type RecoveryResponse,
  runRecovery,
  type TenantHealth,
} from "@/lib/fleet-api";

// ── Fixture (clearly marked) ──────────────────────────────────────────
// The Control Plane fleet endpoints (commit a02667e) are NOT yet deployed.
// This fixture lets the drill-down render its full layout when the API is
// unreachable. fetchTenantHealth() still calls the documented route exactly.
function fixtureTenantHealth(id: string): TenantHealth {
  return {
    tenant_id: id,
    name: "Acme Corp",
    tier: "critical",
    signals: [
      {
        signal: "subscription_state",
        value: "canceled",
        tier: "critical",
        source: "Core tenant metadata",
      },
      {
        signal: "events_quota_pct",
        value: "118%",
        tier: "at_risk",
        source: "Core tenant metadata",
      },
      {
        signal: "last_event_age",
        value: "2h",
        tier: "healthy",
        source: "Core (desc probe)",
      },
      {
        signal: "last_sync_age",
        value: "31h",
        tier: "at_risk",
        source: "Core (sync probe)",
      },
      {
        signal: "durability",
        value: "durable=true",
        tier: "healthy",
        source: "Core health_deep",
      },
      {
        signal: "api_key_validity",
        value: "serviceaccount OK",
        tier: "healthy",
        source: "Core auth",
      },
    ],
    subscription: {
      tier: "pro",
      status: "canceled",
      renews_at: "2026-05-01T00:00:00Z",
      grandfathered: false,
      events_used: 590_000,
      events_quota: 500_000,
    },
  };
}

// ── Recovery action catalog ───────────────────────────────────────────
// Only the NEW recovery actions go through fleet-api.ts. Guarded actions
// (reactivate/suspend/edit_quotas) keep calling the existing tenant endpoints
// from the tenant detail page — they are intentionally NOT duplicated here.
interface RecoverySpec {
  action: RecoveryAction;
  label: string;
  title: string;
  description: string;
  risk: RecoveryRisk;
  confirmMode: ConfirmMode;
  icon: typeof Wrench;
  /** restore needs a snapshot id passed in the body. */
  needsSnapshot?: boolean;
}

const RECOVERY_SPECS: RecoverySpec[] = [
  {
    action: "resync",
    label: "Force re-sync",
    title: "Force re-sync",
    description:
      "Re-trigger ingestion / reconcile counters for stale sync state. Guarded — affects one tenant.",
    risk: "guarded",
    confirmMode: "none",
    icon: RefreshCcw,
  },
  {
    action: "reconcile-subscription",
    label: "Reconcile subscription",
    title: "Reconcile subscription",
    description:
      "Re-apply the tier's entitlements (QuotasForTier) to fix quota drift. Guarded — affects one tenant.",
    risk: "guarded",
    confirmMode: "none",
    icon: Wrench,
  },
  {
    action: "resolve-dunning",
    label: "Resolve dunning",
    title: "Resolve dunning",
    description:
      "Re-issue checkout / mark for manual review / extend grace for a past-due tenant. Guarded.",
    risk: "guarded",
    confirmMode: "none",
    icon: CreditCard,
  },
  {
    action: "rotate-keys",
    label: "Rotate keys",
    title: "Rotate API keys",
    description:
      "Re-mint the tenant's keys with the canonical serviceaccount role. Destructive — current keys stop working. Confirm via the echoed dry-run token.",
    risk: "destructive",
    confirmMode: "count_token",
    icon: KeyRound,
  },
  {
    action: "reprovision",
    label: "Reprovision tenant",
    title: "Reprovision tenant",
    description:
      "Rewrite tenant metadata for a corrupt/incomplete onboarding. Destructive — type the tenant id to confirm. Hard-rejected if active with recent events.",
    risk: "destructive",
    confirmMode: "typed_id",
    icon: ShieldAlert,
  },
  {
    action: "restore",
    label: "Restore from backup",
    title: "Restore from backup",
    description:
      "Replay a snapshot to recover from corruption. Destructive — type the tenant id to confirm. The preview shows the snapshot id, age, and event count.",
    risk: "destructive",
    confirmMode: "typed_id",
    icon: RotateCcw,
    needsSnapshot: true,
  },
];

function signalTierVariant(signal: HealthSignal) {
  return signal.tier;
}

export default function FleetTenantPage() {
  const params = useParams();
  const router = useRouter();
  const tenantId = params.id as string;

  const [health, setHealth] = useState<TenantHealth | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [usingFixture, setUsingFixture] = useState(false);
  const [activeSpec, setActiveSpec] = useState<RecoverySpec | null>(null);
  const [snapshotId, setSnapshotId] = useState("");

  const loadData = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchTenantHealth(tenantId);
      setHealth(data);
      setUsingFixture(false);
    } catch (err) {
      console.error("Failed to load tenant health (using fixture):", err);
      setHealth(fixtureTenantHealth(tenantId));
      setUsingFixture(true);
    } finally {
      setIsLoading(false);
    }
  }, [tenantId]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Build the recovery request body for a given dry_run + confirmation state.
  const buildBody = useCallback(
    (spec: RecoverySpec, opts: { dryRun: boolean; confirmToken?: string }): RecoveryRequest => {
      const body: RecoveryRequest = {
        dry_run: opts.dryRun,
        reason: "Admin fleet recovery console",
      };
      if (!opts.dryRun) {
        // Apply: pass the confirmation the guard requires.
        if (spec.confirmMode === "typed_id") {
          body.confirm_tenant_id = tenantId;
        }
        if (opts.confirmToken) {
          body.confirm_token = opts.confirmToken;
        }
      }
      if (spec.needsSnapshot && snapshotId.trim()) {
        body.snapshot_id = snapshotId.trim();
      }
      return body;
    },
    [tenantId, snapshotId]
  );

  const handlePreview = useCallback(
    async (spec: RecoverySpec): Promise<RecoveryResponse> => {
      return runRecovery(tenantId, spec.action, buildBody(spec, { dryRun: true }));
    },
    [tenantId, buildBody]
  );

  const handleApply = useCallback(
    async (spec: RecoverySpec, confirmToken?: string): Promise<void> => {
      await runRecovery(tenantId, spec.action, buildBody(spec, { dryRun: false, confirmToken }));
      await loadData();
    },
    [tenantId, buildBody, loadData]
  );

  if (isLoading) {
    return (
      <div className="space-y-6" data-testid="fleet-tenant-loading">
        <Skeleton className="h-8 w-48" />
        <div className="grid gap-4 md:grid-cols-2">
          <Skeleton className="h-48" />
          <Skeleton className="h-48" />
        </div>
        <Skeleton className="h-64" />
      </div>
    );
  }

  if (!health) {
    return (
      <div className="space-y-4" data-testid="fleet-tenant-error">
        <p className="text-muted-foreground">Tenant health not found.</p>
        <Button variant="outline" onClick={() => router.push("/fleet")}>
          Back to Fleet
        </Button>
      </div>
    );
  }

  const sub = health.subscription;

  return (
    <div className="space-y-6" data-testid="fleet-tenant-page">
      {/* Breadcrumb */}
      <nav
        className="flex items-center gap-2 text-sm text-muted-foreground"
        data-testid="fleet-tenant-breadcrumb"
      >
        <Link
          href="/fleet"
          className="hover:text-foreground transition-colors flex items-center gap-1"
        >
          <ArrowLeft className="h-4 w-4" />
          Fleet
        </Link>
        <span>/</span>
        <span className="text-foreground font-medium">{health.name || health.tenant_id}</span>
      </nav>

      {/* Header */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-3">
          <h1 className="text-3xl font-bold tracking-tight">{health.name || health.tenant_id}</h1>
          <HealthChip tier={health.tier} />
        </div>
        <div className="flex gap-2">
          <Link href={`/tenants/${health.tenant_id}`}>
            <Button variant="outline" data-testid="fleet-link-tenant">
              <ExternalLink className="mr-2 h-4 w-4" />
              Tenant detail
            </Button>
          </Link>
          <Link href="/billing">
            <Button variant="outline" data-testid="fleet-link-billing">
              <CreditCard className="mr-2 h-4 w-4" />
              Billing
            </Button>
          </Link>
        </div>
      </div>

      {usingFixture && (
        <div
          className="flex items-center gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-600"
          data-testid="fleet-tenant-fixture-banner"
        >
          <Stethoscope className="h-4 w-4 shrink-0" />
          <span>Showing sample data — the tenant-health endpoint is not reachable yet.</span>
        </div>
      )}

      <div className="grid gap-4 lg:grid-cols-3">
        {/* Signals table (2/3) */}
        <Card className="lg:col-span-2" data-testid="fleet-signals-card">
          <CardHeader>
            <CardTitle>Health signals</CardTitle>
            <CardDescription>
              Every signal, its observed value, the tier it triggered, and the backend it was read
              from.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Table data-testid="fleet-signals-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Signal</TableHead>
                  <TableHead>Value</TableHead>
                  <TableHead>Tier</TableHead>
                  <TableHead>Source</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {health.signals.map((s) => (
                  <TableRow key={s.signal} data-testid={`fleet-signal-${s.signal}`}>
                    <TableCell className="font-mono text-sm">{s.signal}</TableCell>
                    <TableCell className="font-medium">{s.value}</TableCell>
                    <TableCell>
                      <HealthChip tier={signalTierVariant(s)} />
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">{s.source}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        {/* Subscription + quota (1/3) */}
        <Card data-testid="fleet-subscription-card">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CreditCard className="h-5 w-5" />
              Subscription
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {sub ? (
              <>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Tier</span>
                    <span className="text-sm font-medium capitalize">{sub.tier || "--"}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Status</span>
                    <Badge
                      variant={
                        sub.status === "active" || sub.status === "on_trial"
                          ? "default"
                          : sub.status === "past_due"
                            ? "secondary"
                            : "destructive"
                      }
                      data-testid="fleet-sub-status"
                    >
                      {sub.status || "unknown"}
                    </Badge>
                  </div>
                  {sub.renews_at && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">Renews</span>
                      <span className="text-sm">
                        {new Date(sub.renews_at).toLocaleDateString()}
                      </span>
                    </div>
                  )}
                  {sub.grandfathered && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">Grandfathered</span>
                      <Badge variant="outline">
                        until{" "}
                        {sub.grandfather_until
                          ? new Date(sub.grandfather_until).toLocaleDateString()
                          : "—"}
                      </Badge>
                    </div>
                  )}
                </div>

                {sub.events_quota !== undefined && (
                  <UsageBar
                    label="Events quota"
                    current={sub.events_used ?? 0}
                    limit={sub.events_quota}
                    testId="fleet-quota-bar"
                  />
                )}
              </>
            ) : (
              <p className="text-sm text-muted-foreground">No subscription data.</p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Recovery console */}
      <Card data-testid="fleet-recovery-console">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wrench className="h-5 w-5" />
            Recovery console
          </CardTitle>
          <CardDescription>
            Guarded recovery actions. Every action previews a dry-run first; Destructive actions
            require a typed confirmation before Apply unlocks. Suspend / reactivate / edit-quotas
            live on the{" "}
            <Link href={`/tenants/${health.tenant_id}`} className="underline">
              tenant detail page
            </Link>
            .
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Restore needs a snapshot id input */}
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {RECOVERY_SPECS.map((spec) => {
              const Icon = spec.icon;
              return (
                <button
                  key={spec.action}
                  type="button"
                  onClick={() => setActiveSpec(spec)}
                  className="flex flex-col items-start gap-1.5 rounded-lg border p-4 text-left transition-colors hover:bg-muted"
                  data-testid={`recovery-action-${spec.action}`}
                >
                  <div className="flex items-center gap-2">
                    <Icon
                      className={
                        spec.risk === "destructive"
                          ? "h-4 w-4 text-red-500"
                          : "h-4 w-4 text-muted-foreground"
                      }
                    />
                    <span className="text-sm font-medium">{spec.label}</span>
                    {spec.risk === "destructive" && (
                      <Badge variant="destructive" className="text-[10px]">
                        Destructive
                      </Badge>
                    )}
                  </div>
                  <span className="text-xs text-muted-foreground">{spec.description}</span>
                </button>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Recovery dialog (one at a time) */}
      {activeSpec && (
        <RecoveryDialog
          open={true}
          onClose={() => {
            setActiveSpec(null);
            setSnapshotId("");
          }}
          title={activeSpec.title}
          description={activeSpec.description}
          risk={activeSpec.risk}
          confirmMode={activeSpec.confirmMode}
          tenantId={health.tenant_id}
          tenantName={health.name}
          onPreview={() => handlePreview(activeSpec)}
          onApply={(token) => handleApply(activeSpec, token)}
          applyLabel={activeSpec.label}
        />
      )}
    </div>
  );
}
