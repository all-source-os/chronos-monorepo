"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Brain, Flame, Loader2, Rocket, Swords, Zap } from "lucide-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useState } from "react";
import { FadeIn } from "@/components/ui/fade-in";

function DemoPanelLoading() {
  return (
    <div
      className="h-72 animate-pulse rounded-xl border border-border bg-card"
      role="status"
      aria-label="Loading demo panel"
    />
  );
}

const LiveEventStreamPanel = dynamic(
  () =>
    import("@/components/demo/live-event-stream-panel").then(
      (module) => module.LiveEventStreamPanel
    ),
  { ssr: false, loading: DemoPanelLoading }
);
const VectorQueryPlayground = dynamic(
  () =>
    import("@/components/demo/vector-query-playground").then(
      (module) => module.VectorQueryPlayground
    ),
  { ssr: false, loading: DemoPanelLoading }
);
const SpeedSimplicityDashboard = dynamic(
  () =>
    import("@/components/demo/speed-simplicity-dashboard").then(
      (module) => module.SpeedSimplicityDashboard
    ),
  { ssr: false, loading: DemoPanelLoading }
);
const McpShowdownPanel = dynamic(
  () => import("@/components/demo/mcp-showdown-panel").then((module) => module.McpShowdownPanel),
  { ssr: false, loading: DemoPanelLoading }
);
const CostCalculator = dynamic(
  () => import("@/components/demo/cost-calculator").then((module) => module.CostCalculator),
  { ssr: false, loading: DemoPanelLoading }
);
const PrimePlayground = dynamic(
  () => import("@/components/demo/prime-playground").then((module) => module.PrimePlayground),
  { ssr: false, loading: DemoPanelLoading }
);
const FeedbackWidget = dynamic(
  () => import("@/components/demo/feedback-widget").then((module) => module.FeedbackWidget),
  { ssr: false }
);

type DemoView = "live-fire" | "mcp-showdown" | "prime";

const VIEWS: { id: DemoView; label: string; icon: React.ElementType }[] = [
  { id: "live-fire", label: "Live Fire", icon: Flame },
  { id: "mcp-showdown", label: "MCP Showdown", icon: Swords },
  { id: "prime", label: "Prime Graph", icon: Brain },
];

export default function DemoPage() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const viewParam = searchParams.get("view");
  const activeView: DemoView =
    viewParam === "mcp-showdown" ? "mcp-showdown" : viewParam === "prime" ? "prime" : "live-fire";

  const [seeding, setSeeding] = useState(false);
  const [seeded, setSeeded] = useState(false);
  const [seedError, setSeedError] = useState<string | null>(null);

  const setView = useCallback(
    (view: DemoView) => {
      const params = new URLSearchParams(searchParams.toString());
      params.set("view", view);
      router.push(`/dashboard/demo?${params.toString()}`);
    },
    [router, searchParams]
  );

  const handleSeed = useCallback(async () => {
    setSeeding(true);
    setSeedError(null);
    try {
      const response = await fetch("/api/v1/demo/seed", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });
      const data = (await response.json().catch(() => null)) as {
        seeded?: boolean;
        error?: string;
      } | null;
      if (!response.ok) {
        throw new Error(data?.error || `Demo service returned HTTP ${response.status}.`);
      }
      if (!data?.seeded) {
        throw new Error("Demo service did not confirm setup. Try again.");
      }
      setSeeded(true);
    } catch (err) {
      setSeedError(err instanceof Error ? err.message : "Failed to seed demo data");
    } finally {
      setSeeding(false);
    }
  }, []);

  return (
    <div className="space-y-6">
      {/* Header */}
      <FadeIn delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <div className="flex items-center gap-2">
              <Zap className="h-6 w-6 text-primary" />
              <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Demo Zone</h1>
            </div>
            <p className="mt-1 text-muted-foreground">
              Experience AllSource in action — real data, real speed, real results.
            </p>
          </div>

          {/* View toggle + Build Your Own CTA */}
          <div className="flex items-center gap-3">
            <div className="flex items-center rounded-lg border border-border bg-muted/50 p-1">
              {VIEWS.map((view) => (
                <button
                  type="button"
                  key={view.id}
                  onClick={() => setView(view.id)}
                  className={cn(
                    "flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    activeView === view.id
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground"
                  )}
                  aria-label={`Switch to ${view.label} view`}
                  aria-pressed={activeView === view.id}
                >
                  <view.icon className="h-4 w-4" />
                  {view.label}
                </button>
              ))}
            </div>
            {seeded && (
              <Button asChild size="sm" data-testid="build-your-own-cta">
                <Link href="/dashboard/demo/onboarding">
                  <Rocket className="mr-2 h-4 w-4" />
                  Build Your Own
                </Link>
              </Button>
            )}
          </div>
        </div>
      </FadeIn>

      {/* Content area */}
      <FadeIn delay={0.2} inView>
        {activeView === "live-fire" ? (
          <LiveFireView
            seeded={seeded}
            seeding={seeding}
            seedError={seedError}
            onSeed={handleSeed}
          />
        ) : activeView === "prime" ? (
          <PrimeView />
        ) : (
          <McpShowdownView
            seeded={seeded}
            seeding={seeding}
            seedError={seedError}
            onSeed={handleSeed}
          />
        )}
      </FadeIn>

      {/* Feedback widget — floating bottom-right after seeding */}
      {seeded && <FeedbackWidget />}
    </div>
  );
}

interface ViewProps {
  seeded: boolean;
  seeding: boolean;
  seedError: string | null;
  onSeed: () => void;
}

function EmptyState({ seeding, seedError, onSeed }: Omit<ViewProps, "seeded">) {
  return (
    <Card>
      <CardContent className="flex flex-col items-center justify-center py-16">
        <Zap className="mb-4 h-12 w-12 text-muted-foreground/50" />
        <h2 className="mb-2 text-xl font-semibold">No demo data yet</h2>
        <p className="mb-6 max-w-md text-center text-muted-foreground">
          Seed your event store with 1,000 diverse events to experience the full Demo Zone —
          real-time streaming, vector search, and speed comparisons.
        </p>
        <Button onClick={onSeed} disabled={seeding} size="lg">
          {seeding ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Seeding...
            </>
          ) : (
            <>
              <Zap className="mr-2 h-4 w-4" />
              Start Demo
            </>
          )}
        </Button>
        {seedError && (
          <div
            className="mt-4 max-w-md rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-center"
            role="alert"
          >
            <p className="text-sm font-medium text-destructive">Demo could not start</p>
            <p className="mt-1 text-sm text-muted-foreground">{seedError}</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function LiveFireView({ seeded, seeding, seedError, onSeed }: ViewProps) {
  if (!seeded) {
    return <EmptyState seeding={seeding} seedError={seedError} onSeed={onSeed} />;
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[2fr_1.5fr_1.5fr]">
      {/* Left: Event Stream (40%) */}
      <LiveEventStreamPanel />

      {/* Middle: Vector Query (30%) */}
      <VectorQueryPlayground />

      {/* Right: Speed Dashboard (30%) */}
      <SpeedSimplicityDashboard />
    </div>
  );
}

function McpShowdownView(_props: ViewProps) {
  return (
    <div className="space-y-4">
      <McpShowdownPanel />
      <CostCalculator />
    </div>
  );
}

function PrimeView() {
  return (
    <div className="grid gap-4 lg:grid-cols-1">
      <PrimePlayground />
    </div>
  );
}
