"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import {
  Activity,
  AlertTriangle,
  Database,
  ExternalLink,
  Layers,
  RefreshCw,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

interface BenchmarkData {
  allsource: {
    throughput_events_per_sec: number;
    query_latency_p50_ms: number;
    vector_search: string;
    vector_search_setup: string;
    services_required: number;
    storage: string;
  };
  competitors: {
    kafka: {
      throughput_events_per_sec: number;
      vector_search: string;
      vector_search_setup: string;
      services_required: number;
      note: string;
    };
    redis: {
      vector_search: string;
      vector_search_setup: string;
      services_required: number;
    };
  };
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
  return n.toString();
}

/**
 * Animated count-up hook. Animates from 0 to `target` over `duration` ms
 * once `shouldAnimate` becomes true.
 */
function useCountUp(target: number, duration: number, shouldAnimate: boolean): number {
  const [value, setValue] = useState(0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (!shouldAnimate || target === 0) {
      setValue(0);
      return;
    }

    const startTime = performance.now();

    function tick(now: number) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      // Ease-out cubic
      const eased = 1 - (1 - progress) ** 3;
      setValue(Math.round(target * eased));

      if (progress < 1) {
        rafRef.current = requestAnimationFrame(tick);
      }
    }

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [target, duration, shouldAnimate]);

  return value;
}

/**
 * Animated horizontal bar that fills to `percentage`% width.
 */
function AnimatedBar({
  percentage,
  label,
  value,
  color,
  animate,
}: {
  percentage: number;
  label: string;
  value: string;
  color: string;
  animate: boolean;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs">
        <span className="font-medium">{label}</span>
        <span className="tabular-nums text-muted-foreground">{value}</span>
      </div>
      <div className="h-3 w-full overflow-hidden rounded-full bg-muted">
        <div
          className={`h-full rounded-full transition-all duration-1000 ease-out ${color}`}
          style={{ width: animate ? `${percentage}%` : "0%" }}
        />
      </div>
    </div>
  );
}

export function SpeedSimplicityDashboard() {
  const [benchmarks, setBenchmarks] = useState<BenchmarkData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [animate, setAnimate] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const loadBenchmarks = useCallback(() => {
    setLoading(true);
    setError(null);
    fetch("/api/v1/config/benchmarks")
      .then((res) => {
        if (!res.ok) throw new Error("Failed to fetch benchmarks");
        return res.json();
      })
      .then((data: BenchmarkData) => {
        setBenchmarks(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : "Failed to load benchmarks");
        setLoading(false);
      });
  }, []);

  // Fetch benchmark config from Query Service through the same-origin route.
  useEffect(() => {
    loadBenchmarks();
  }, [loadBenchmarks]);

  // Trigger animation when component becomes visible
  const observerRef = useRef<IntersectionObserver | null>(null);
  useEffect(() => {
    if (loading || !panelRef.current) return;

    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          setAnimate(true);
          observerRef.current?.disconnect();
        }
      },
      { threshold: 0.3 }
    );

    observerRef.current.observe(panelRef.current);
    return () => observerRef.current?.disconnect();
  }, [loading]);

  const allsourceThroughput = benchmarks?.allsource.throughput_events_per_sec ?? 0;
  const kafkaThroughput = benchmarks?.competitors.kafka.throughput_events_per_sec ?? 0;
  const maxThroughput = Math.max(allsourceThroughput, kafkaThroughput);

  const animatedAllsource = useCountUp(allsourceThroughput, 1500, animate);
  const animatedKafka = useCountUp(kafkaThroughput, 1500, animate);

  const allsourceServices = benchmarks?.allsource.services_required ?? 1;
  const kafkaServices = benchmarks?.competitors.kafka.services_required ?? 3;

  if (loading) {
    return (
      <Card data-testid="speed-dashboard" role="status" aria-label="Loading benchmark evidence">
        <CardContent className="flex items-center gap-3 p-5 text-sm text-muted-foreground">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          Loading published benchmark evidence…
        </CardContent>
      </Card>
    );
  }

  if (error || !benchmarks) {
    return (
      <Card data-testid="speed-dashboard">
        <CardContent className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-start gap-3">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
            <div>
              <p className="text-sm font-medium">Benchmark evidence unavailable</p>
              <p className="mt-1 text-xs text-muted-foreground">
                {error || "Could not load benchmark data."}
              </p>
            </div>
          </div>
          <Button variant="outline" size="sm" onClick={loadBenchmarks}>
            <RefreshCw className="h-3.5 w-3.5" />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <section ref={panelRef} className="space-y-4" data-testid="speed-dashboard">
      <div className="flex flex-col gap-3 border-b border-border pb-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">
            Published evidence
          </p>
          <h2 className="mt-1 text-lg font-semibold">Reproduce, do not trust</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Repository benchmark configuration. Separate from this browser session.
          </p>
        </div>
        <Button asChild variant="outline" size="sm" className="w-fit">
          <a
            href="https://github.com/all-source-os/all-source#benchmarks"
            target="_blank"
            rel="noreferrer"
          >
            Reproduce benchmarks
            <ExternalLink className="h-3.5 w-3.5" />
          </a>
        </Button>
      </div>
      <div className="grid gap-4 md:grid-cols-3">
        {/* Card 1: Throughput */}
        <Card data-testid="throughput-card" className="h-full">
          <CardContent className="p-4">
            <div className="mb-3 flex items-center gap-2">
              <Activity className="h-4 w-4 text-primary" />
              <h3 className="text-sm font-semibold">Throughput</h3>
            </div>
            <div className="space-y-2">
              <AnimatedBar
                label="AllSource"
                value={`${formatNumber(animatedAllsource)} evt/s`}
                percentage={maxThroughput > 0 ? (allsourceThroughput / maxThroughput) * 100 : 0}
                color="bg-primary"
                animate={animate}
              />
              <AnimatedBar
                label="Kafka"
                value={`${formatNumber(animatedKafka)} evt/s`}
                percentage={maxThroughput > 0 ? (kafkaThroughput / maxThroughput) * 100 : 0}
                color="bg-orange-500"
                animate={animate}
              />
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              {allsourceThroughput > kafkaThroughput
                ? `${(allsourceThroughput / kafkaThroughput).toFixed(0)}x faster than Kafka`
                : "Comparable throughput"}
            </p>
          </CardContent>
        </Card>

        {/* Card 2: Vector Search */}
        <Card data-testid="vector-search-card" className="h-full">
          <CardContent className="p-4">
            <div className="mb-3 flex items-center gap-2">
              <Database className="h-4 w-4 text-blue-500" />
              <h3 className="text-sm font-semibold">Vector Search</h3>
            </div>
            <div className="space-y-2">
              <div className="flex items-center justify-between rounded-md bg-primary/10 px-3 py-2">
                <span className="text-sm font-medium">AllSource</span>
                <span className="rounded-full bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground">
                  {benchmarks.allsource.vector_search_setup}
                </span>
              </div>
              <div className="flex items-center justify-between rounded-md bg-orange-500/10 px-3 py-2">
                <span className="text-sm font-medium">Redis</span>
                <span className="rounded-full bg-orange-500/80 px-2 py-0.5 text-xs font-medium text-white">
                  {benchmarks.competitors.redis.vector_search_setup}
                </span>
              </div>
            </div>
            <p className="mt-2 text-xs text-muted-foreground">
              Built-in, zero setup — no external vector DB required.
            </p>
          </CardContent>
        </Card>

        {/* Card 3: No Glue — service count comparison */}
        <Card data-testid="no-glue-card" className="h-full">
          <CardContent className="p-4">
            <div className="mb-3 flex items-center gap-2">
              <Layers className="h-4 w-4 text-green-500" />
              <h3 className="text-sm font-semibold">
                Service count: {allsourceServices} vs {kafkaServices}
              </h3>
            </div>

            {/* AllSource: single service */}
            <div className="mb-3 flex items-center gap-3">
              <div className="flex items-center justify-center rounded-lg border-2 border-primary bg-primary/10 p-2">
                <Zap className="h-5 w-5 text-primary" />
              </div>
              <div>
                <p className="text-sm font-medium">AllSource</p>
                <p className="text-xs text-muted-foreground">Events + vectors + queries</p>
              </div>
            </div>

            <div className="mb-3 text-center text-xs font-medium text-muted-foreground">vs</div>

            {/* Kafka + Pinecone + ETL */}
            <div className="flex items-center gap-2">
              <div className="flex items-center justify-center rounded-lg border border-orange-500/50 bg-orange-500/10 p-1.5">
                <span className="text-xs font-bold text-orange-600">K</span>
              </div>
              <span className="text-xs text-muted-foreground">+</span>
              <div className="flex items-center justify-center rounded-lg border border-purple-500/50 bg-purple-500/10 p-1.5">
                <span className="text-xs font-bold text-purple-600">P</span>
              </div>
              <span className="text-xs text-muted-foreground">+</span>
              <div className="flex items-center justify-center rounded-lg border border-gray-500/50 bg-gray-500/10 p-1.5">
                <span className="text-xs font-bold text-gray-600">ETL</span>
              </div>
              <div className="ml-1">
                <p className="text-xs text-muted-foreground">Kafka + Pinecone + ETL</p>
              </div>
            </div>

            <p className="mt-3 text-xs text-muted-foreground">
              {benchmarks.competitors.kafka.note}
            </p>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
