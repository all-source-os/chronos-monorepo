"use client";

import { cn } from "@allsource/ui";
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useState } from "react";
import { siteConfig } from "@/lib/config";

/**
 * Self-contained, pseudo-live homepage demo. Two panes:
 *   1. Events streaming into Core (JSON, scripted loop).
 *   2. An agent recalling those events through a low-latency projection.
 *
 * This is intentionally a leaf client component so it never blocks the hero
 * text (the LCP element). The animation is scripted/looped for v1 — the
 * `SCRIPTED_EVENTS` array and `referenceReadLatency` are the only seams a real
 * Core/MCP feed has to replace later.
 *
 * Respects `prefers-reduced-motion`: with motion reduced, the full event log
 * and the answered recall render immediately, no looping.
 */

type DemoEvent = {
  type: string;
  // Pre-serialized payload so the rendered JSON is deterministic (no Date.now
  // in render → no hydration drift, and the final frame is paintable on the
  // server for the reduced-motion path).
  payload: Record<string, string | number>;
  ts: string;
};

const SCRIPTED_EVENTS: DemoEvent[] = [
  { type: "user.signed_up", payload: { user: "u_8f21", plan: "indie" }, ts: "14:58:02" },
  { type: "cart.checkout", payload: { user: "u_8f21", total: 4900 }, ts: "15:00:11" },
  { type: "agent.decided", payload: { agent: "claude", action: "upsell" }, ts: "15:00:12" },
  { type: "user.viewed", payload: { user: "u_8f21", page: "/pricing" }, ts: "15:01:44" },
  { type: "cart.checkout", payload: { user: "u_8f21", total: 1900 }, ts: "15:02:03" },
];

const STEP_MS = 1100;

function EventLine({ event }: { event: DemoEvent }) {
  return (
    <div className="font-mono text-[11px] leading-relaxed sm:text-xs">
      <span className="text-muted-foreground">{event.ts}</span>{" "}
      <span className="text-primary">{`{`}</span>
      <span className="text-foreground">&quot;type&quot;</span>
      <span className="text-muted-foreground">: </span>
      <span className="text-emerald-500 dark:text-emerald-400">&quot;{event.type}&quot;</span>
      <span className="text-muted-foreground">, </span>
      <span className="text-foreground">&quot;data&quot;</span>
      <span className="text-muted-foreground">: </span>
      <span className="text-muted-foreground">{JSON.stringify(event.payload)}</span>
      <span className="text-primary">{`}`}</span>
    </div>
  );
}

export default function HeroDemo() {
  const reduceMotion = useReducedMotion();

  // When motion is reduced, render the complete, settled state on first paint.
  const [visibleCount, setVisibleCount] = useState(reduceMotion ? SCRIPTED_EVENTS.length : 0);
  const [answered, setAnswered] = useState(reduceMotion);

  useEffect(() => {
    if (reduceMotion) {
      setVisibleCount(SCRIPTED_EVENTS.length);
      setAnswered(true);
      return;
    }

    let cancelled = false;
    const timers: ReturnType<typeof setTimeout>[] = [];

    const run = () => {
      if (cancelled) return;
      setAnswered(false);
      setVisibleCount(0);

      SCRIPTED_EVENTS.forEach((_, i) => {
        timers.push(
          setTimeout(
            () => {
              if (!cancelled) setVisibleCount(i + 1);
            },
            STEP_MS * (i + 1)
          )
        );
      });

      // Agent answers shortly after the last event lands.
      timers.push(
        setTimeout(
          () => {
            if (!cancelled) setAnswered(true);
          },
          STEP_MS * (SCRIPTED_EVENTS.length + 1)
        )
      );

      // Loop.
      timers.push(setTimeout(run, STEP_MS * (SCRIPTED_EVENTS.length + 4)));
    };

    run();

    return () => {
      cancelled = true;
      timers.forEach(clearTimeout);
    };
  }, [reduceMotion]);

  const shown = SCRIPTED_EVENTS.slice(0, visibleCount);

  return (
    <div className="w-full max-w-xl rounded-xl border border-border bg-card/80 shadow-xl shadow-primary/5 backdrop-blur-sm">
      {/* Window chrome with the headline throughput stat baked into it */}
      <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
        <div className="flex items-center gap-1.5">
          <span className="h-2.5 w-2.5 rounded-full bg-destructive/60" />
          <span className="h-2.5 w-2.5 rounded-full bg-amber-400/60" />
          <span className="h-2.5 w-2.5 rounded-full bg-emerald-400/60" />
        </div>
        <span className="font-mono text-[10px] text-muted-foreground sm:text-xs">
          AllSource Core · {siteConfig.stats[0]?.display} events/sec
        </span>
      </div>

      {/* Top pane: events streaming in */}
      <div className="border-b border-border px-4 py-3">
        <div className="mb-2 flex items-center gap-2">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-400" />
          <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground sm:text-xs">
            Events streaming → WAL + Parquet
          </span>
        </div>
        <div className="min-h-[7.5rem] space-y-1 overflow-hidden">
          {shown.map((event) => (
            <motion.div
              key={`${event.ts}-${event.type}`}
              initial={reduceMotion ? false : { opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.25 }}
            >
              <EventLine event={event} />
            </motion.div>
          ))}
        </div>
      </div>

      {/* Bottom pane: agent recall */}
      <div className="space-y-3 px-4 py-3">
        <div className="flex items-start gap-2">
          <span className="mt-0.5 rounded bg-primary/15 px-1.5 py-0.5 text-[10px] font-semibold text-primary sm:text-xs">
            Claude
          </span>
          <p className="text-xs text-foreground sm:text-sm">
            What did the user do yesterday at 3pm?
          </p>
        </div>

        <div
          className={cn(
            "rounded-lg border border-border bg-background/60 px-3 py-2 transition-opacity duration-300",
            answered ? "opacity-100" : "opacity-40"
          )}
        >
          {answered ? (
            <div className="space-y-1.5">
              <p className="text-xs text-foreground sm:text-sm">
                At <span className="font-mono">15:00</span> they checked out a{" "}
                <span className="font-semibold">$49</span> cart, then the agent decided to upsell.
              </p>
              <p className="flex items-center gap-1.5 font-mono text-[10px] text-emerald-500 dark:text-emerald-400 sm:text-xs">
                published Core read benchmark: {siteConfig.referenceReadLatency}
              </p>
            </div>
          ) : (
            <p className="font-mono text-[10px] text-muted-foreground sm:text-xs">
              recalling from event log…
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
