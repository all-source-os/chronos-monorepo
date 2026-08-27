"use client";

import { X } from "lucide-react";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";

const STORAGE_KEY = "allsource-design-partners-dismissed";

// Marketing-only banner. It lives in the root layout, so without this guard it
// also renders on the auth screens (/login, /signup) and the dashboard, where a
// "see pricing" promo is out of place.
const HIDDEN_PREFIXES = [
  "/login",
  "/signup",
  "/dashboard",
  "/onboarding",
  "/verify-email",
  "/reset-password",
  "/forgot-password",
];

export function EarlyAccessBanner() {
  const pathname = usePathname();
  const [dismissed, setDismissed] = useState(true); // start hidden to avoid flash

  useEffect(() => {
    setDismissed(localStorage.getItem(STORAGE_KEY) === "1");
  }, []);

  if (pathname && HIDDEN_PREFIXES.some((p) => pathname.startsWith(p))) return null;
  if (dismissed) return null;

  const handleDismiss = () => {
    localStorage.setItem(STORAGE_KEY, "1");
    setDismissed(true);
  };

  return (
    <div className="relative isolate flex items-center gap-x-6 overflow-hidden bg-primary px-6 py-2.5 sm:px-3.5 sm:before:flex-1">
      <p className="text-sm/6 text-primary-foreground">
        <strong className="font-semibold">Building an AI agent with cross-session memory?</strong>
        <svg viewBox="0 0 2 2" aria-hidden="true" className="mx-2 inline size-0.5 fill-current">
          <circle r={1} cx={1} cy={1} />
        </svg>
        Five design partner spots are open.
        <a
          href="/design-partners?utm_source=website&utm_medium=banner&utm_campaign=design_partners_2026"
          className="ml-2 font-semibold text-primary-foreground underline underline-offset-2 hover:opacity-80"
        >
          See program &rarr;
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
