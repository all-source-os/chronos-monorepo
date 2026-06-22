"use client";

import useSWR from "swr";
import { apiClient } from "@/lib/api/client";
import { useAuthStore } from "@/lib/stores/auth-store";

export interface NotificationPreferences {
  usage_alerts: boolean;
  pipeline_errors: boolean;
  security_alerts: boolean;
  product_updates: boolean;
  tips: boolean;
}

const DEFAULT_PREFERENCES: NotificationPreferences = {
  usage_alerts: true,
  pipeline_errors: true,
  security_alerts: true,
  product_updates: false,
  tips: false,
};

function entityId(userId: string): string {
  return `user-prefs:${userId}`;
}

async function fetchPreferences(userId: string): Promise<NotificationPreferences> {
  const response = await apiClient.getEventsByEntity(entityId(userId));
  // apiClient.request() already unwrapped the `{ data: [...] }` envelope, so
  // response.data is the events array — `response.data.data` was undefined and
  // preferences silently fell back to defaults forever.
  const raw = response.data as unknown;
  const events = (
    Array.isArray(raw) ? raw : ((raw as { data?: unknown[] })?.data ?? [])
  ) as Array<{ payload?: { notifications?: Partial<NotificationPreferences> } }>;
  if (events.length > 0) {
    // Events are returned newest-first; take the latest
    const payload = events[0]?.payload;
    if (payload?.notifications) {
      return { ...DEFAULT_PREFERENCES, ...payload.notifications };
    }
  }
  return DEFAULT_PREFERENCES;
}

export function useNotificationPreferences() {
  const { user } = useAuthStore();
  const userId = user?.id;

  const { data, error, isLoading, mutate } = useSWR(
    userId ? `/notification-prefs/${userId}` : null,
    () => fetchPreferences(userId!),
    {
      revalidateOnFocus: false,
      dedupingInterval: 30000,
      fallbackData: DEFAULT_PREFERENCES,
    }
  );

  const preferences = data || DEFAULT_PREFERENCES;

  const updatePreference = async (key: keyof NotificationPreferences, value: boolean) => {
    if (!userId) return;

    const updated = { ...preferences, [key]: value };

    // Optimistic update
    mutate(updated, false);

    try {
      await apiClient.createEvent({
        entity_id: entityId(userId),
        event_type: "preferences.updated",
        payload: { notifications: updated },
      });
      // Revalidate to confirm
      mutate();
    } catch (err) {
      // Rollback on failure
      console.error("Failed to save notification preferences:", err);
      mutate(preferences, false);
    }
  };

  return {
    preferences,
    isLoading,
    error: error?.message,
    updatePreference,
  };
}
