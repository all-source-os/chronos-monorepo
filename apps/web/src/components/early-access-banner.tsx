"use client";

import { X } from "lucide-react";
import { useEffect, useState } from "react";

const STORAGE_KEY = "allsource-ea-dismissed";

export function EarlyAccessBanner() {
  const [dismissed, setDismissed] = useState(true); // start hidden to avoid flash

  useEffect(() => {
    setDismissed(localStorage.getItem(STORAGE_KEY) === "1");
  }, []);

  if (dismissed) return null;

  const handleDismiss = () => {
    localStorage.setItem(STORAGE_KEY, "1");
    setDismissed(true);
  };

  return (
    <div className="relative isolate flex items-center gap-x-6 overflow-hidden bg-primary px-6 py-2.5 sm:px-3.5 sm:before:flex-1">
      <p className="text-sm/6 text-primary-foreground">
        <strong className="font-semibold">Your AI agent can now pay per call</strong>
        <svg viewBox="0 0 2 2" aria-hidden="true" className="mx-2 inline size-0.5 fill-current">
          <circle r={1} cx={1} cy={1} />
        </svg>
        x402 micropayments are live in v0.19.
        <a
          href="/pricing"
          className="ml-2 font-semibold text-primary-foreground underline underline-offset-2 hover:opacity-80"
        >
          See pricing &rarr;
        </a>
      </p>
      <div className="flex flex-1 justify-end">
        <button
          type="button"
          onClick={handleDismiss}
          className="-m-1.5 p-1.5 text-primary-foreground/70 hover:text-primary-foreground"
        >
          <span className="sr-only">Dismiss</span>
          <X className="size-4" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
