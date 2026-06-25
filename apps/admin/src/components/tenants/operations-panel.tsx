"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  CreditCard,
  KeyRound,
  Pencil,
  RefreshCcw,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  Wrench,
} from "lucide-react";
import { useCallback, useState } from "react";
import {
  type ConfirmMode,
  RecoveryDialog,
  type RecoveryRisk,
} from "@/components/fleet/recovery-dialog";
import { EditQuotasDialog } from "@/components/tenants/edit-quotas-dialog";
import { SuspendDialog } from "@/components/tenants/suspend-dialog";
import {
  type HealthTier,
  type RecoveryAction,
  type RecoveryRequest,
  type RecoveryResponse,
  runRecovery,
} from "@/lib/fleet-api";
import type { TenantQuotas, TenantStatus } from "@/lib/tenants-api";

/**
 * Operations panel (Pillar B) — surfaces the ALREADY-SHIPPED recovery + lifecycle
 * actions as a first-class, health-driven operations cockpit on the tenant 360.
 *
 * It does NOT add a second recovery flow or a parallel design system. It reuses,
 * verbatim:
 *   - RecoveryDialog (components/fleet/recovery-dialog.tsx) for the guarded
 *     recovery actions — it already renders the dry-run preview → typed-id /
 *     echoed-count confirm → Apply guard, mirroring the server guard.
 *   - SuspendDialog / EditQuotasDialog (components/tenants/*) for lifecycle/quota.
 *   - runRecovery (lib/fleet-api.ts) for the /api/v1/admin/recovery/* calls and
 *     the existing tenants-api lifecycle calls for suspend/unsuspend/quotas.
 *
 * Every call is SAME-ORIGIN via the BFF. No new server guard is added — the
 * Control Plane use-case layer enforces dry-run + typed/echoed confirmation +
 * Core audit; this panel renders that contract as real UI.
 *
 * Health-driven prominence: when a recovery action maps to a Critical/At-Risk
 * health signal (e.g. api_key_validity Critical → Rotate keys), that action is
 * pulled to the front and flagged "Recommended", so at-risk tenants surface
 * recovery first.
 */

// ── Recovery action catalog (mirrors fleet/[id]/page.tsx — the canonical
// recovery surface — so the two consoles read identically) ────────────────
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
  /** The health signal whose Critical/At-Risk tier recommends this action. */
  signalTrigger?: string;
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
    signalTrigger: "last_sync_age",
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
    signalTrigger: "events_quota_pct",
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
    signalTrigger: "subscription_state",
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
    signalTrigger: "api_key_validity",
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

interface OperationsPanelProps {
  tenantId: string;
  tenantName: string;
  status: TenantStatus;
  quotas: TenantQuotas;
  /** Map of health signal → tier (from …/fleet/health/:id) for prominence. */
  signalTiers?: Record<string, HealthTier>;
  /** Lifecycle handlers wired by the parent to the existing tenants-api calls. */
  onSaveQuotas: (quotas: TenantQuotas) => Promise<void>;
  onToggleSuspend: () => Promise<void>;
  /** Re-fetch the 360 after a successful recovery apply. */
  onAfterRecovery?: () => Promise<void> | void;
}

function isRecommended(spec: RecoverySpec, signalTiers?: Record<string, HealthTier>): boolean {
  if (!spec.signalTrigger || !signalTiers) return false;
  const tier = signalTiers[spec.signalTrigger];
  return tier === "critical" || tier === "at_risk";
}

export function OperationsPanel({
  tenantId,
  tenantName,
  status,
  quotas,
  signalTiers,
  onSaveQuotas,
  onToggleSuspend,
  onAfterRecovery,
}: OperationsPanelProps) {
  const [activeSpec, setActiveSpec] = useState<RecoverySpec | null>(null);
  const [snapshotId, setSnapshotId] = useState("");
  const [editQuotasOpen, setEditQuotasOpen] = useState(false);
  const [suspendOpen, setSuspendOpen] = useState(false);

  const isSuspended = status === "suspended";

  // Build the recovery request body for a given dry_run + confirmation state.
  // Identical contract to fleet/[id]/page.tsx so the guard is rendered the same.
  const buildBody = useCallback(
    (spec: RecoverySpec, opts: { dryRun: boolean; confirmToken?: string }): RecoveryRequest => {
      const body: RecoveryRequest = {
        dry_run: opts.dryRun,
        reason: "Admin tenant 360 operations panel",
      };
      if (!opts.dryRun) {
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
    async (spec: RecoverySpec): Promise<RecoveryResponse> =>
      runRecovery(tenantId, spec.action, buildBody(spec, { dryRun: true })),
    [tenantId, buildBody]
  );

  const handleApply = useCallback(
    async (spec: RecoverySpec, confirmToken?: string): Promise<void> => {
      await runRecovery(tenantId, spec.action, buildBody(spec, { dryRun: false, confirmToken }));
      await onAfterRecovery?.();
    },
    [tenantId, buildBody, onAfterRecovery]
  );

  // Order: recommended (health-driven) actions first, then the rest in catalog order.
  const orderedSpecs = [...RECOVERY_SPECS].sort((a, b) => {
    const ra = isRecommended(a, signalTiers) ? 0 : 1;
    const rb = isRecommended(b, signalTiers) ? 0 : 1;
    return ra - rb;
  });

  return (
    <Card data-testid="tenant-operations-panel">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Wrench className="h-5 w-5" />
          Operations
        </CardTitle>
        <CardDescription>
          Guarded lifecycle &amp; recovery actions for this tenant. Every recovery action previews a
          dry-run first; Destructive actions stay disabled until a typed/echoed confirmation
          matches. The same guard is enforced server-side (a raw curl hits it too).
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Lifecycle controls (existing tenant endpoints) */}
        <div className="space-y-2" data-testid="ops-lifecycle">
          <p className="text-sm font-medium">Lifecycle</p>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              onClick={() => setEditQuotasOpen(true)}
              data-testid="ops-edit-quotas-btn"
            >
              <Pencil className="mr-2 h-4 w-4" />
              Edit quotas
            </Button>
            <Button
              variant={isSuspended ? "default" : "destructive"}
              onClick={() => setSuspendOpen(true)}
              data-testid="ops-suspend-btn"
            >
              {isSuspended ? (
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

        {/* Recovery actions (shipped /api/v1/admin/recovery/* — guarded) */}
        <div className="space-y-2" data-testid="ops-recovery">
          <p className="text-sm font-medium">Recovery</p>
          {activeSpec?.needsSnapshot && (
            <div className="space-y-1.5" data-testid="ops-snapshot-input">
              <Label htmlFor="ops-snapshot-id" className="text-xs text-muted-foreground">
                Snapshot id (restore) — passed straight through to the recovery body
              </Label>
              <Input
                id="ops-snapshot-id"
                value={snapshotId}
                onChange={(e) => setSnapshotId(e.target.value)}
                placeholder="snapshot-…"
                autoComplete="off"
                className="max-w-sm"
              />
            </div>
          )}
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {orderedSpecs.map((spec) => {
              const Icon = spec.icon;
              const recommended = isRecommended(spec, signalTiers);
              return (
                <button
                  key={spec.action}
                  type="button"
                  onClick={() => setActiveSpec(spec)}
                  className={cn(
                    "flex flex-col items-start gap-1.5 rounded-lg border p-4 text-left transition-colors hover:bg-muted",
                    recommended && "border-red-500/50 ring-1 ring-red-500/20"
                  )}
                  data-testid={`ops-action-${spec.action}`}
                >
                  <div className="flex flex-wrap items-center gap-2">
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
                    {recommended && (
                      <Badge
                        variant="secondary"
                        className="text-[10px]"
                        data-testid={`ops-recommended-${spec.action}`}
                      >
                        Recommended
                      </Badge>
                    )}
                  </div>
                  <span className="text-xs text-muted-foreground">{spec.description}</span>
                </button>
              );
            })}
          </div>
        </div>
      </CardContent>

      {/* Lifecycle dialogs (existing) */}
      <EditQuotasDialog
        open={editQuotasOpen}
        onClose={() => setEditQuotasOpen(false)}
        quotas={quotas}
        onSave={onSaveQuotas}
      />
      <SuspendDialog
        open={suspendOpen}
        onClose={() => setSuspendOpen(false)}
        tenantName={tenantName}
        isSuspended={isSuspended}
        onConfirm={onToggleSuspend}
      />

      {/* Recovery dialog (one at a time) — the shipped guarded flow, verbatim */}
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
          tenantId={tenantId}
          tenantName={tenantName}
          onPreview={() => handlePreview(activeSpec)}
          onApply={(token) => handleApply(activeSpec, token)}
          applyLabel={activeSpec.label}
        />
      )}
    </Card>
  );
}
