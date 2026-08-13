import type { Event } from "@/lib/api/client";

export function eventMatchesLocalFilters(event: Event, search: string, fromDate: string): boolean {
  const normalizedSearch = search.trim().toLowerCase();
  const matchesSearch = normalizedSearch
    ? event.event_type.toLowerCase().includes(normalizedSearch) ||
      event.entity_id.toLowerCase().includes(normalizedSearch)
    : true;
  const matchesDate = fromDate
    ? new Date(event.timestamp).getTime() >= new Date(`${fromDate}T00:00:00`).getTime()
    : true;
  return matchesSearch && matchesDate;
}
