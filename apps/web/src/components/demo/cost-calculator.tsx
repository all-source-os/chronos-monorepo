"use client";

import { Card, CardContent } from "@allsource/ui";
import { useEffect, useState, useCallback } from "react";
import { Calculator, DollarSign, Zap, ArrowDown, Layers, Scissors, Package } from "lucide-react";

interface CostFormulas {
  raw_cost_per_million_tokens_usd: number;
  mcp_cost_per_million_tokens_usd: number;
  tokens_per_event_raw: number;
  tokens_per_event_mcp: number;
  vectors_cost_multiplier: number;
}

interface BenchmarkConfig {
  cost_formulas: CostFormulas;
}

// Logarithmic scale helpers: map slider 0-100 to 1M-100M
function sliderToEventsPerDay(value: number): number {
  // 0 → 1M, 100 → 100M (log scale)
  const minLog = Math.log10(1_000_000);
  const maxLog = Math.log10(100_000_000);
  return Math.round(Math.pow(10, minLog + (value / 100) * (maxLog - minLog)));
}

function eventsPerDayToSlider(events: number): number {
  const minLog = Math.log10(1_000_000);
  const maxLog = Math.log10(100_000_000);
  return ((Math.log10(events) - minLog) / (maxLog - minLog)) * 100;
}

// Logarithmic scale: map slider 0-100 to 1-10k
function sliderToVectorsPerEvent(value: number): number {
  const minLog = Math.log10(1);
  const maxLog = Math.log10(10_000);
  return Math.round(Math.pow(10, minLog + (value / 100) * (maxLog - minLog)));
}

function formatNumber(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function formatCurrency(n: number): string {
  if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `$${(n / 1_000).toFixed(1)}K`;
  return `$${n.toFixed(2)}`;
}

export function CostCalculator() {
  const [eventsSlider, setEventsSlider] = useState(50); // middle = ~10M
  const [vectorsSlider, setVectorsSlider] = useState(25); // ~5-6 vectors
  const [formulas, setFormulas] = useState<CostFormulas | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
    fetch(`${apiUrl}/api/v1/config/benchmarks`)
      .then((res) => res.json())
      .then((data: BenchmarkConfig) => {
        setFormulas(data.cost_formulas);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const eventsPerDay = sliderToEventsPerDay(eventsSlider);
  const vectorsPerEvent = sliderToVectorsPerEvent(vectorsSlider);

  const calculate = useCallback(
    (f: CostFormulas) => {
      const eventsPerMonth = eventsPerDay * 30;
      const tokensRaw = eventsPerMonth * f.tokens_per_event_raw;
      const tokensMcp = eventsPerMonth * f.tokens_per_event_mcp;
      const vectorMultiplier = 1 + (vectorsPerEvent - 1) * (f.vectors_cost_multiplier - 1) / 9999;

      const rawCost = (tokensRaw / 1_000_000) * f.raw_cost_per_million_tokens_usd * vectorMultiplier;
      const mcpCost = (tokensMcp / 1_000_000) * f.mcp_cost_per_million_tokens_usd * vectorMultiplier;
      const savings = rawCost > 0 ? Math.round(((rawCost - mcpCost) / rawCost) * 100) : 0;

      return { rawCost, mcpCost, savings };
    },
    [eventsPerDay, vectorsPerEvent]
  );

  const costs = formulas ? calculate(formulas) : null;

  if (loading) {
    return (
      <Card data-testid="cost-calculator">
        <CardContent className="flex items-center justify-center p-8">
          <p className="text-sm text-muted-foreground">Loading cost calculator...</p>
        </CardContent>
      </Card>
    );
  }

  if (!formulas) {
    return null;
  }

  return (
    <Card data-testid="cost-calculator" role="region" aria-label="Cost calculator">
      <CardContent className="p-6">
        <div className="mb-4 flex items-center gap-2">
          <Calculator className="h-5 w-5 text-primary" aria-hidden="true" />
          <h3 className="text-lg font-semibold">Cost Calculator</h3>
          <span className="ml-auto text-xs text-muted-foreground">
            Estimates based on token pricing
          </span>
        </div>

        <div className="grid gap-6 lg:grid-cols-[1fr_1fr_1.5fr]">
          {/* Sliders */}
          <div className="space-y-6">
            {/* Events/day slider */}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <label htmlFor="events-slider" className="text-sm font-medium">
                  Events / day
                </label>
                <span
                  className="text-sm font-bold tabular-nums text-primary"
                  data-testid="events-per-day-value"
                >
                  {formatNumber(eventsPerDay)}
                </span>
              </div>
              <input
                id="events-slider"
                type="range"
                min={0}
                max={100}
                step={1}
                value={eventsSlider}
                onChange={(e) => setEventsSlider(Number(e.target.value))}
                className="h-2 w-full cursor-pointer appearance-none rounded-lg bg-muted accent-primary"
                aria-label="Events per day"
                data-testid="events-slider"
              />
              <div className="mt-1 flex justify-between text-xs text-muted-foreground">
                <span>1M</span>
                <span>100M</span>
              </div>
            </div>

            {/* Vectors/event slider */}
            <div>
              <div className="mb-2 flex items-center justify-between">
                <label htmlFor="vectors-slider" className="text-sm font-medium">
                  Vectors / event
                </label>
                <span
                  className="text-sm font-bold tabular-nums text-primary"
                  data-testid="vectors-per-event-value"
                >
                  {formatNumber(vectorsPerEvent)}
                </span>
              </div>
              <input
                id="vectors-slider"
                type="range"
                min={0}
                max={100}
                step={1}
                value={vectorsSlider}
                onChange={(e) => setVectorsSlider(Number(e.target.value))}
                className="h-2 w-full cursor-pointer appearance-none rounded-lg bg-muted accent-primary"
                aria-label="Vectors per event"
                data-testid="vectors-slider"
              />
              <div className="mt-1 flex justify-between text-xs text-muted-foreground">
                <span>1</span>
                <span>10K</span>
              </div>
            </div>
          </div>

          {/* Cost output cards */}
          <div className="flex flex-col gap-3" aria-live="polite" aria-atomic="true">
            <div className="rounded-lg bg-orange-500/10 p-4" data-testid="raw-cost-card">
              <div className="mb-1 flex items-center gap-1.5 text-orange-500">
                <DollarSign className="h-4 w-4" />
                <span className="text-xs font-medium">Raw Mode</span>
              </div>
              <p className="text-2xl font-bold tabular-nums" data-testid="raw-cost-value">
                {costs ? formatCurrency(costs.rawCost) : "—"}
              </p>
              <p className="text-xs text-muted-foreground">per month (tokens)</p>
            </div>

            <div className="rounded-lg bg-green-500/10 p-4" data-testid="mcp-cost-card">
              <div className="mb-1 flex items-center gap-1.5 text-green-500">
                <DollarSign className="h-4 w-4" />
                <span className="text-xs font-medium">MCP Mode</span>
              </div>
              <p className="text-2xl font-bold tabular-nums" data-testid="mcp-cost-value">
                {costs ? formatCurrency(costs.mcpCost) : "—"}
              </p>
              <p className="text-xs text-muted-foreground">per month (tokens)</p>
            </div>

            {costs && (
              <div
                className="flex items-center justify-center gap-2 rounded-lg bg-green-500/20 p-3"
                data-testid="savings-badge"
              >
                <ArrowDown className="h-4 w-4 text-green-500" />
                <span className="text-lg font-bold text-green-500" data-testid="savings-percentage">
                  {costs.savings}% savings
                </span>
              </div>
            )}
          </div>

          {/* Explainer bullet points */}
          <div className="space-y-3">
            <h4 className="text-sm font-medium text-muted-foreground">Why MCP saves</h4>
            <div className="space-y-2.5">
              <div className="flex items-start gap-2.5">
                <Package className="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
                <div>
                  <p className="text-sm font-medium">Smart Caching</p>
                  <p className="text-xs text-muted-foreground">
                    MCP caches repeated query patterns — identical queries skip tokenization entirely.
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-2.5">
                <Scissors className="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
                <div>
                  <p className="text-sm font-medium">Context Pruning</p>
                  <p className="text-xs text-muted-foreground">
                    Strips irrelevant context before sending to the model — 10x fewer tokens per event.
                  </p>
                </div>
              </div>
              <div className="flex items-start gap-2.5">
                <Layers className="mt-0.5 h-4 w-4 shrink-0 text-green-500" />
                <div>
                  <p className="text-sm font-medium">Batch Routing</p>
                  <p className="text-xs text-muted-foreground">
                    Groups similar events into batched requests — reduces per-event overhead dramatically.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
