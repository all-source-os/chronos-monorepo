import { Check, Minus, X } from "lucide-react";
import type { Cell, ComparisonRow } from "../_data/competitors";

/**
 * Renders a single comparison cell. Booleans become check/cross/partial icons;
 * "unknown"/"varies" and any other string render verbatim so we never imply a
 * value we cannot defend.
 */
function CellValue({ value }: { value: Cell }) {
  if (value === "yes") {
    return <Check className="mx-auto h-4 w-4 text-primary" aria-label="Yes" />;
  }
  if (value === "no") {
    return <X className="mx-auto h-4 w-4 text-muted-foreground/40" aria-label="No" />;
  }
  if (value === "partial") {
    return <Minus className="mx-auto h-4 w-4 text-muted-foreground" aria-label="Partial" />;
  }
  return <span className="text-sm font-medium text-foreground">{value}</span>;
}

/**
 * Shared comparison table for every `/vs/<slug>` page. DRY: the rows come from
 * the per-competitor data map, so all three pages stay in lockstep with the
 * homepage matrix and product facts.
 */
export function ComparisonTable({
  competitorName,
  rows,
}: {
  competitorName: string;
  rows: ComparisonRow[];
}) {
  return (
    <div className="mx-auto max-w-4xl overflow-hidden rounded-2xl border border-border bg-background/50">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/40">
              <th className="px-6 py-4 text-left font-semibold">Capability</th>
              <th className="px-6 py-4 text-center font-semibold text-primary">AllSource</th>
              <th className="px-6 py-4 text-center font-semibold text-muted-foreground">
                {competitorName}
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-border/50">
            {rows.map((row) => (
              <tr key={row.feature} className="align-top hover:bg-muted/20">
                <td className="px-6 py-4">
                  <div className="font-medium text-foreground">{row.feature}</div>
                  {row.note && <div className="mt-1 text-xs text-muted-foreground">{row.note}</div>}
                </td>
                <td className="px-6 py-4 text-center">
                  <CellValue value={row.allsource} />
                </td>
                <td className="px-6 py-4 text-center">
                  <CellValue value={row.competitor} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
