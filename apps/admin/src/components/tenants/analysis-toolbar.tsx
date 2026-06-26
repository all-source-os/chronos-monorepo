"use client";

import { Badge, Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertTriangle,
  Filter,
  Loader2,
  RefreshCw,
  ScanLine,
  Sparkles,
  Trash2,
  Wallet,
} from "lucide-react";
import type { AnalysisCategory } from "@/lib/analysis-api";

/**
 * AnalysisToolbar — the on-demand deep-analysis controls for the /tenants page
 * (prompt 047 requirement 2 + the "show only flagged" filter from requirement 3).
 *
 * Each button RUNS the read-only deep analysis (GET …/tenants/analyze?category=)
 * for one bucket; "Analyze all" runs every category. The buttons never mutate.
 * The page owns the fetch + state; this component is presentation + callbacks:
 *   - per-button loading spinner (the active category),
 *   - the instant client "N flagged" chip (always present, zero backend),
 *   - the deep "N flagged (deep)" chip once a scan has run,
 *   - the "Show only flagged" toggle,
 *   - the inline "analysis unavailable" notice when the endpoint 404s — the
 *     instant heuristics keep working regardless (graceful degrade, §6).
 */

interface ToolbarButton {
  category: AnalysisCategory | "all";
  label: string;
  Icon: typeof ScanLine;
}

const BUTTONS: ToolbarButton[] = [
  { category: "data_integrity", label: "Scan anomalies", Icon: ScanLine },
  { category: "plan_billing", label: "Audit plans", Icon: Wallet },
  { category: "litter", label: "Find litter", Icon: Trash2 },
  { category: "usage_health", label: "Usage health", Icon: RefreshCw },
  { category: "all", label: "Analyze all", Icon: Sparkles },
];

interface AnalysisToolbarProps {
  /** Tenants flagged by the instant client heuristics (zero backend). */
  clientFlaggedCount: number;
  /** Tenants flagged by the deep scan, or null if no scan has run yet. */
  deepFlaggedCount: number | null;
  /** The category currently loading (its button shows a spinner), or null. */
  loadingCategory: AnalysisCategory | "all" | null;
  /** Whether the "show only flagged" filter is on. */
  showOnlyFlagged: boolean;
  /** True when the last deep scan reported the endpoint unavailable (404/5xx). */
  unavailable: boolean;
  /** Message to show when unavailable. */
  unavailableMessage?: string;
  /** Total findings surfaced (client ∪ deep), for the drawer-open affordance. */
  totalFindings: number;
  onRun: (category: AnalysisCategory | "all") => void;
  onToggleFlagged: (next: boolean) => void;
  onOpenFindings: () => void;
}

export function AnalysisToolbar({
  clientFlaggedCount,
  deepFlaggedCount,
  loadingCategory,
  showOnlyFlagged,
  unavailable,
  unavailableMessage,
  totalFindings,
  onRun,
  onToggleFlagged,
  onOpenFindings,
}: AnalysisToolbarProps) {
  const anyLoading = loadingCategory !== null;

  return (
    <div className="space-y-3" data-testid="analysis-toolbar">
      <div className="flex flex-wrap items-center gap-2">
        {BUTTONS.map(({ category, label, Icon }) => {
          const isLoading = loadingCategory === category;
          return (
            <Button
              key={category}
              variant={category === "all" ? "default" : "outline"}
              size="sm"
              disabled={anyLoading}
              onClick={() => onRun(category)}
              data-testid={`analysis-run-${category}`}
            >
              {isLoading ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Icon className="mr-2 h-4 w-4" />
              )}
              {label}
            </Button>
          );
        })}

        <div className="ml-auto flex items-center gap-2">
          {/* Instant client signal — always present, zero backend. */}
          <Badge
            variant={clientFlaggedCount > 0 ? "secondary" : "outline"}
            className="cursor-pointer"
            onClick={onOpenFindings}
            title="Tenants flagged by instant client heuristics"
            data-testid="analysis-client-flagged"
          >
            {clientFlaggedCount} flagged
          </Badge>

          {/* Deep signal — only after a scan has run. */}
          {deepFlaggedCount !== null && (
            <Badge
              variant={deepFlaggedCount > 0 ? "destructive" : "outline"}
              className="cursor-pointer"
              onClick={onOpenFindings}
              title="Tenants flagged by the deep Control Plane analysis"
              data-testid="analysis-deep-flagged"
            >
              {deepFlaggedCount} flagged (deep)
            </Badge>
          )}

          <Button
            variant={showOnlyFlagged ? "default" : "outline"}
            size="sm"
            onClick={() => onToggleFlagged(!showOnlyFlagged)}
            data-testid="analysis-toggle-flagged"
            aria-pressed={showOnlyFlagged}
          >
            <Filter className="mr-2 h-4 w-4" />
            {showOnlyFlagged ? "Showing flagged" : "Show only flagged"}
          </Button>

          {totalFindings > 0 && (
            <Button
              variant="outline"
              size="sm"
              onClick={onOpenFindings}
              data-testid="analysis-open-findings"
            >
              View findings ({totalFindings})
            </Button>
          )}
        </div>
      </div>

      {/* Graceful-degrade notice — never a crashed page (§6). */}
      {unavailable && (
        <div
          className={cn(
            "flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm"
          )}
          data-testid="analysis-unavailable"
        >
          <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5 text-yellow-600" />
          <div>
            <p className="font-medium">Deep analysis unavailable</p>
            <p className="text-muted-foreground text-xs">
              {unavailableMessage ||
                "The Control Plane /analyze endpoint could not be reached. The instant per-row checks below still work."}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
