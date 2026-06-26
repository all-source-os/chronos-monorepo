"use client";

import { Badge } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import type { AnalysisSeverity } from "@/lib/analysis-api";
import { severityColour, severityLabel } from "@/lib/analysis-api";

/**
 * AnomalyBadge — a per-row severity dot + finding count for the /tenants table.
 *
 * Uses the SAME dot+colour vocabulary as HealthChip (health-chip.tsx):
 *   red → critical · yellow → warn · blue → info · green → ok
 * Renders a compact "● N" pill; the `title` (hover) lists the finding titles so an
 * operator can see WHY a row is flagged without expanding. Presentation only.
 */

interface AnomalyBadgeProps {
  /** The row's worst severity (client heuristic ∪ deep finding). */
  severity: AnalysisSeverity;
  /** Total number of findings on this tenant. */
  count: number;
  /** Finding titles for the hover tooltip. */
  titles?: string[];
  className?: string;
  onClick?: () => void;
}

export function AnomalyBadge({ severity, count, titles, className, onClick }: AnomalyBadgeProps) {
  if (count <= 0) return null;
  const { dot, badge } = severityColour(severity);
  const tooltip =
    titles && titles.length > 0
      ? titles.join("\n")
      : `${count} ${severityLabel(severity)} finding${count === 1 ? "" : "s"}`;

  return (
    <Badge
      variant={badge}
      className={cn("inline-flex items-center gap-1.5", onClick && "cursor-pointer", className)}
      title={tooltip}
      onClick={
        onClick
          ? (e: React.MouseEvent) => {
              e.stopPropagation();
              onClick();
            }
          : undefined
      }
      data-testid={`anomaly-badge-${severity}`}
    >
      <span className={cn("h-2 w-2 rounded-full", dot)} />
      {count}
    </Badge>
  );
}
