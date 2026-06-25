"use client";

import useSWR from "swr";

/**
 * Tenant-facing notices (ADMIN_TENANT_POWER_TOOL §4 Pillar C / §9 Phase 6).
 *
 * Reads the authenticated tenant's OWN active notices and lets the user dismiss
 * them. Fetches via the same-origin /api/notices proxy, which forwards to the
 * Control Plane with the httpOnly `auth_token` cookie attached as Bearer (the
 * client never holds a token). Dismissal persists server-side (the CP records
 * `admin.notice.dismissed`), so a dismissed notice does not reappear.
 *
 * Resilience (the "list clients always return arrays" rule): a wrapped, empty,
 * or failed response collapses to `[]` via asList — the banner renders nothing
 * and never crashes the dashboard.
 */

export type NoticeSeverity = "info" | "warning" | "critical";

export interface Notice {
  id: string;
  tenant_id: string;
  title: string;
  body: string;
  severity: NoticeSeverity;
  created_at: string;
  expires_at?: string;
  dismissed?: boolean;
}

/**
 * Coerce any notices response shape to a Notice[]. The CP returns
 * `{ notices: [...], count }`; the proxy passes it through verbatim. Guard
 * against the wrapped (`{ data: { notices } }`), bare-array, and
 * undefined/odd-response cases so `.map` in the banner can never throw.
 */
function asNoticeList(raw: unknown): Notice[] {
  if (Array.isArray(raw)) return raw as Notice[];
  if (raw && typeof raw === "object") {
    const obj = raw as Record<string, unknown>;
    if (Array.isArray(obj.notices)) return obj.notices as Notice[];
    // Defensive: in case a future proxy wraps the body in `{ data: ... }`.
    if (obj.data && typeof obj.data === "object") {
      const inner = (obj.data as Record<string, unknown>).notices;
      if (Array.isArray(inner)) return inner as Notice[];
    }
  }
  return [];
}

async function fetchNotices(): Promise<Notice[]> {
  try {
    const response = await fetch("/api/notices", {
      headers: { "Content-Type": "application/json" },
      credentials: "include",
    });
    // Any non-OK (401 no session, 502 CP down, …) → no banner, no throw.
    if (!response.ok) return [];
    const text = await response.text();
    if (!text) return [];
    const data = JSON.parse(text) as unknown;
    return asNoticeList(data);
  } catch {
    // Network/parse failure → render nothing; the dashboard is unaffected.
    return [];
  }
}

export function useNotices() {
  const { data, isLoading, mutate } = useSWR<Notice[]>("/api/notices", fetchNotices, {
    revalidateOnFocus: false,
    dedupingInterval: 60_000,
    fallbackData: [],
    // The banner is supporting UI — a failed refresh must never surface an error.
    shouldRetryOnError: false,
  });

  // Belt-and-suspenders: even if SWR hands back something unexpected, the
  // consumer always sees an array (only active, non-dismissed notices).
  const notices = asNoticeList(data).filter((n) => n.dismissed !== true);

  const dismiss = async (id: string): Promise<void> => {
    // Optimistic remove — a dismissed notice disappears immediately and, because
    // the CP persists the dismissal, does not come back on the next revalidate.
    mutate((current) => asNoticeList(current).filter((n) => n.id !== id), false);

    try {
      const response = await fetch(`/api/notices/${encodeURIComponent(id)}/dismiss`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
      });
      if (!response.ok) throw new Error(`dismiss failed: HTTP ${response.status}`);
      // Confirm against the server (dropped from the active set going forward).
      mutate();
    } catch (err) {
      // Roll back the optimistic removal so the notice reappears for retry.
      console.error("Failed to dismiss notice:", err);
      mutate();
    }
  };

  return { notices, isLoading, dismiss };
}
