"use client";

import { X } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

const STORAGE_KEY = "allsource-design-partners-dismissed";
const COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 90;

export function EarlyAccessBanner({ initialDismissed = false }: { initialDismissed?: boolean }) {
  const [dismissed, setDismissed] = useState(initialDismissed);

  if (dismissed) return null;

  const handleDismiss = () => {
    localStorage.setItem(STORAGE_KEY, "1");
    document.cookie = `${STORAGE_KEY}=1; Path=/; Max-Age=${COOKIE_MAX_AGE_SECONDS}; SameSite=Lax; Secure`;
    setDismissed(true);
  };

  return (
    <aside
      aria-label="Design partner announcement"
      className="relative isolate flex min-h-12 items-center gap-x-3 overflow-hidden bg-primary px-3 sm:px-6 sm:before:flex-1"
    >
      <p className="py-2 text-sm/6 text-primary-foreground">
        <strong className="font-semibold">Building an AI agent with cross-session memory?</strong>
        <svg viewBox="0 0 2 2" aria-hidden="true" className="mx-2 inline size-0.5 fill-current">
          <circle r={1} cx={1} cy={1} />
        </svg>
        Five design partner spots are open.
        <Link
          href="/design-partners?utm_source=website&utm_medium=banner&utm_campaign=design_partners_2026"
          className="ml-2 inline-flex min-h-12 items-center font-semibold text-primary-foreground underline underline-offset-2 hover:opacity-80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-foreground"
        >
          See program &rarr;
        </Link>
      </p>
      <div className="flex flex-1 justify-end">
        <button
          type="button"
          onClick={handleDismiss}
          className="grid size-12 shrink-0 place-items-center rounded-md text-primary-foreground/80 hover:bg-primary-foreground/10 hover:text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-foreground"
        >
          <span className="sr-only">Dismiss</span>
          <X className="size-4" aria-hidden="true" />
        </button>
      </div>
    </aside>
  );
}
