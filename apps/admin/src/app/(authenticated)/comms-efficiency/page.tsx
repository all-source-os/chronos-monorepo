"use client";

import {
  Badge,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, Eye, MousePointerClick, RefreshCw, Target, TrendingUp } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { StatCard } from "@/components/monitoring/stat-card";
import {
  type EfficiencyGroup,
  type EfficiencyProjection,
  fetchCommsEfficiency,
  humanizeSeconds,
  pct,
  signedPct,
  stageLabel,
} from "@/lib/comms-efficiency-api";

export default function CommsEfficiencyPage() {
  const [data, setData] = useState<EfficiencyProjection | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (refresh = false) => {
    try {
      if (refresh) setRefreshing(true);
      const proj = await fetchCommsEfficiency(refresh);
      setData(proj);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load efficiency");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const hero = data?.hero;
  const groups = data?.groups ?? [];

  return (
    <div className="space-y-6" data-testid="comms-efficiency-page">
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Comms Efficiency</h1>
          <p className="text-sm text-muted-foreground">
            Did the messages we sent drive real downstream behavior? Every figure is a temporal join
            over the tenant&apos;s own Core event stream — no external analytics.
          </p>
        </div>
        <button
          type="button"
          onClick={() => void load(true)}
          disabled={refreshing}
          className="inline-flex items-center gap-2 rounded-lg border border-border px-3 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50"
        >
          <RefreshCw className={cn("h-4 w-4", refreshing && "animate-spin")} />
          Recompute
        </button>
      </div>

      {error && (
        <Card className="border-red-500/40">
          <CardContent className="flex items-center gap-2 p-4 text-sm text-red-500">
            <AlertTriangle className="h-4 w-4" /> {error}
          </CardContent>
        </Card>
      )}

      {/* Honesty caveat — opens are unreliable (Apple MPP). */}
      <div className="flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-3 text-xs text-muted-foreground">
        <Eye className="mt-0.5 h-4 w-4 shrink-0 text-yellow-500" />
        <p>
          <span className="font-semibold text-foreground">Open-rate is unreliable.</span> Apple Mail
          Privacy Protection pre-fetches images, inflating opens. This panel leads with{" "}
          <span className="font-semibold text-foreground">clicks, conversion, and causal lift</span>
          ; opens are shown muted and never optimized on.
        </p>
      </div>

      {/* HERO — trial → paid is the number that funds the company. */}
      <Card className="border-primary/30">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Target className="h-5 w-5 text-primary" /> Trial → Paid (hero metric)
          </CardTitle>
          <CardDescription>
            subscription.activated within the attribution window. The free tier is retired — there
            is no free segment.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4 md:grid-cols-4">
            <HeroFigure
              label="Conversion (sent)"
              value={hero ? pct(hero.conversion_rate) : "—"}
              sub={hero ? `${hero.converted} of ${hero.sent} sent` : ""}
              emphasis
            />
            <HeroFigure
              label="Causal lift vs holdout"
              value={hero?.has_holdout ? signedPct(hero.lift) : "no holdout"}
              sub={
                hero?.has_holdout
                  ? `holdout ${pct(hero.holdout_conversion_rate)} (${hero.holdout_converted}/${hero.held_out})`
                  : "set a holdout % to measure lift"
              }
              emphasis
            />
            <HeroFigure
              label="Clicked"
              value={String(hero?.clicked ?? 0)}
              sub={hero ? `of ${hero.delivered} delivered` : ""}
            />
            <HeroFigure
              label="Median time-to-paid"
              value={hero ? humanizeSeconds(hero.time_to_goal_median_sec) : "—"}
              sub="send → activation"
            />
          </div>
        </CardContent>
      </Card>

      {/* Quick stat cards — clicks + conversion lead. */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <StatCard
          title="Campaigns measured"
          value={String(groups.length)}
          subtitle="campaign / stage / variant / tier rows"
          icon={TrendingUp}
        />
        <StatCard
          title="Total clicks"
          value={String(groups.reduce((a, g) => a + g.clicked, 0))}
          subtitle="lead engagement signal"
          icon={MousePointerClick}
        />
        <StatCard
          title="Total conversions"
          value={String(groups.reduce((a, g) => a + g.converted, 0))}
          subtitle="goal events in-window (last-touch)"
          icon={Target}
        />
      </div>

      {/* Funnel table */}
      <Card>
        <CardHeader>
          <CardTitle>Funnel by campaign / stage / variant / tier</CardTitle>
          <CardDescription>
            Delivered → Clicked → Goal-in-window, with causal lift, time-to-goal, and unsub /
            complaint cost. Opens are shown muted (see caveat above).
          </CardDescription>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          {groups.length === 0 ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              No comms-efficiency data yet. Once a campaign sends (and engagement webhooks arrive),
              its funnel appears here.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Campaign / Stage</TableHead>
                  <TableHead>Variant</TableHead>
                  <TableHead>Tier</TableHead>
                  <TableHead className="text-right">Sent</TableHead>
                  <TableHead className="text-right">Delivered</TableHead>
                  <TableHead className="text-right font-semibold">Clicked</TableHead>
                  <TableHead className="text-right font-semibold">Click %</TableHead>
                  <TableHead className="text-right font-semibold">Goal</TableHead>
                  <TableHead className="text-right font-semibold">Conv %</TableHead>
                  <TableHead className="text-right font-semibold">Lift</TableHead>
                  <TableHead className="text-right">TTG</TableHead>
                  <TableHead className="text-right text-muted-foreground">Open %</TableHead>
                  <TableHead className="text-right">Unsub %</TableHead>
                  <TableHead className="text-right">Spam %</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {groups.map((g) => (
                  <FunnelRow key={`${g.campaign}|${g.stage}|${g.variant}|${g.tier}`} g={g} />
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Goal legend — real-today vs needs-new-signal honesty. */}
      {data?.goal_legend && data.goal_legend.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Goal signals</CardTitle>
            <CardDescription>
              Which lifecycle goals fire as real Core events today vs still need a new signal.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {data.goal_legend.map((entry) => (
              <div key={entry.stage} className="flex items-start gap-2 text-sm">
                <Badge variant={entry.state === "real" ? "secondary" : "outline"}>
                  {entry.state === "real" ? "real" : "needs signal"}
                </Badge>
                <div>
                  <span className="font-medium">{stageLabel(entry.stage)}</span>{" "}
                  <span className="text-muted-foreground">→ {entry.goal_event || "any event"}</span>
                  {entry.note && <p className="text-xs text-muted-foreground">{entry.note}</p>}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Methodology notes */}
      {data?.notes && data.notes.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">How to read this</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="list-disc space-y-1 pl-5 text-xs text-muted-foreground">
              {data.notes.map((n) => (
                <li key={n}>{n}</li>
              ))}
            </ul>
            {data.generated_at && (
              <p className="mt-3 text-xs text-muted-foreground">
                Generated {new Date(data.generated_at).toLocaleString()}
              </p>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function HeroFigure({
  label,
  value,
  sub,
  emphasis,
}: {
  label: string;
  value: string;
  sub?: string;
  emphasis?: boolean;
}) {
  return (
    <div className="space-y-1">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{label}</p>
      <p className={cn("font-bold tabular-nums", emphasis ? "text-3xl text-primary" : "text-2xl")}>
        {value}
      </p>
      {sub && <p className="text-xs text-muted-foreground">{sub}</p>}
    </div>
  );
}

function FunnelRow({ g }: { g: EfficiencyGroup }) {
  return (
    <TableRow>
      <TableCell>
        <div className="font-medium">{g.campaign || "(untagged)"}</div>
        <div className="text-xs text-muted-foreground">{stageLabel(g.stage)}</div>
      </TableCell>
      <TableCell>{g.variant || "—"}</TableCell>
      <TableCell>
        <Badge variant="outline">{g.tier || "—"}</Badge>
      </TableCell>
      <TableCell className="text-right tabular-nums">{g.sent}</TableCell>
      <TableCell className="text-right tabular-nums">{g.delivered}</TableCell>
      <TableCell className="text-right font-semibold tabular-nums">{g.clicked}</TableCell>
      <TableCell className="text-right font-semibold tabular-nums">{pct(g.click_rate)}</TableCell>
      <TableCell className="text-right font-semibold tabular-nums">
        <span className="inline-flex items-center gap-1">
          {g.converted}
          {g.goal_state === "needs_signal" && (
            <AlertTriangle
              className="h-3 w-3 text-yellow-500"
              aria-label="goal needs a new signal"
            />
          )}
          {g.churned > 0 && (
            <span className="text-xs text-muted-foreground">(+{g.churned} churned)</span>
          )}
        </span>
      </TableCell>
      <TableCell className="text-right font-semibold tabular-nums">
        {pct(g.conversion_rate)}
      </TableCell>
      <TableCell
        className={cn(
          "text-right font-semibold tabular-nums",
          g.held_out > 0
            ? g.lift > 0
              ? "text-green-500"
              : g.lift < 0
                ? "text-red-500"
                : ""
            : "text-muted-foreground"
        )}
      >
        {g.held_out > 0 ? signedPct(g.lift) : "—"}
      </TableCell>
      <TableCell className="text-right tabular-nums">
        {humanizeSeconds(g.time_to_goal_median_sec)}
      </TableCell>
      {/* Open % — visually subordinated (muted) because MPP inflates it. */}
      <TableCell className="text-right tabular-nums text-muted-foreground">
        {pct(g.open_rate)}
      </TableCell>
      <TableCell className={cn("text-right tabular-nums", g.unsub_rate > 0 && "text-yellow-600")}>
        {pct(g.unsub_rate)}
      </TableCell>
      <TableCell className={cn("text-right tabular-nums", g.complaint_rate > 0 && "text-red-500")}>
        {pct(g.complaint_rate)}
      </TableCell>
    </TableRow>
  );
}
