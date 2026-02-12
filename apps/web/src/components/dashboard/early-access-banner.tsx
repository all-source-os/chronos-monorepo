"use client";

import { cn } from "@allsource/ui/utils";
import { Sparkles, X } from "lucide-react";
import { useEffect, useState } from "react";

const STORAGE_KEY = "allsource-early-access-banner-dismissed";

export function EarlyAccessBanner() {
  const [dismissed, setDismissed] = useState(true); // Start hidden to prevent flash

  useEffect(() => {
    const isDismissed = localStorage.getItem(STORAGE_KEY) === "true";
    setDismissed(isDismissed);
  }, []);

  const handleDismiss = () => {
    localStorage.setItem(STORAGE_KEY, "true");
    setDismissed(true);
  };

  if (dismissed) return null;

  return (
    <div
      className={cn(
        "relative flex items-center justify-center gap-2 px-4 py-2",
        "bg-gradient-to-r from-primary/10 via-primary/5 to-primary/10",
        "border-b border-primary/20 text-sm"
      )}
    >
      <Sparkles className="h-4 w-4 text-primary" />
      <span className="text-muted-foreground">
        <span className="font-medium text-foreground">Early Access</span>
        {" — "}
        You&apos;re exploring AllSource before public launch. Some features use demo data.
      </span>
      <a
        href="https://github.com/all-source-os/allsource-monorepo/issues"
        target="_blank"
        rel="noopener noreferrer"
        className="ml-2 font-medium text-primary hover:underline"
      >
        Share feedback
      </a>
      <button
        onClick={handleDismiss}
        className="absolute right-2 top-1/2 -translate-y-1/2 rounded-md p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
        aria-label="Dismiss banner"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
