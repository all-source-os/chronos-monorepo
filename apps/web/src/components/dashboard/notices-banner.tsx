"use client";

import { cn } from "@allsource/ui/utils";
import { AlertCircle, AlertTriangle, Info, X } from "lucide-react";
import { type Notice, type NoticeSeverity, useNotices } from "@/hooks/use-notices";

/**
 * Admin-sent notices banner (ADMIN_TENANT_POWER_TOOL §4 Pillar C / §9 Phase 6 —
 * the web half). Renders the authenticated tenant's OWN active notices near the
 * top of the dashboard shell. Each notice is dismissible; dismissal persists
 * server-side (the CP records `admin.notice.dismissed`) so it does not reappear.
 *
 * Degrades gracefully: no notices / endpoint unreachable → renders nothing.
 * The banner is supporting UI — it never blocks the dashboard.
 *
 * Visual vocabulary matches the existing dashboard banners (demo-banner,
 * early-access-banner, historical-mode-banner): a full-width `border-b` strip,
 * a leading severity icon, dismiss `X` on the right. Severity drives the tint.
 */

const SEVERITY_STYLES: Record<
  NoticeSeverity,
  { container: string; icon: string; Icon: typeof Info; close: string }
> = {
  info: {
    container: "bg-gradient-to-r from-primary/10 via-primary/5 to-primary/10 border-primary/20",
    icon: "text-primary",
    Icon: Info,
    close: "text-muted-foreground hover:bg-primary/10 hover:text-foreground",
  },
  warning: {
    container: "bg-amber-500/10 border-amber-500/30",
    icon: "text-amber-600 dark:text-amber-400",
    Icon: AlertTriangle,
    close:
      "text-amber-600/80 hover:bg-amber-500/20 hover:text-amber-700 dark:text-amber-400/80 dark:hover:text-amber-300",
  },
  critical: {
    container: "bg-destructive/10 border-destructive/30",
    icon: "text-destructive",
    Icon: AlertCircle,
    close: "text-destructive/80 hover:bg-destructive/20 hover:text-destructive",
  },
};

function normalizeSeverity(severity: string): NoticeSeverity {
  return severity === "warning" || severity === "critical" ? severity : "info";
}

function NoticeRow({ notice, onDismiss }: { notice: Notice; onDismiss: (id: string) => void }) {
  const severity = normalizeSeverity(notice.severity);
  const style = SEVERITY_STYLES[severity];
  const { Icon } = style;

  return (
    <div
      data-testid="notice-banner"
      data-notice-id={notice.id}
      data-severity={severity}
      className={cn(
        "relative flex items-start gap-3 border-b px-4 py-2.5 text-sm md:px-6",
        style.container
      )}
    >
      <Icon className={cn("mt-0.5 h-4 w-4 flex-shrink-0", style.icon)} />
      <div className="min-w-0 flex-1 pr-8">
        <span className="font-medium text-foreground">{notice.title}</span>
        {notice.body ? (
          <>
            {" — "}
            <span className="text-muted-foreground">{notice.body}</span>
          </>
        ) : null}
      </div>
      <button
        type="button"
        data-testid="notice-dismiss"
        onClick={() => onDismiss(notice.id)}
        className={cn(
          "absolute right-4 top-2.5 rounded-md p-1 transition-colors md:right-6",
          style.close
        )}
        aria-label="Dismiss notice"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

export function NoticesBanner() {
  const { notices, dismiss } = useNotices();

  // Nothing to show (no notices, no session, or endpoint unreachable) → render
  // nothing. The dashboard layout is unaffected.
  if (!notices.length) return null;

  return (
    <div data-testid="notices-banner">
      {notices.map((notice) => (
        <NoticeRow key={notice.id} notice={notice} onDismiss={dismiss} />
      ))}
    </div>
  );
}
