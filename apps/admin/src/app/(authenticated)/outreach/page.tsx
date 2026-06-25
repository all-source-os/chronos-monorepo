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
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Textarea,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertCircle, LifeBuoy, RefreshCw, Send, ShieldAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { HealthChip } from "@/components/fleet/health-chip";
import {
  type ConfirmMode,
  RecoveryDialog,
  type RecoveryRisk,
} from "@/components/fleet/recovery-dialog";
import {
  type CreateNoticeResult,
  createNotice,
  fetchAtRiskCohort,
  type NoticeSeverity,
} from "@/lib/comms-api";
import type { FleetWorstTenant, HealthTier, RecoveryResponse } from "@/lib/fleet-api";

/**
 * At-risk outreach (Pillar C, §9 Phase 6) — the fleet-level cohort screen. It
 * lists every At-Risk / Critical tenant (from the shipped fleet-health rollup via
 * fetchAtRiskCohort → fetchFleetHealth — NO tier re-derivation here) and sends a
 * templated in-app notice to the whole cohort, AUDIENCE BY HEALTH TIER.
 *
 * The cohort send is a blast-radius action, so it reuses the SHIPPED guard UI
 * verbatim: components/fleet/recovery-dialog.tsx with confirmMode "count_token".
 * The dialog renders the two-step guard — (1) a dry-run "Preview" that shows the
 * recipient_tenant_ids + count and (2) an echoed-confirm_token Apply that stays
 * DISABLED until the preview ran AND the operator types the affected count. The
 * SAME guard is enforced server-side (the CP mints + validates the confirm_token);
 * the UI never bypasses it.
 *
 * The notice/recovery response shapes differ, so a thin adapter
 * (noticeResultToRecoveryResponse) maps CreateNoticeResult.would.recipient_tenant_ids
 * onto the RecoveryResponse the dialog consumes — letting us reuse the dialog
 * with zero changes.
 */

// The health tiers the operator can target. "at_risk" and "critical" are the
// outreach cohorts; the page defaults to at_risk.
const TARGET_TIERS: { tier: HealthTier; label: string }[] = [
  { tier: "at_risk", label: "At-Risk" },
  { tier: "critical", label: "Critical" },
];

const SEVERITIES: NoticeSeverity[] = ["info", "warning", "critical"];

/**
 * Adapt a cohort-notice CreateNoticeResult into the RecoveryResponse the shipped
 * recovery-dialog renders. The dialog reads `would`, `count`, `affected[]`, and
 * `confirm_token`; the notice dry-run returns `would.recipient_tenant_ids` + a
 * `confirm_token`. We surface the recipient ids as both the `would` payload (the
 * dialog's JSON preview) and the `affected` list (the dialog's per-tenant list +
 * count), and forward the confirm_token unchanged so Apply echoes it.
 */
function noticeResultToRecoveryResponse(
  result: CreateNoticeResult,
  tenantNameById: Record<string, string>
): RecoveryResponse {
  const ids = Array.isArray(result.would?.recipient_tenant_ids)
    ? result.would.recipient_tenant_ids
    : [];
  const count = result.would?.count ?? result.count ?? ids.length;
  return {
    dry_run: result.dry_run,
    would: result.would ? { recipient_tenant_ids: ids, count } : undefined,
    confirm_token: result.confirm_token,
    count,
    affected: ids.map((id) => ({ tenant_id: id, name: tenantNameById[id] || id })),
  };
}

export default function OutreachPage() {
  const [cohort, setCohort] = useState<FleetWorstTenant[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Targeting + message
  const [targetTier, setTargetTier] = useState<HealthTier>("at_risk");
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [severity, setSeverity] = useState<NoticeSeverity>("warning");

  // Guarded cohort send (recovery-dialog two-step)
  const [dialogOpen, setDialogOpen] = useState(false);
  const [appliedResult, setAppliedResult] = useState<CreateNoticeResult | null>(null);

  const loadCohort = useCallback(async () => {
    setIsLoading(true);
    setLoadError(null);
    try {
      // Pull a generous slice of the worst-N list; the fleet rollup already
      // ranks Critical → Degraded, so this contains the at-risk/critical cohort.
      const worst = await fetchAtRiskCohort({ limit: 100 });
      setCohort(worst);
    } catch (err) {
      console.error("Failed to load at-risk cohort:", err);
      setLoadError(err instanceof Error ? err.message : "Failed to load fleet health");
      setCohort([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCohort();
  }, [loadCohort]);

  // The tenants in the currently-selected target tier (display + count preview).
  const targeted = cohort.filter((t) => t.tier === targetTier);

  // ── Guarded send wiring (reuses recovery-dialog.tsx) ────────────────
  // PREVIEW: createNotice with audience {health_tier} + dry_run:true → the CP
  // returns would.recipient_tenant_ids + count + a confirm_token. We adapt it to
  // the RecoveryResponse the dialog renders, resolving recipient ids → names from
  // the loaded cohort.
  const handlePreview = useCallback(async (): Promise<RecoveryResponse> => {
    const result = await createNotice({
      audience: { health_tier: targetTier },
      title: title.trim(),
      body: body.trim(),
      severity,
      dry_run: true,
    });
    const nameById: Record<string, string> = {};
    for (const t of cohort) nameById[t.tenant_id] = t.name || t.tenant_id;
    return noticeResultToRecoveryResponse(result, nameById);
  }, [targetTier, title, body, severity, cohort]);

  // APPLY: createNotice with the SAME audience + the echoed confirm_token (no
  // dry_run) → the CP validates the token and posts to every recipient.
  const handleApply = useCallback(
    async (confirmToken?: string): Promise<void> => {
      const result = await createNotice({
        audience: { health_tier: targetTier },
        title: title.trim(),
        body: body.trim(),
        severity,
        confirm_token: confirmToken,
      });
      setAppliedResult(result);
      await loadCohort();
    },
    [targetTier, title, body, severity, loadCohort]
  );

  const sendReady = title.trim() !== "" && body.trim() !== "" && targeted.length > 0;

  const dialogRisk: RecoveryRisk = "destructive";
  const dialogConfirmMode: ConfirmMode = "count_token";

  return (
    <div className="space-y-6" data-testid="outreach-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">At-risk outreach</h1>
          <p className="text-muted-foreground">
            Reach the tenants who need help. Send a templated in-app notice to every At-Risk /
            Critical tenant — audience by health tier, behind the same dry-run →
            echoed-confirm_token guard as recovery.
          </p>
        </div>
        <button
          type="button"
          onClick={loadCohort}
          className="rounded-md p-2 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
          aria-label="Refresh cohort"
          data-testid="outreach-refresh-btn"
        >
          <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
        </button>
      </div>

      {loadError && (
        <div
          className="flex items-center gap-2 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-500"
          data-testid="outreach-load-error"
        >
          <AlertCircle className="h-4 w-4 shrink-0" />
          <span>{loadError}</span>
        </div>
      )}

      {/* Compose the cohort notice */}
      <Card data-testid="outreach-compose-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <LifeBuoy className="h-5 w-5" />
            Compose outreach notice
          </CardTitle>
          <CardDescription>
            Targets the selected health-tier cohort. The send is guarded: you must run a dry-run
            preview (recipients + count) and echo the count before it applies — the Control Plane
            mints and validates the confirm_token.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="outreach-tier" className="text-xs text-muted-foreground">
                Target health tier
              </Label>
              <Select
                id="outreach-tier"
                value={targetTier}
                onChange={(e) => setTargetTier(e.target.value as HealthTier)}
                data-testid="outreach-tier-select"
              >
                {TARGET_TIERS.map((t) => (
                  <option key={t.tier} value={t.tier}>
                    {t.label} ({cohort.filter((c) => c.tier === t.tier).length})
                  </option>
                ))}
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="outreach-severity" className="text-xs text-muted-foreground">
                Severity
              </Label>
              <Select
                id="outreach-severity"
                value={severity}
                onChange={(e) => setSeverity(e.target.value as NoticeSeverity)}
                data-testid="outreach-severity-select"
              >
                {SEVERITIES.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </Select>
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="outreach-title" className="text-xs text-muted-foreground">
              Title
            </Label>
            <Input
              id="outreach-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="We noticed your workspace needs attention"
              data-testid="outreach-title"
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="outreach-body" className="text-xs text-muted-foreground">
              Body
            </Label>
            <Textarea
              id="outreach-body"
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Your sync looks stalled — here's how to fix it…"
              data-testid="outreach-body"
            />
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3">
            <p className="text-sm text-muted-foreground" data-testid="outreach-target-count">
              <span className="font-medium text-foreground">{targeted.length}</span> tenant
              {targeted.length !== 1 ? "s" : ""} in the{" "}
              <span className="font-medium capitalize">{targetTier.replace("_", "-")}</span> cohort
              will receive this notice.
            </p>
            <Button
              onClick={() => {
                setAppliedResult(null);
                setDialogOpen(true);
              }}
              disabled={!sendReady}
              data-testid="outreach-send-btn"
            >
              <Send className="mr-2 h-4 w-4" />
              Send to cohort…
            </Button>
          </div>

          {appliedResult && (
            <div
              className="flex items-start gap-2 rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-600"
              data-testid="outreach-applied"
            >
              <Send className="h-4 w-4 shrink-0 mt-0.5" />
              <span>
                Posted to{" "}
                <strong>
                  {appliedResult.created?.length ?? appliedResult.count ?? 0} tenant
                  {(appliedResult.created?.length ?? appliedResult.count ?? 0) !== 1 ? "s" : ""}
                </strong>
                . Each notice renders in that tenant&apos;s dashboard banner and is audited.
              </span>
            </div>
          )}
        </CardContent>
      </Card>

      {/* The cohort itself (from the fleet rollup) */}
      <Card data-testid="outreach-cohort-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldAlert className="h-5 w-5" />
            Tenants needing attention
          </CardTitle>
          <CardDescription>
            At-Risk / Critical tenants from the fleet-health rollup. Rows in the selected target
            tier are highlighted — those are who the notice reaches.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
              Loading cohort…
            </div>
          ) : cohort.length === 0 ? (
            <div
              className="flex h-32 items-center justify-center text-sm text-muted-foreground"
              data-testid="outreach-cohort-empty"
            >
              No at-risk or critical tenants. Nothing to reach out to.
            </div>
          ) : (
            <Table data-testid="outreach-cohort-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Tenant</TableHead>
                  <TableHead>Health</TableHead>
                  <TableHead>Contributing signals</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {cohort.map((t) => {
                  const inTarget = t.tier === targetTier;
                  const reasons = Array.isArray(t.reasons) ? t.reasons : [];
                  return (
                    <TableRow
                      key={t.tenant_id}
                      className={cn(inTarget && "bg-primary/5")}
                      data-testid={`outreach-row-${t.tenant_id}`}
                    >
                      <TableCell>
                        <p className="font-medium">{t.name || t.tenant_id}</p>
                        <p className="text-xs text-muted-foreground font-mono">{t.tenant_id}</p>
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <HealthChip tier={t.tier} />
                          {inTarget && (
                            <Badge
                              variant="secondary"
                              className="text-[10px]"
                              data-testid={`outreach-targeted-${t.tenant_id}`}
                            >
                              Targeted
                            </Badge>
                          )}
                        </div>
                      </TableCell>
                      <TableCell>
                        <ul className="space-y-1">
                          {reasons.map((r) => (
                            <li
                              key={`${t.tenant_id}-${r.signal}`}
                              className="flex items-center gap-2 text-xs"
                            >
                              <HealthChip tier={r.tier} dotOnly />
                              <span className="font-mono text-muted-foreground">{r.signal}</span>
                              <span className="font-medium">{r.value}</span>
                            </li>
                          ))}
                        </ul>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Guarded cohort send — the SHIPPED recovery dialog, reused verbatim.
          confirmMode "count_token" → preview shows recipient_tenant_ids + count;
          Apply stays disabled until the preview ran AND the typed count matches;
          the confirm_token from the preview is echoed back automatically. */}
      {dialogOpen && (
        <RecoveryDialog
          open={true}
          onClose={() => setDialogOpen(false)}
          title={`Send notice to ${targeted.length} ${targetTier.replace("_", "-")} tenant${targeted.length !== 1 ? "s" : ""}`}
          description={`In-app notice "${title.trim() || "(untitled)"}" → every tenant in the ${targetTier.replace("_", "-")} health cohort. Preview the recipients, then echo the count to confirm the blast radius. The Control Plane validates the same confirm_token.`}
          risk={dialogRisk}
          confirmMode={dialogConfirmMode}
          tenantName={`the ${targetTier.replace("_", "-")} cohort`}
          onPreview={handlePreview}
          onApply={handleApply}
          applyLabel="Send to cohort"
        />
      )}
    </div>
  );
}
