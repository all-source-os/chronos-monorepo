"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertTriangle,
  Brain,
  Database,
  Flame,
  Loader2,
  RefreshCw,
  Rocket,
  Swords,
  Zap,
} from "lucide-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { FadeIn } from "@/components/ui/fade-in";
import type { Event } from "@/lib/api/client";
import { normalizeDemoEvents } from "@/lib/demo/events";

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
  const [events, setEvents] = useState<Event[]>([]);
  const [checkingEvents, setCheckingEvents] = useState(true);
  const [eventError, setEventError] = useState<string | null>(null);
  const [seedError, setSeedError] = useState<string | null>(null);
  const seeded = events.length > 0;

  const loadEvents = useCallback(async () => {
    setCheckingEvents(true);
    setEventError(null);
    try {
      const response = await fetch("/api/events?limit=100", {
        cache: "no-store",
      });
      if (!response.ok) throw new Error(`Event query returned HTTP ${response.status}.`);
      setEvents(normalizeDemoEvents(await response.json()));
    } catch (error) {
      setEventError(
        error instanceof Error ? error.message : "Workspace events could not be loaded."
      );
    } finally {
      setCheckingEvents(false);
    }
  }, []);

  useEffect(() => {
    loadEvents();
  }, [loadEvents]);

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
      await loadEvents();
    } catch (err) {
      setSeedError(err instanceof Error ? err.message : "Failed to seed demo data");
    } finally {
      setSeeding(false);
    }
  }, [loadEvents]);

  return (
    <div className="mx-auto max-w-6xl space-y-6">
      {/* Header */}
      <FadeIn delay={0.1} inView>
        <div className="flex flex-col gap-4 border-b border-border pb-6 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <div className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-primary">
              <Zap className="h-4 w-4" />
              Live product workbench
            </div>
            <h1 className="text-3xl font-bold tracking-tight md:text-4xl">Demo Zone</h1>
            <p className="mt-2 max-w-2xl text-base leading-7 text-muted-foreground">
              Add sample events to your workspace, watch them arrive, then find them again.
            </p>
          </div>

          {/* View toggle + Build Your Own CTA */}
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <div className="grid grid-cols-3 rounded-lg border border-border bg-muted/50 p-1">
              {VIEWS.map((view) => (
                <button
                  type="button"
                  key={view.id}
                  onClick={() => setView(view.id)}
                  className={cn(
                    "flex items-center justify-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
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
            <Button asChild size="sm" data-testid="build-your-own-cta">
              <Link href="/dashboard/demo/onboarding">
                <Rocket className="h-4 w-4" />
                Connect your app
              </Link>
            </Button>
          </div>
        </div>
      </FadeIn>

      {/* Content area */}
      <FadeIn delay={0.2} inView>
        {activeView === "live-fire" ? (
          <LiveFireView
            events={events}
            checkingEvents={checkingEvents}
            eventError={eventError}
            seeding={seeding}
            seedError={seedError}
            onSeed={handleSeed}
            onRetry={loadEvents}
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
    <Card className="overflow-hidden border-border/80 py-0">
      <CardContent className="grid p-0 lg:grid-cols-[minmax(0,1.4fr)_minmax(280px,0.6fr)]">
        <div className="p-6 md:p-8">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">
            Empty workspace
          </p>
          <h2 className="mt-2 text-2xl font-semibold tracking-tight">Put real events on screen</h2>
          <p className="mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
            Add 60 inspectable sample events to this workspace. They use your tenant boundary and
            remain available in Events after this demo.
          </p>
          <Button onClick={onSeed} disabled={seeding} size="lg" className="mt-6">
            {seeding ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Adding sample events…
              </>
            ) : (
              <>
                <Zap className="mr-2 h-4 w-4" />
                Add sample events
              </>
            )}
          </Button>
          {seedError && (
            <div
              className="mt-4 max-w-md rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3"
              role="alert"
            >
              <p className="text-sm font-medium text-destructive">Demo could not start</p>
              <p className="mt-1 text-sm text-muted-foreground">{seedError}</p>
            </div>
          )}
        </div>
        <aside className="border-t border-border bg-muted/20 p-6 lg:border-l lg:border-t-0 lg:p-8">
          <Database className="h-5 w-5 text-primary" />
          <p className="mt-4 text-sm font-semibold">What gets added</p>
          <ul className="mt-3 space-y-2 text-sm text-muted-foreground">
            <li>Errors and API timeouts</li>
            <li>Memory and latency signals</li>
            <li>Signup activity</li>
            <li>Stable fields for filtering</li>
          </ul>
          <p className="mt-5 border-t border-border pt-4 text-xs leading-5 text-muted-foreground">
            Existing events are never replaced or deleted.
          </p>
        </aside>
      </CardContent>
    </Card>
  );
}

function LiveFireView({
  events,
  checkingEvents,
  eventError,
  seeding,
  seedError,
  onSeed,
  onRetry,
}: Omit<ViewProps, "seeded"> & {
  events: Event[];
  checkingEvents: boolean;
  eventError: string | null;
  onRetry: () => void;
}) {
  if (checkingEvents) {
    return <DemoPanelLoading />;
  }

  if (eventError && events.length === 0) {
    return (
      <Card>
        <CardContent className="flex flex-col gap-4 p-6 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex items-start gap-3">
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-500" />
            <div>
              <h2 className="font-semibold">Workspace events unavailable</h2>
              <p className="mt-1 text-sm text-muted-foreground">{eventError}</p>
            </div>
          </div>
          <Button variant="outline" onClick={onRetry} className="w-fit">
            <RefreshCw className="h-4 w-4" />
            Retry
          </Button>
        </CardContent>
      </Card>
    );
  }

  if (events.length === 0) {
    return <EmptyState seeding={seeding} seedError={seedError} onSeed={onSeed} />;
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 rounded-lg border border-border bg-muted/15 px-4 py-3 text-sm sm:flex-row sm:items-center sm:justify-between">
        <div>
          <span className="font-semibold text-foreground">{events.length} events loaded</span>
          <span className="text-muted-foreground"> · current workspace</span>
        </div>
        <Link href="/dashboard/events" className="font-medium text-primary hover:underline">
          Inspect all events →
        </Link>
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.9fr)]">
        <LiveEventStreamPanel initialEvents={events} />

        <VectorQueryPlayground initialEvents={events} />
      </div>

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
