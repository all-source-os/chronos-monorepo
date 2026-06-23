// Operational event namespaces that are platform telemetry, not a tenant's
// domain data: liveness probes (`service.heartbeat`), internal system events,
// and audit chatter. Hidden by default from activity feeds so they can't bury
// real events once a tenant has thousands. Kept in ONE place so the Core query
// exclude param and the client-side WebSocket filter stay in sync.
export const PLATFORM_NOISE_PREFIXES = ["service.", "_system.", "audit."] as const;

/** Comma-separated form for the Core `exclude_event_type_prefix` query param. */
export const PLATFORM_NOISE_PREFIX_PARAM = PLATFORM_NOISE_PREFIXES.join(",");

/** True when an event type belongs to a platform-noise namespace. */
export function isPlatformNoise(eventType: string): boolean {
  return PLATFORM_NOISE_PREFIXES.some((p) => eventType.startsWith(p));
}
