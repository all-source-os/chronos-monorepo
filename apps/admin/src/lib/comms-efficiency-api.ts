/**
 * Proactive-comms efficiency API client (prompt 050).
 *
 * Thin consumer of the Control Plane efficiency endpoint. This client performs NO
 * scoring — it renders the funnel/lift the Control Plane reconciler computes by
 * joining engagement events ⋈ goal events in Core (no parallel analytics stack).
 * Mirrors fleet-api.ts: fetch(url, { credentials: "include" }), typed interfaces,
 * getApiUrl() falling back to NEXT_PUBLIC_API_URL.
 *
 * Endpoint (Control Plane admin group, gated by AdminAuthMiddleware):
 *   GET /api/v1/admin/comms/efficiency[?refresh=true]
 */

function getApiUrl(): string {
  if (typeof window !== "undefined") {
    // Client-side: hit the same-origin BFF proxy (src/app/api/v1/[...path]/route.ts),
    // which attaches the admin_token Bearer and forwards to the Control Plane.
    return "";
  }
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
}

/** Whether a stage's goal signal fires today or still needs a new signal. */
export type GoalState = "real" | "needs_signal";

/** Trial→paid headline funnel — THE hero metric (subscription.activated in-window). */
export interface TrialToPaidHero {
  goal_event: string;
  sent: number;
  held_out: number;
  delivered: number;
  clicked: number;
  converted: number;
  holdout_converted: number;
  conversion_rate: number; // converted / sent (intent-to-treat)
  holdout_conversion_rate: number; // holdout_converted / held_out
  lift: number; // conversion_rate − holdout_rate (causal)
  time_to_goal_median_sec: number;
  has_holdout: boolean;
}

/** One campaign/stage/variant/tier funnel row. */
export interface EfficiencyGroup {
  campaign: string;
  stage: string;
  variant: string;
  tier: string;
  goal_event: string;
  goal_state: GoalState;
  goal_note?: string;
  window_days: number;

  sent: number;
  held_out: number;
  delivered: number;
  opened: number;
  clicked: number;
  bounced: number;
  unsubscribed: number;
  complained: number;
  churned: number;

  open_rate: number; // UNRELIABLE (Apple MPP) — subordinated in the UI
  click_rate: number; // LEAD signal
  converted: number;
  holdout_converted: number;
  conversion_rate: number; // converted / delivered
  convert_sent: number; // converted / sent
  convert_holdout: number; // holdout_converted / held_out
  lift: number; // convert_sent − convert_holdout (causal)
  time_to_goal_median_sec: number;
  unsub_rate: number;
  complaint_rate: number;
}

/** Per-stage goal legend: is the goal signal real today or does it need a signal? */
export interface GoalLegendEntry {
  stage: string;
  goal_event: string;
  state: GoalState;
  note?: string;
}

/** The operator-side projection the panel renders. */
export interface EfficiencyProjection {
  generated_at: string;
  hero: TrialToPaidHero;
  groups: EfficiencyGroup[];
  notes: string[];
  goal_legend: GoalLegendEntry[];
}

export async function fetchCommsEfficiency(refresh = false): Promise<EfficiencyProjection> {
  const qs = refresh ? "?refresh=true" : "";
  const url = `${getApiUrl()}/api/v1/admin/comms/efficiency${qs}`;
  const res = await fetch(url, { credentials: "include" });
  if (!res.ok) {
    throw new Error(`Failed to fetch comms efficiency: ${res.status}`);
  }
  return res.json();
}

// ── Presentation helpers (display only — never scoring) ─────────────────

/** Format a 0–1 ratio as a percentage string. */
export function pct(ratio: number): string {
  if (!Number.isFinite(ratio)) return "—";
  return `${(ratio * 100).toFixed(1)}%`;
}

/** Format a signed lift (0–1) with an explicit + / − so causal direction reads. */
export function signedPct(ratio: number): string {
  if (!Number.isFinite(ratio)) return "—";
  const v = ratio * 100;
  const sign = v > 0 ? "+" : "";
  return `${sign}${v.toFixed(1)}pp`;
}

/** Human time-to-goal from seconds (median). */
export function humanizeSeconds(sec: number): string {
  if (!sec || sec <= 0) return "—";
  const d = Math.floor(sec / 86400);
  if (d >= 1) return `${d}d`;
  const h = Math.floor(sec / 3600);
  if (h >= 1) return `${h}h`;
  const m = Math.floor(sec / 60);
  return `${m}m`;
}

/** A human label for a lifecycle stage (display only). */
export function stageLabel(stage: string): string {
  if (!stage) return "(untagged)";
  return stage
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}
