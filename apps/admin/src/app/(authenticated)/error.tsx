"use client";

import { useEffect } from "react";

/**
 * Error boundary for the authenticated admin routes. Without this, a render-time
 * exception (e.g. calling .toLocaleString() on an undefined API field) propagates
 * to the document level and the browser shows a blank "This page couldn't load"
 * with no message. This boundary keeps the app shell and surfaces the actual
 * error so a missing/odd API field is diagnosable instead of opaque.
 */
export default function AuthenticatedError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("[admin] route render error:", error);
  }, [error]);

  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 p-8 text-center">
      <h2 className="text-xl font-semibold">Something went wrong</h2>
      <p className="max-w-md text-sm text-muted-foreground">
        {error.message || "An unexpected error occurred rendering this page."}
      </p>
      <button
        type="button"
        onClick={reset}
        className="rounded-md border px-4 py-2 text-sm font-medium transition-colors hover:bg-muted"
      >
        Try again
      </button>
    </div>
  );
}
