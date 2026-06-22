"use client";

import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, Eye, ShieldAlert } from "lucide-react";
import { useState } from "react";
import type { RecoveryResponse } from "@/lib/fleet-api";

/**
 * Recovery dialog — extends the SuspendDialog UX (suspend-dialog.tsx) scaled up
 * for risk. It renders the server guards as REAL, ENFORCED client UI:
 *
 *   (a) Two-step dry-run: "Preview" calls the action with dry_run:true and renders
 *       the returned `would` payload. The real "Apply" stays DISABLED until a
 *       preview has been shown.
 *   (b) Typed-name confirmation (Destructive): an input compared client-side to
 *       the target tenant_id. Apply stays disabled until it matches AND the
 *       preview has run.
 *   (c) Batch: the dry-run's affected-tenant list + count is shown; the operator
 *       echoes the count to confirm (the dialog forwards the confirm_token).
 *
 * The client guard mirrors the server guard — both must hold. Guarded (non-
 * Destructive) actions only require the preview-then-apply step; Destructive
 * actions additionally require the typed id (or echoed token for batch).
 *
 * The dialog is presentation-only: it calls `onPreview` / `onApply` callbacks the
 * parent wires to fleet-api.ts. It never recomputes health or talks to the API
 * directly.
 */

export type RecoveryRisk = "guarded" | "destructive";

/** How the Destructive Apply is unlocked. */
export type ConfirmMode =
  | "none" // Guarded — preview then apply
  | "typed_id" // Destructive single-tenant — type the tenant id
  | "count_token"; // Batch — echo the affected count + forward confirm_token

interface RecoveryDialogProps {
  open: boolean;
  onClose: () => void;
  /** Action title, e.g. "Reprovision Tenant". */
  title: string;
  /** Plain-language description of what the action does and its blast radius. */
  description: string;
  risk: RecoveryRisk;
  confirmMode: ConfirmMode;
  /** The target tenant id (typed-name confirmation compares against this). */
  tenantId?: string;
  /** Friendly tenant name for copy. */
  tenantName?: string;
  /** Calls the action with dry_run:true and returns the preview payload. */
  onPreview: () => Promise<RecoveryResponse>;
  /** Applies the action for real. Receives the confirm_token from the preview, if any. */
  onApply: (confirmToken?: string) => Promise<void>;
  applyLabel?: string;
}

export function RecoveryDialog({
  open,
  onClose,
  title,
  description,
  risk,
  confirmMode,
  tenantId,
  tenantName,
  onPreview,
  onApply,
  applyLabel,
}: RecoveryDialogProps) {
  const [preview, setPreview] = useState<RecoveryResponse | null>(null);
  const [typedId, setTypedId] = useState("");
  const [typedCount, setTypedCount] = useState("");
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const reset = () => {
    setPreview(null);
    setTypedId("");
    setTypedCount("");
    setError(null);
    setIsPreviewing(false);
    setIsSubmitting(false);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handlePreview = async () => {
    setIsPreviewing(true);
    setError(null);
    try {
      const result = await onPreview();
      setPreview(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Preview failed");
    } finally {
      setIsPreviewing(false);
    }
  };

  // ── Guard evaluation (mirrors the server guard) ───────────────────────
  // GUARD 1: a dry-run preview must have been shown before Apply is enabled.
  const previewShown = preview !== null;

  // GUARD 2 (Destructive, typed_id): the typed id must exactly equal the target.
  const idMatches = confirmMode === "typed_id" ? typedId === tenantId : true;

  // GUARD 3 (Batch, count_token): the typed count must equal the affected count.
  const affectedCount = preview?.count ?? preview?.affected?.length ?? 0;
  const countMatches =
    confirmMode === "count_token"
      ? typedCount.trim() !== "" && Number(typedCount) === affectedCount
      : true;

  const applyEnabled = previewShown && idMatches && countMatches && !isSubmitting && !isPreviewing;

  const handleApply = async () => {
    if (!applyEnabled) return;
    setIsSubmitting(true);
    setError(null);
    try {
      await onApply(preview?.confirm_token);
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Apply failed");
      setIsSubmitting(false);
    }
  };

  const isDestructive = risk === "destructive";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="recovery-dialog"
    >
      {/* Backdrop (click to dismiss — mirrors SuspendDialog) */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel button is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel button is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={handleClose} />

      {/* Dialog */}
      <Card className="relative z-50 w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            {isDestructive ? (
              <ShieldAlert className="h-5 w-5 text-red-500" />
            ) : (
              <Eye className="h-5 w-5 text-muted-foreground" />
            )}
            {title}
          </CardTitle>
          <CardDescription>{description}</CardDescription>
        </CardHeader>

        <CardContent className="space-y-4">
          {isDestructive && (
            <div
              className="flex items-start gap-2 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-500"
              data-testid="recovery-destructive-warning"
            >
              <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
              <p>
                This is a <strong>Destructive</strong> action
                {tenantName ? (
                  <>
                    {" "}
                    on <strong>{tenantName}</strong>
                  </>
                ) : null}
                . Preview first, then confirm the typed{" "}
                {confirmMode === "count_token" ? "count" : "tenant id"} to enable Apply. The same
                guard is enforced server-side.
              </p>
            </div>
          )}

          {/* Step 1 — dry-run preview */}
          <div className="space-y-2" data-testid="recovery-preview-pane">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">1. Dry-run preview</span>
              <Button
                variant="outline"
                size="sm"
                onClick={handlePreview}
                disabled={isPreviewing || isSubmitting}
                data-testid="recovery-preview-btn"
              >
                <Eye className="mr-1.5 h-3.5 w-3.5" />
                {isPreviewing ? "Previewing…" : previewShown ? "Re-preview" : "Preview"}
              </Button>
            </div>

            {!previewShown ? (
              <p className="text-xs text-muted-foreground" data-testid="recovery-preview-empty">
                Run a dry-run to see exactly what would change before applying.
              </p>
            ) : (
              <div className="space-y-3" data-testid="recovery-preview-result">
                {/* `would` payload */}
                {preview?.would && (
                  <pre
                    className="max-h-48 overflow-auto rounded-lg border bg-muted p-3 text-xs"
                    data-testid="recovery-would-payload"
                  >
                    {JSON.stringify(preview.would, null, 2)}
                  </pre>
                )}

                {/* Batch — affected tenant list + count */}
                {confirmMode === "count_token" && (
                  <div
                    className="rounded-lg border p-3 text-sm"
                    data-testid="recovery-affected-list"
                  >
                    <p className="font-medium mb-2">
                      {affectedCount} tenant{affectedCount !== 1 ? "s" : ""} would be affected
                    </p>
                    {preview?.affected && preview.affected.length > 0 && (
                      <ul className="space-y-1 max-h-40 overflow-auto">
                        {preview.affected.map((t) => (
                          <li
                            key={t.tenant_id}
                            className="flex items-center justify-between gap-2 text-xs"
                            data-testid={`recovery-affected-${t.tenant_id}`}
                          >
                            <span className="font-medium">{t.name || t.tenant_id}</span>
                            <span className="font-mono text-muted-foreground">{t.tenant_id}</span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                )}

                {preview?.confirm_token && (
                  <p className="text-xs text-muted-foreground" data-testid="recovery-confirm-token">
                    Confirmation token issued — it will be echoed automatically on Apply.
                  </p>
                )}
              </div>
            )}
          </div>

          {/* Step 2 — confirmation (Destructive only) */}
          {confirmMode === "typed_id" && (
            <div className="space-y-2">
              <Label htmlFor="recovery-confirm-id" className="text-sm font-medium">
                2. Type the tenant id to confirm
              </Label>
              <Input
                id="recovery-confirm-id"
                value={typedId}
                onChange={(e) => setTypedId(e.target.value)}
                placeholder={tenantId}
                autoComplete="off"
                disabled={!previewShown || isSubmitting}
                data-testid="recovery-confirm-input"
                className={cn(
                  typedId !== "" && !idMatches && "border-red-500 focus-visible:ring-red-500"
                )}
              />
              <p className="text-xs text-muted-foreground">
                Must exactly equal <span className="font-mono">{tenantId}</span>
                {!previewShown && " — run the preview first."}
              </p>
            </div>
          )}

          {confirmMode === "count_token" && previewShown && (
            <div className="space-y-2">
              <Label htmlFor="recovery-confirm-count" className="text-sm font-medium">
                2. Type the affected count to confirm
              </Label>
              <Input
                id="recovery-confirm-count"
                inputMode="numeric"
                value={typedCount}
                onChange={(e) => setTypedCount(e.target.value)}
                placeholder={String(affectedCount)}
                autoComplete="off"
                disabled={isSubmitting}
                data-testid="recovery-confirm-count-input"
                className={cn(
                  typedCount !== "" && !countMatches && "border-red-500 focus-visible:ring-red-500"
                )}
              />
              <p className="text-xs text-muted-foreground">
                Type <span className="font-mono">{affectedCount}</span> to confirm the blast radius.
              </p>
            </div>
          )}

          {error && (
            <p className="text-sm text-red-500" data-testid="recovery-error">
              {error}
            </p>
          )}
        </CardContent>

        <CardFooter className="flex justify-end gap-2">
          <Button
            variant="outline"
            onClick={handleClose}
            disabled={isSubmitting}
            data-testid="recovery-cancel-btn"
          >
            Cancel
          </Button>
          <Button
            variant={isDestructive ? "destructive" : "default"}
            onClick={handleApply}
            disabled={!applyEnabled}
            data-testid="recovery-apply-btn"
          >
            {isSubmitting ? "Applying…" : applyLabel || "Apply"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
