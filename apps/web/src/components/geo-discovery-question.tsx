"use client";

/**
 * GEO layer 4 — "How did you find us?", the one question we get to ask.
 *
 * ## The design constraints, and why they are constraints
 *
 * - **Never blocks.** No required field, no gate on continuing, no modal. A
 *   signup funnel that drops 5% to gain attribution is a bad trade, and this
 *   question is worth far less than a signup.
 * - **Two beats, one entity.** The source is recorded the instant it is
 *   clicked, so a user who then walks away still counts. The optional free
 *   text follows and *replays the first answer's timestamp*, so Core appends a
 *   version to the same entity instead of counting the signup twice. Same
 *   mechanism layer 1 uses for a conversion.
 * - **The free text only appears for AI answers.** "What did you ask it?" is
 *   nonsense for someone who came from Hacker News, and a nonsense question
 *   costs completion rate.
 * - **Nothing here holds a credential.** The answer goes to
 *   `/api/geo/self-report`, which is the only side that talks to AllSource and
 *   the only side that decides which tenant an answer belongs to.
 *
 * The free-text answer is the highest-value data in the whole programme: it is
 * the buyer's literal prompt, and it is the only first-party record of how
 * buyers actually phrase the problem. Hence the inviting copy — this is not a
 * form field, it is the ask.
 */

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Check, Loader2 } from "lucide-react";
import { useId, useState } from "react";
import { DISCOVERY_SOURCES, promptsForVerbatim } from "@/lib/geo-discovery-sources";

/** Matches `MAX_VERBATIM` in the route handler — the route is the authority. */
const MAX_VERBATIM = 500;

type Phase = "asking" | "elaborating" | "done";

async function post(body: Record<string, unknown>): Promise<string | null> {
  try {
    const response = await fetch("/api/geo/self-report", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
      keepalive: true,
    });
    if (!response.ok) return null;
    const data = (await response.json()) as { observed_at?: unknown };
    return typeof data.observed_at === "string" ? data.observed_at : null;
  } catch {
    // Attribution is telemetry. It must never surface as a broken page.
    return null;
  }
}

export function GeoDiscoveryQuestion({ className }: { className?: string }) {
  const [selected, setSelected] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("asking");
  const [verbatim, setVerbatim] = useState("");
  const [saving, setSaving] = useState(false);
  /** The first answer's `observed_at`, replayed so the follow-up lands on the same entity. */
  const [observedAt, setObservedAt] = useState<string | null>(null);
  const headingId = useId();
  const verbatimId = useId();

  const choose = async (id: string) => {
    if (saving) return;
    setSelected(id);
    setSaving(true);
    // Record the source now. Everything after this point is a bonus.
    const stamp = await post({ source: id });
    setObservedAt(stamp);
    setSaving(false);
    setPhase(promptsForVerbatim(id) ? "elaborating" : "done");
  };

  const sendVerbatim = async () => {
    if (!selected || saving) return;
    const text = verbatim.trim();
    if (text) {
      setSaving(true);
      await post({ source: selected, verbatim: text, observedAt });
      setSaving(false);
    }
    setPhase("done");
  };

  if (phase === "done") {
    return (
      <div
        className={cn(
          "flex items-center justify-center gap-2 rounded-xl border border-border bg-muted/40 p-4 text-sm text-muted-foreground",
          className
        )}
      >
        <Check className="h-4 w-4 text-green-500" aria-hidden="true" />
        Thanks — that genuinely helps us decide what to write next.
      </div>
    );
  }

  return (
    <section
      aria-labelledby={headingId}
      className={cn("rounded-xl border border-border bg-card p-5 text-left", className)}
    >
      <h2 id={headingId} className="text-sm font-semibold">
        How did you find us?
      </h2>
      <p className="mt-1 text-xs text-muted-foreground">
        Optional, and it takes one click. Skip it and nothing happens.
      </p>

      <div className="mt-4 flex flex-wrap gap-2">
        {DISCOVERY_SOURCES.map((source) => (
          <button
            key={source.id}
            type="button"
            onClick={() => choose(source.id)}
            disabled={saving}
            aria-pressed={selected === source.id}
            className={cn(
              "rounded-full border px-3 py-1.5 text-xs transition-colors disabled:opacity-60",
              selected === source.id
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border bg-background hover:bg-muted"
            )}
          >
            {source.label}
          </button>
        ))}
      </div>

      {phase === "elaborating" && selected && (
        <div className="mt-4 animate-in fade-in-50 slide-in-from-top-2">
          <label htmlFor={verbatimId} className="text-sm font-medium">
            What did you ask it?
          </label>
          <p className="mt-1 text-xs text-muted-foreground">
            Your actual words are more useful to us than a tidy summary — it tells us which
            questions we should be showing up for. Still optional.
          </p>
          <textarea
            id={verbatimId}
            value={verbatim}
            onChange={(e) => setVerbatim(e.target.value.slice(0, MAX_VERBATIM))}
            rows={2}
            maxLength={MAX_VERBATIM}
            placeholder="e.g. what should I use to give my agent long-term memory?"
            className="mt-2 w-full resize-y rounded-lg border border-border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
          />
          <div className="mt-2 flex items-center gap-2">
            <Button type="button" size="sm" onClick={sendVerbatim} disabled={saving}>
              {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : "Send"}
            </Button>
            <button
              type="button"
              onClick={() => setPhase("done")}
              className="text-xs text-muted-foreground underline-offset-4 hover:underline"
            >
              No thanks
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
