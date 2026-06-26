"use client";

import {
  Badge,
  Button,
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { ArrowRight, Terminal } from "lucide-react";
import Link from "next/link";
import { useMemo } from "react";
import type {
  AnalysisCategory,
  AnalysisFinding,
  AnalysisSeverity,
  SuggestedAction,
} from "@/lib/analysis-api";
import { categoryLabel, severityColour, severityLabel, severityRank } from "@/lib/analysis-api";

/**
 * FindingsDrawer — the grouped findings panel (prompt 047 requirement 3).
 *
 *   - fleet_findings render at the TOP (they affect many tenants),
 *   - per-tenant findings are grouped by category, sorted worst-severity first,
 *   - every finding shows title + detail + a "Fix →" link built from
 *     suggested_action: a navigable admin route → a <Link>; an API route or a
 *     "task" command → a read-only INSTRUCTION (NEVER auto-executed, verification
 *     #5). This UI introduces no mutation.
 *
 * Merges the instant client findings with the deep findings so the operator sees
 * one list. Presentation only — the page owns fetch + state.
 */

/** A per-tenant finding the drawer renders, tagged with its tenant for grouping. */
export interface TenantFindingRow {
  tenantId: string;
  tenantName: string;
  finding: AnalysisFinding;
  /** "client" (instant heuristic) or "deep" (CP /analyze). */
  source: "client" | "deep";
}

interface FindingsDrawerProps {
  open: boolean;
  onClose: () => void;
  fleetFindings: AnalysisFinding[];
  tenantFindings: TenantFindingRow[];
  generatedAt?: string;
}

const CATEGORY_ORDER: AnalysisCategory[] = [
  "data_integrity",
  "plan_billing",
  "usage_health",
  "litter",
];

/**
 * Decide how to render a suggested action's target. Navigable admin pages (a
 * non-API path) become a clickable link; an /api/* route or a task command is an
 * instruction only — operators cannot (and must not) trigger a POST by clicking.
 */
function actionKindFor(action: SuggestedAction): "link" | "instruction" {
  if (action.kind === "task") return "instruction";
  // kind === "link": only same-origin admin PAGE routes are safely navigable.
  const t = action.target;
  if (t.startsWith("/") && !t.startsWith("/api/")) return "link";
  return "instruction";
}

function FixAction({ action }: { action?: SuggestedAction }) {
  if (!action) return null;
  const mode = actionKindFor(action);

  if (mode === "link") {
    return (
      <Link
        href={action.target}
        className="inline-flex items-center gap-1 text-xs font-medium text-primary hover:underline"
        data-testid="finding-fix-link"
      >
        {action.label || "Fix"}
        <ArrowRight className="h-3 w-3" />
      </Link>
    );
  }

  // Instruction: show the existing route/command as copyable text — read-only.
  const isTask = action.kind === "task";
  return (
    <div className="space-y-1" data-testid="finding-fix-instruction">
      <span className="inline-flex items-center gap-1 text-xs font-medium text-muted-foreground">
        {isTask && <Terminal className="h-3 w-3" />}
        {action.label || (isTask ? "Run" : "Fix")}
      </span>
      <code className="block w-full overflow-x-auto rounded bg-muted px-2 py-1 text-[11px]">
        {action.target}
      </code>
    </div>
  );
}

function SeverityDot({ severity }: { severity: AnalysisSeverity }) {
  const { dot } = severityColour(severity);
  return <span className={cn("mt-1 h-2 w-2 shrink-0 rounded-full", dot)} />;
}

function FindingItem({
  finding,
  tenantName,
  source,
}: {
  finding: AnalysisFinding;
  tenantName?: string;
  source?: "client" | "deep";
}) {
  return (
    <li
      className="flex items-start gap-2 rounded-lg border p-3"
      data-testid={`finding-item-${finding.code}`}
    >
      <SeverityDot severity={finding.severity} />
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium">{finding.title}</span>
          <Badge variant={severityColour(finding.severity).badge} className="text-[10px]">
            {severityLabel(finding.severity)}
          </Badge>
          {tenantName && <span className="text-xs text-muted-foreground">· {tenantName}</span>}
          {typeof finding.affected_count === "number" && finding.affected_count > 0 && (
            <span className="text-xs text-muted-foreground">
              · {finding.affected_count} affected
            </span>
          )}
          {source === "client" && (
            <Badge variant="outline" className="text-[10px]">
              instant
            </Badge>
          )}
          <code className="ml-auto text-[10px] text-muted-foreground">{finding.code}</code>
        </div>
        <p className="text-xs text-muted-foreground">{finding.detail}</p>
        <FixAction action={finding.suggested_action} />
      </div>
    </li>
  );
}

export function FindingsDrawer({
  open,
  onClose,
  fleetFindings,
  tenantFindings,
  generatedAt,
}: FindingsDrawerProps) {
  // Group per-tenant findings by category, each sorted worst-severity first.
  const grouped = useMemo(() => {
    const byCat = new Map<AnalysisCategory, TenantFindingRow[]>();
    for (const row of tenantFindings) {
      const cat = row.finding.category;
      const list = byCat.get(cat) ?? [];
      list.push(row);
      byCat.set(cat, list);
    }
    for (const list of byCat.values()) {
      list.sort((a, b) => severityRank(b.finding.severity) - severityRank(a.finding.severity));
    }
    return byCat;
  }, [tenantFindings]);

  const sortedFleet = useMemo(
    () => [...fleetFindings].sort((a, b) => severityRank(b.severity) - severityRank(a.severity)),
    [fleetFindings]
  );

  const totalCount = fleetFindings.length + tenantFindings.length;

  return (
    <Drawer open={open} onOpenChange={(o) => !o && onClose()}>
      <DrawerContent
        className="max-h-[85vh]"
        data-testid="findings-drawer"
        aria-describedby="findings-drawer-desc"
      >
        <DrawerHeader className="text-left">
          <DrawerTitle className="flex items-center gap-2">
            Tenant analysis findings
            <Badge variant="secondary">{totalCount}</Badge>
          </DrawerTitle>
          <DrawerDescription id="findings-drawer-desc">
            Read-only anomaly findings. Each “Fix →” links to an existing guarded action — nothing
            here mutates data.
            {generatedAt ? ` Deep scan generated ${generatedAt}.` : ""}
          </DrawerDescription>
        </DrawerHeader>

        <div className="overflow-y-auto px-4 pb-8 space-y-6">
          {totalCount === 0 ? (
            <p
              className="py-8 text-center text-sm text-muted-foreground"
              data-testid="findings-empty"
            >
              No findings. Run a scan or adjust filters.
            </p>
          ) : (
            <>
              {/* Fleet-wide findings first. */}
              {sortedFleet.length > 0 && (
                <section data-testid="findings-fleet">
                  <h3 className="mb-2 text-sm font-semibold">
                    Fleet-wide <span className="text-muted-foreground">({sortedFleet.length})</span>
                  </h3>
                  <ul className="space-y-2">
                    {sortedFleet.map((f) => (
                      <FindingItem key={`fleet-${f.category}-${f.code}`} finding={f} />
                    ))}
                  </ul>
                </section>
              )}

              {/* Per-tenant findings grouped by category. */}
              {CATEGORY_ORDER.map((cat) => {
                const rows = grouped.get(cat);
                if (!rows || rows.length === 0) return null;
                return (
                  <section key={cat} data-testid={`findings-category-${cat}`}>
                    <h3 className="mb-2 text-sm font-semibold">
                      {categoryLabel(cat)}{" "}
                      <span className="text-muted-foreground">({rows.length})</span>
                    </h3>
                    <ul className="space-y-2">
                      {rows.map((row) => (
                        <FindingItem
                          key={`${row.tenantId}-${row.finding.code}`}
                          finding={row.finding}
                          tenantName={row.tenantName}
                          source={row.source}
                        />
                      ))}
                    </ul>
                  </section>
                );
              })}
            </>
          )}

          <div className="flex justify-end">
            <Button variant="outline" onClick={onClose} data-testid="findings-close">
              Close
            </Button>
          </div>
        </div>
      </DrawerContent>
    </Drawer>
  );
}
