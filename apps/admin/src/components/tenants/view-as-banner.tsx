"use client";

import { Button } from "@allsource/ui";
import { Eye, LogOut } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { stopViewAs, type ViewAsSession } from "@/lib/viewas-api";

/**
 * Persistent view-as banner (ADMIN_TENANT_POWER_TOOL §5.3).
 *
 * An always-visible "You are viewing as <tenant> — read only" bar with a live
 * countdown and a one-click Exit. WHY unmissable: the operator must NEVER forget
 * they are impersonating — an accidental belief they're in their own account is
 * how mistakes happen (§5.3).
 *
 * Teardown is wired for BOTH paths (§5.3):
 *   - Exit       → stopViewAs(reason "exit")   then return to the tenant 360;
 *   - auto-expiry → when the countdown reaches 0, stopViewAs(reason "expired")
 *                   then return — so the frame never silently outlives its token
 *                   and every started has a paired stopped in the CP audit.
 *
 * The countdown is derived from the token's `expires_at` (Unix seconds), guarded
 * so a missing/0 expiry renders "--:--" rather than NaN (§6).
 */

interface ViewAsBannerProps {
  session: ViewAsSession;
  /** Where to send the operator after Exit / expiry (defaults to the tenant 360). */
  returnTo?: string;
}

function formatRemaining(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function ViewAsBanner({ session, returnTo }: ViewAsBannerProps) {
  const router = useRouter();
  const [remaining, setRemaining] = useState<number>(() =>
    session.expires_at > 0 ? session.expires_at - Math.floor(Date.now() / 1000) : 0
  );
  const [exiting, setExiting] = useState(false);
  // Guard so the expiry teardown fires exactly once even if the tick re-renders.
  const torndown = useRef(false);

  const destination =
    returnTo || (session.tenant_id ? `/tenants/${session.tenant_id}` : "/tenants");

  const teardown = useCallback(
    async (reason: "exit" | "expired") => {
      if (torndown.current) return;
      torndown.current = true;
      setExiting(true);
      try {
        await stopViewAs(session.tenant_id, reason);
      } catch {
        // The server clears the cookie regardless; still leave the frame.
      }
      // Hard-navigate back to the normal admin frame. A full navigation ensures
      // no stale view-as state survives and the (now-cleared) cookie is gone.
      router.push(destination);
      router.refresh();
    },
    [session.tenant_id, destination, router]
  );

  // Live countdown + auto-expiry. When the remaining time hits 0, run the
  // expired teardown exactly once.
  useEffect(() => {
    if (session.expires_at <= 0) return;
    const tick = () => {
      const left = session.expires_at - Math.floor(Date.now() / 1000);
      setRemaining(left);
      if (left <= 0) {
        void teardown("expired");
      }
    };
    tick();
    const interval = setInterval(tick, 1000);
    return () => clearInterval(interval);
  }, [session.expires_at, teardown]);

  const tenantLabel = session.tenant_name || session.tenant_id || "tenant";
  const hasExpiry = session.expires_at > 0;

  return (
    <div
      className="sticky top-0 z-50 flex flex-wrap items-center justify-between gap-3 border-b border-amber-500/40 bg-amber-500/15 px-4 py-2.5 text-sm backdrop-blur"
      role="status"
      data-testid="viewas-banner"
    >
      <div className="flex items-center gap-2">
        <Eye className="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" />
        <span className="font-medium">
          You are viewing as{" "}
          <span className="font-semibold" data-testid="viewas-tenant-name">
            {tenantLabel}
          </span>{" "}
          — <span className="uppercase tracking-wide">read only</span>
        </span>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-xs text-muted-foreground" data-testid="viewas-countdown">
          {hasExpiry ? `Expires in ${formatRemaining(remaining)}` : "—"}
        </span>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void teardown("exit")}
          disabled={exiting}
          data-testid="viewas-exit-btn"
        >
          <LogOut className="mr-1.5 h-3.5 w-3.5" />
          {exiting ? "Exiting…" : "Exit"}
        </Button>
      </div>
    </div>
  );
}
