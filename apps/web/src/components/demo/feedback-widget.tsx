"use client";

import { useState } from "react";
import { ThumbsUp, ThumbsDown } from "lucide-react";
import { cn } from "@allsource/ui/utils";

export function FeedbackWidget() {
  const [submitted, setSubmitted] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const handleFeedback = async (positive: boolean) => {
    setSubmitting(true);
    try {
      await fetch("/api/v1/feedback", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          positive,
          page: "demo_zone",
          timestamp: new Date().toISOString(),
        }),
      });
    } catch {
      // Fire-and-forget — don't block UI on failure
    } finally {
      setSubmitted(true);
      setSubmitting(false);
    }
  };

  if (submitted) {
    return (
      <div
        data-testid="feedback-widget"
        className="fixed bottom-6 right-6 z-50 rounded-lg border border-border bg-background px-4 py-3 shadow-lg"
      >
        <p
          className="text-sm font-medium text-green-600 dark:text-green-400"
          data-testid="feedback-thanks"
        >
          Thanks!
        </p>
      </div>
    );
  }

  return (
    <div
      data-testid="feedback-widget"
      className="fixed bottom-6 right-6 z-50 rounded-lg border border-border bg-background px-4 py-3 shadow-lg"
    >
      <p className="mb-2 text-sm font-medium">Did this show the power?</p>
      <div className="flex items-center gap-2">
        <button
          data-testid="feedback-thumbs-up"
          onClick={() => handleFeedback(true)}
          disabled={submitting}
          className={cn(
            "rounded-md border border-border p-2 transition-colors hover:bg-green-100 hover:text-green-700 dark:hover:bg-green-900/30 dark:hover:text-green-400",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            submitting && "opacity-50"
          )}
          aria-label="Thumbs up — yes, this showed the power"
        >
          <ThumbsUp className="h-5 w-5" />
        </button>
        <button
          data-testid="feedback-thumbs-down"
          onClick={() => handleFeedback(false)}
          disabled={submitting}
          className={cn(
            "rounded-md border border-border p-2 transition-colors hover:bg-red-100 hover:text-red-700 dark:hover:bg-red-900/30 dark:hover:text-red-400",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            submitting && "opacity-50"
          )}
          aria-label="Thumbs down — no, this didn't show the power"
        >
          <ThumbsDown className="h-5 w-5" />
        </button>
      </div>
    </div>
  );
}
