"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { useCallback, useEffect, useRef, useState } from "react";
import { Play, Square, Zap, Clock, DollarSign, Hash, Trophy, RotateCcw } from "lucide-react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";

interface SideMetrics {
  tokensUsed: number;
  latencyMs: number;
  costPerMillion: number;
  eventsProcessed: number;
}

interface LatencyDataPoint {
  time: number; // seconds elapsed
  raw: number;
  mcp: number;
}

const INITIAL_METRICS: SideMetrics = {
  tokensUsed: 0,
  latencyMs: 0,
  costPerMillion: 0,
  eventsProcessed: 0,
};

// Simulated test parameters
const TEST_DURATION_MS = 30_000; // 30s test
const TICK_INTERVAL_MS = 200; // update every 200ms
const EVENTS_PER_TICK = 2000; // 10k/sec = 2k per 200ms tick
// Record a chart point every 500ms to keep chart readable
const CHART_SAMPLE_INTERVAL_MS = 500;

// Speed Race queries
const RACE_QUERIES = [
  "Find 'cat videos' last week",
  "Error 500 in api-gateway today",
  "User signups past 24 hours",
  "High latency events p99 > 200ms",
  "Failed login attempts this week",
];

export function McpShowdownPanel() {
  const [running, setRunning] = useState(false);
  const [completed, setCompleted] = useState(false);
  const [rawMetrics, setRawMetrics] = useState<SideMetrics>(INITIAL_METRICS);
  const [mcpMetrics, setMcpMetrics] = useState<SideMetrics>(INITIAL_METRICS);
  const [latencyHistory, setLatencyHistory] = useState<LatencyDataPoint[]>([]);
  const [elapsedSec, setElapsedSec] = useState(0);
  // Speed Race state
  const [raceRunning, setRaceRunning] = useState(false);
  const [raceComplete, setRaceComplete] = useState(false);
  const [raceQuery, setRaceQuery] = useState(RACE_QUERIES[0]!);
  const [rawProgress, setRawProgress] = useState(0);
  const [mcpProgress, setMcpProgress] = useState(0);
  const [rawRaceLatency, setRawRaceLatency] = useState(0);
  const [mcpRaceLatency, setMcpRaceLatency] = useState(0);
  const raceAnimRef = useRef<ReturnType<typeof requestAnimationFrame> | undefined>(undefined);

  const intervalRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);
  const startTimeRef = useRef(0);
  const lastChartSampleRef = useRef(0);

  const resetTest = useCallback(() => {
    setRawMetrics(INITIAL_METRICS);
    setMcpMetrics(INITIAL_METRICS);
    setLatencyHistory([]);
    setElapsedSec(0);
    setCompleted(false);
  }, []);

  const stopTest = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = undefined;
    }
    setRunning(false);
    setCompleted(true);
  }, []);

  const startTest = useCallback(() => {
    resetTest();
    setRunning(true);
    startTimeRef.current = Date.now();
    lastChartSampleRef.current = 0;

    let rawTotal = 0;
    let mcpTotal = 0;

    intervalRef.current = setInterval(() => {
      const elapsed = Date.now() - startTimeRef.current;

      if (elapsed >= TEST_DURATION_MS) {
        clearInterval(intervalRef.current);
        intervalRef.current = undefined;
        setRunning(false);
        setCompleted(true);
        return;
      }

      rawTotal += EVENTS_PER_TICK;
      mcpTotal += EVENTS_PER_TICK;

      // Raw mode: higher tokens, higher latency with spikes, higher cost
      const loadFactor = Math.min(elapsed / 10_000, 1); // ramp up over 10s
      const spike = Math.random() > 0.85 ? Math.random() * 80 : 0; // occasional spikes
      const rawLatency = Math.round(85 + Math.random() * 40 + loadFactor * 30 + spike);
      const rawTokensPer = 150;
      const rawCostRate = 3.0;

      // MCP mode: lower tokens, flat latency, lower cost
      const mcpLatency = Math.round(12 + Math.random() * 12);
      const mcpTokensPer = 15;
      const mcpCostRate = 0.3;

      const elapsedSeconds = Math.round(elapsed / 1000);
      setElapsedSec(elapsedSeconds);

      setRawMetrics({
        tokensUsed: rawTotal * rawTokensPer,
        latencyMs: rawLatency,
        costPerMillion: rawCostRate * rawTokensPer,
        eventsProcessed: rawTotal,
      });

      setMcpMetrics({
        tokensUsed: mcpTotal * mcpTokensPer,
        latencyMs: mcpLatency,
        costPerMillion: mcpCostRate * mcpTokensPer,
        eventsProcessed: mcpTotal,
      });

      // Sample latency for chart at a lower frequency
      if (elapsed - lastChartSampleRef.current >= CHART_SAMPLE_INTERVAL_MS) {
        lastChartSampleRef.current = elapsed;
        setLatencyHistory((prev) => [
          ...prev,
          { time: parseFloat((elapsed / 1000).toFixed(1)), raw: rawLatency, mcp: mcpLatency },
        ]);
      }
    }, TICK_INTERVAL_MS);
  }, [resetTest]);

  const startRace = useCallback(() => {
    setRaceRunning(true);
    setRaceComplete(false);
    setRawProgress(0);
    setMcpProgress(0);
    setRawRaceLatency(0);
    setMcpRaceLatency(0);

    // Simulate realistic latencies: raw 35-65ms, MCP 6-14ms
    const targetRawMs = Math.round(35 + Math.random() * 30);
    const targetMcpMs = Math.round(6 + Math.random() * 8);
    const animDuration = 1500; // 1.5s visual animation
    const raceStart = performance.now();

    // MCP finishes at fraction of animation proportional to its latency
    const mcpFinishFrac = targetMcpMs / targetRawMs;

    let mcpDone = false;
    let rawDone = false;

    const animate = (now: number) => {
      const elapsed = now - raceStart;
      const rawFrac = Math.min(elapsed / animDuration, 1);
      const mcpFrac = Math.min(elapsed / (animDuration * mcpFinishFrac), 1);

      setRawProgress(rawFrac * 100);
      setMcpProgress(mcpFrac * 100);

      if (mcpFrac >= 1 && !mcpDone) {
        mcpDone = true;
        setMcpRaceLatency(targetMcpMs);
      }
      if (rawFrac >= 1 && !rawDone) {
        rawDone = true;
        setRawRaceLatency(targetRawMs);
      }

      if (rawFrac < 1 || mcpFrac < 1) {
        raceAnimRef.current = requestAnimationFrame(animate);
      } else {
        setRaceRunning(false);
        setRaceComplete(true);
      }
    };

    raceAnimRef.current = requestAnimationFrame(animate);
  }, []);

  const reRunRace = useCallback(() => {
    // Pick a different query
    const currentIdx = RACE_QUERIES.indexOf(raceQuery);
    const nextIdx = (currentIdx + 1) % RACE_QUERIES.length;
    setRaceQuery(RACE_QUERIES[nextIdx]!);
    // Will start after state update via the startRace call
    setRaceComplete(false);
    setRawProgress(0);
    setMcpProgress(0);
    setRawRaceLatency(0);
    setMcpRaceLatency(0);
  }, [raceQuery]);

  // Auto-start race after query change from reRun
  const prevQueryRef = useRef(raceQuery);
  useEffect(() => {
    if (prevQueryRef.current !== raceQuery && !raceRunning) {
      prevQueryRef.current = raceQuery;
      startRace();
    }
  }, [raceQuery, raceRunning, startRace]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      if (raceAnimRef.current) {
        cancelAnimationFrame(raceAnimRef.current);
      }
    };
  }, []);

  const testProgress = running ? Math.min((elapsedSec / (TEST_DURATION_MS / 1000)) * 100, 100) : completed ? 100 : 0;

  return (
    <div data-testid="mcp-showdown-panel" className="space-y-4" role="region" aria-label="MCP Showdown comparison">
      {/* Shared controls */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold">MCP Showdown</h2>
          {running && (
            <span className="text-sm text-muted-foreground" data-testid="test-running-label" aria-live="polite">
              {elapsedSec}s / {TEST_DURATION_MS / 1000}s
            </span>
          )}
          {completed && (
            <span className="text-sm text-muted-foreground" data-testid="test-complete-label" aria-live="polite">
              Test complete
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {running ? (
            <Button variant="destructive" size="sm" onClick={stopTest} data-testid="stop-test-button" aria-label="Stop test">
              <Square className="mr-2 h-4 w-4" />
              Stop Test
            </Button>
          ) : (
            <Button size="sm" onClick={startTest} data-testid="start-test-button" aria-label={completed ? "Restart test" : "Start test"}>
              <Play className="mr-2 h-4 w-4" />
              {completed ? "Restart Test" : "Start Test"}
            </Button>
          )}
        </div>
      </div>

      {/* Progress bar */}
      {(running || completed) && (
        <div
          className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
          data-testid="test-progress-bar"
          role="progressbar"
          aria-valuenow={Math.round(testProgress)}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-label="Test progress"
        >
          <div
            className="h-full rounded-full bg-primary transition-all duration-300"
            style={{ width: `${testProgress}%` }}
          />
        </div>
      )}

      {/* Dual latency line charts */}
      {(running || completed) && latencyHistory.length > 1 && (
        <Card data-testid="latency-chart-card">
          <CardContent className="p-4">
            <h3 className="mb-3 text-sm font-medium">Latency Over Time</h3>
            <div className="h-48" data-testid="latency-chart">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={latencyHistory} margin={{ top: 5, right: 20, left: 0, bottom: 5 }}>
                  <CartesianGrid strokeDasharray="3 3" opacity={0.2} />
                  <XAxis
                    dataKey="time"
                    tickFormatter={(v: number) => `${v}s`}
                    fontSize={11}
                    tick={{ fill: "var(--color-muted-foreground)" }}
                    stroke="var(--color-muted-foreground)"
                  />
                  <YAxis
                    tickFormatter={(v: number) => `${v}ms`}
                    fontSize={11}
                    tick={{ fill: "var(--color-muted-foreground)" }}
                    stroke="var(--color-muted-foreground)"
                    domain={[0, "auto"]}
                  />
                  <Tooltip
                    formatter={(value: number, name: string) => [
                      `${value}ms`,
                      name === "raw" ? "Raw Mode" : "MCP Mode",
                    ]}
                    labelFormatter={(label: number) => `${label}s`}
                    contentStyle={{
                      backgroundColor: "var(--color-background)",
                      border: "1px solid var(--color-border)",
                      borderRadius: "6px",
                      fontSize: "12px",
                      color: "var(--color-foreground)",
                    }}
                  />
                  <ReferenceLine y={100} stroke="var(--color-muted-foreground)" strokeDasharray="5 5" opacity={0.4} />
                  <Line
                    type="monotone"
                    dataKey="raw"
                    stroke="#f97316"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                    name="raw"
                  />
                  <Line
                    type="monotone"
                    dataKey="mcp"
                    stroke="#22c55e"
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                    name="mcp"
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
            <div className="mt-2 flex items-center justify-center gap-6 text-xs">
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-4 rounded-sm bg-orange-500" />
                <span className="text-muted-foreground">Raw Mode</span>
              </div>
              <div className="flex items-center gap-1.5">
                <div className="h-2 w-4 rounded-sm bg-green-500" />
                <span className="text-muted-foreground">MCP Mode</span>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Speed Race */}
      <Card data-testid="speed-race-card">
        <CardContent className="p-4">
          <div className="mb-3 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Trophy className="h-4 w-4 text-yellow-500" />
              <h3 className="text-sm font-medium">Speed Race</h3>
            </div>
            <div className="flex items-center gap-2">
              {raceComplete && (
                <Button variant="ghost" size="sm" onClick={reRunRace} data-testid="rerun-race-button" aria-label="Try another speed race query">
                  <RotateCcw className="mr-1.5 h-3.5 w-3.5" />
                  Try Another
                </Button>
              )}
              {!raceRunning && !raceComplete && (
                <Button size="sm" onClick={startRace} data-testid="speed-race-button" aria-label="Start speed race">
                  <Zap className="mr-1.5 h-3.5 w-3.5" />
                  Speed Race
                </Button>
              )}
            </div>
          </div>

          {/* Query label */}
          <p className="mb-3 text-xs text-muted-foreground" data-testid="race-query-label">
            Query: <span className="font-medium text-foreground">&quot;{raceQuery}&quot;</span>
          </p>

          {/* Race bars */}
          <div className="space-y-3">
            {/* Raw bar */}
            <div data-testid="raw-race-bar">
              <div className="mb-1 flex items-center justify-between text-xs">
                <span className="font-medium text-orange-500">Raw Mode</span>
                {rawRaceLatency > 0 && (
                  <span className="tabular-nums font-bold text-orange-500" data-testid="raw-race-latency" aria-live="polite">
                    {rawRaceLatency}ms
                  </span>
                )}
              </div>
              <div
                className="h-6 w-full overflow-hidden rounded-full bg-orange-500/10"
                role="progressbar"
                aria-valuenow={Math.round(rawProgress)}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label="Raw mode race progress"
              >
                <div
                  className="h-full rounded-full bg-orange-500 transition-none"
                  style={{ width: `${rawProgress}%` }}
                  data-testid="raw-race-fill"
                />
              </div>
            </div>

            {/* MCP bar */}
            <div data-testid="mcp-race-bar">
              <div className="mb-1 flex items-center justify-between text-xs">
                <span className="font-medium text-green-500">MCP Mode</span>
                {mcpRaceLatency > 0 && (
                  <span className="tabular-nums font-bold text-green-500" data-testid="mcp-race-latency" aria-live="polite">
                    {mcpRaceLatency}ms
                  </span>
                )}
              </div>
              <div
                className="h-6 w-full overflow-hidden rounded-full bg-green-500/10"
                role="progressbar"
                aria-valuenow={Math.round(mcpProgress)}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label="MCP mode race progress"
              >
                <div
                  className="h-full rounded-full bg-green-500 transition-none"
                  style={{ width: `${mcpProgress}%` }}
                  data-testid="mcp-race-fill"
                />
              </div>
            </div>
          </div>

          {/* Completion label */}
          {raceComplete && rawRaceLatency > 0 && mcpRaceLatency > 0 && (
            <p className="mt-3 text-center text-sm font-semibold text-green-500" data-testid="race-completion-label" aria-live="polite">
              Tokens slashed. Speed {Math.round(rawRaceLatency / mcpRaceLatency)}x. No code tweaks.
            </p>
          )}
        </CardContent>
      </Card>

      {/* Split screen */}
      <div className="grid gap-4 md:grid-cols-2" data-testid="showdown-split">
        {/* Left: Raw Mode */}
        <Card className="border-orange-500/30" data-testid="raw-mode-panel" role="region" aria-label="Raw Mode metrics">
          <CardContent className="p-6">
            <div className="mb-4 flex items-center gap-2">
              <div className="h-3 w-3 rounded-full bg-orange-500" aria-hidden="true" />
              <h3 className="text-lg font-semibold" data-testid="raw-mode-label">Raw Mode</h3>
              <span className="ml-auto text-xs text-muted-foreground">Direct Core queries</span>
            </div>
            <MetricsGrid metrics={rawMetrics} accentColor="orange" running={running} />
          </CardContent>
        </Card>

        {/* Right: MCP Mode */}
        <Card className="border-green-500/30" data-testid="mcp-mode-panel" role="region" aria-label="MCP Mode metrics">
          <CardContent className="p-6">
            <div className="mb-4 flex items-center gap-2">
              <div className="h-3 w-3 rounded-full bg-green-500" aria-hidden="true" />
              <h3 className="text-lg font-semibold" data-testid="mcp-mode-label">MCP Mode</h3>
              <span className="ml-auto text-xs text-muted-foreground">MCP-routed queries</span>
            </div>
            <MetricsGrid metrics={mcpMetrics} accentColor="green" running={running} />
          </CardContent>
        </Card>
      </div>

      {/* Summary comparison (after test completes) */}
      {completed && (
        <Card data-testid="showdown-summary">
          <CardContent className="p-4">
            <h3 className="mb-3 text-center text-sm font-medium text-muted-foreground">Test Results</h3>
            <div className="grid grid-cols-3 gap-4 text-center text-sm">
              <div>
                <p className="text-muted-foreground">Token Savings</p>
                <p className="text-2xl font-bold text-green-500" data-testid="token-savings">
                  {rawMetrics.tokensUsed > 0
                    ? `${Math.round(((rawMetrics.tokensUsed - mcpMetrics.tokensUsed) / rawMetrics.tokensUsed) * 100)}%`
                    : "—"}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground">Avg Latency Improvement</p>
                <p className="text-2xl font-bold text-green-500" data-testid="latency-improvement">
                  {latencyHistory.length > 0
                    ? (() => {
                        const avgRaw = latencyHistory.reduce((sum, d) => sum + d.raw, 0) / latencyHistory.length;
                        const avgMcp = latencyHistory.reduce((sum, d) => sum + d.mcp, 0) / latencyHistory.length;
                        return `${(avgRaw / Math.max(avgMcp, 1)).toFixed(1)}x faster`;
                      })()
                    : "—"}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground">Cost Reduction</p>
                <p className="text-2xl font-bold text-green-500" data-testid="cost-reduction">
                  {rawMetrics.costPerMillion > 0
                    ? `${Math.round(((rawMetrics.costPerMillion - mcpMetrics.costPerMillion) / rawMetrics.costPerMillion) * 100)}%`
                    : "—"}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

interface MetricsGridProps {
  metrics: SideMetrics;
  accentColor: "orange" | "green";
  running: boolean;
}

function MetricsGrid({ metrics, accentColor, running }: MetricsGridProps) {
  const colorClass = accentColor === "orange" ? "text-orange-500" : "text-green-500";
  const bgClass = accentColor === "orange" ? "bg-orange-500/10" : "bg-green-500/10";

  return (
    <div className="grid grid-cols-2 gap-3">
      <MetricCard
        icon={<Hash className="h-4 w-4" />}
        label="Tokens Used"
        value={formatNumber(metrics.tokensUsed)}
        colorClass={colorClass}
        bgClass={bgClass}
        testId="metric-tokens"
        animate={running}
      />
      <MetricCard
        icon={<Clock className="h-4 w-4" />}
        label="Latency"
        value={`${metrics.latencyMs}ms`}
        colorClass={colorClass}
        bgClass={bgClass}
        testId="metric-latency"
        animate={running}
      />
      <MetricCard
        icon={<DollarSign className="h-4 w-4" />}
        label="Cost / 1M events"
        value={`$${metrics.costPerMillion.toFixed(0)}`}
        colorClass={colorClass}
        bgClass={bgClass}
        testId="metric-cost"
        animate={running}
      />
      <MetricCard
        icon={<Zap className="h-4 w-4" />}
        label="Events Processed"
        value={formatNumber(metrics.eventsProcessed)}
        colorClass={colorClass}
        bgClass={bgClass}
        testId="metric-events"
        animate={running}
      />
    </div>
  );
}

interface MetricCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  colorClass: string;
  bgClass: string;
  testId: string;
  animate: boolean;
}

function MetricCard({ icon, label, value, colorClass, bgClass, testId, animate }: MetricCardProps) {
  return (
    <div className={`rounded-lg p-3 ${bgClass}`} data-testid={testId} aria-label={`${label}: ${value}`}>
      <div className={`mb-1 flex items-center gap-1.5 ${colorClass}`} aria-hidden="true">
        {icon}
        <span className="text-xs font-medium">{label}</span>
      </div>
      <p className={`text-xl font-bold tabular-nums ${animate ? "transition-all duration-200" : ""}`} aria-live={animate ? "off" : undefined}>
        {value}
      </p>
    </div>
  );
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}
