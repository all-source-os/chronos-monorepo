/**
 * GEO layer 4 — the "how did you find us?" vocabulary and the
 * `geo.selfreport.captured` envelope.
 *
 * ## Why layer 4 exists at all
 *
 * Layer 1 (`geo-referrers.ts`) can only see arrivals whose `Referer` survived,
 * and most assistant surfaces strip it. Layer 4 is the correction: it is the
 * only layer that connects an AI answer to a real human who signed up, and the
 * only one that captures *the question they actually asked the model*. That
 * free text is first-party buyer vocabulary; no probe harness can synthesise
 * it.
 *
 * ## This list is a mirror, not an original
 *
 * The canonical vocabulary is `tooling/geo/geo-core/src/discovery.rs`,
 * serialised to `docs/contracts/geo-events/discovery-sources.json`. This file,
 * and `apps/control-plane/geo_selfreport.go`, both mirror it — the three sides
 * cannot import each other. `src/__tests__/geo-discovery-sources.test.ts`
 * asserts this file against the committed JSON.
 *
 * The failure that guards against is quiet: if the form wrote `"ChatGPT"` and
 * the API path wrote `"chatgpt"`, the report would show two channels where
 * there is one, and the AI-sourced share — the headline number of the whole
 * layer — would be silently halved.
 *
 * ## No credentials here
 *
 * Nothing in this file touches an API key. The browser posts an answer to
 * `src/app/api/geo/self-report/route.ts`; that route is the only side that
 * holds a credential and the only side that decides what reaches AllSource.
 */

import {
  deriveIdempotencyKey,
  GEO_EMITTER,
  GEO_SCHEMA_VERSION,
  toRfc3339Seconds,
} from "@/lib/geo-referrers";

/** One answer to "how did you find us?". */
export interface DiscoverySource {
  /** Stored in `geo.selfreport.captured.surface`. Stable forever — a rename splits a historical series in two. */
  id: string;
  /** Human label, shown in the form. */
  label: string;
  /** Whether this counts toward the AI-sourced share — the number the layer exists to produce. */
  ai: boolean;
}

/**
 * The vocabulary, in render order. AI options first: they are what the layer
 * measures, and burying them under "Google" costs answers.
 *
 * `other-ai` is `ai: true` on purpose — an assistant we have not named is
 * still an assistant, and excluding it would bias the correction this layer is
 * a correction *for*.
 */
export const DISCOVERY_SOURCES: readonly DiscoverySource[] = [
  { id: "chatgpt", label: "ChatGPT", ai: true },
  { id: "claude", label: "Claude", ai: true },
  { id: "perplexity", label: "Perplexity", ai: true },
  { id: "gemini", label: "Gemini", ai: true },
  { id: "copilot", label: "Microsoft Copilot", ai: true },
  { id: "other-ai", label: "Another AI assistant", ai: true },
  { id: "search", label: "Google or another search engine", ai: false },
  { id: "x-twitter", label: "X / Twitter", ai: false },
  { id: "hn-reddit", label: "Hacker News or Reddit", ai: false },
  { id: "github", label: "GitHub", ai: false },
  { id: "word-of-mouth", label: "Someone told me", ai: false },
  { id: "other", label: "Something else", ai: false },
] as const;

/**
 * Which signup path collected the answer, stored in
 * `geo.selfreport.captured.source`.
 *
 * Deliberately separate from the discovery source: one is *how we asked*, the
 * other is *what they answered*. The API path is the one that reaches agents
 * and headless signups, and a blended total could not tell us whether it is
 * capturing anything at all.
 */
export const CAPTURE_PATHS = ["signup-form", "onboard-api"] as const;
export type CapturePath = (typeof CAPTURE_PATHS)[number];

/** The capture path used by everything in `apps/web`. */
export const WEB_CAPTURE_PATH: CapturePath = "signup-form";

/** The vocabulary entry for an id, or `undefined`. */
export function discoverySource(id: string): DiscoverySource | undefined {
  return DISCOVERY_SOURCES.find((source) => source.id === id);
}

/** Whether an id is in the vocabulary at all. Used by the route to refuse free text. */
export function isDiscoverySource(id: string): boolean {
  return discoverySource(id) !== undefined;
}

/**
 * Whether the form should offer the free-text "what did you ask it?" field.
 *
 * Only the AI options. Asking someone who arrived from Hacker News what they
 * "asked it" is nonsense, and a nonsense question costs completion rate on the
 * one question we get to ask.
 */
export function promptsForVerbatim(id: string): boolean {
  return discoverySource(id)?.ai === true;
}

// ───────────────────────────────────────────────────────────────────────────
// The geo.selfreport.captured envelope
// ───────────────────────────────────────────────────────────────────────────

/** The `geo.selfreport.captured` payload, mirroring `geo_core::SelfReportCaptured`. */
export interface SelfReportCaptured {
  schema_version: number;
  observed_at: string;
  /** Capture path — which signup path collected the answer. */
  source: string;
  /** Discovery source id — what the human said sent them. */
  surface: string;
  /** The buyer's literal prompt, when they gave one. */
  verbatim: string | null;
  /** Opaque reference back to the person. A tenant id — NEVER an email address. */
  contact_ref: string | null;
  /** Tier at capture, when the capturing path could resolve one. */
  tier: string | null;
}

/** A Core ingest envelope for a self-report. */
export interface SelfReportEnvelope {
  event_type: string;
  entity_id: string;
  payload: SelfReportCaptured;
  metadata: { emitter: string; idempotency_key: string };
}

/**
 * Build the ingest envelope for one self-report.
 *
 * The natural key is `observed_at` (seconds) + `source` + `surface` +
 * `contact_ref`, matching `geo_core::GeoEvent::idempotency_key`. `verbatim`
 * and `tier` are deliberately outside it: a corrected tier or a re-submitted
 * answer is the *same capture, restated*, so re-emitting appends a version to
 * the same Core entity rather than minting a second signup — exactly as a
 * layer-1 conversion does.
 */
export async function buildSelfReportEnvelope(input: {
  observedAt: Date;
  capturePath: CapturePath;
  surface: string;
  verbatim: string | null;
  contactRef: string | null;
  tier: string | null;
}): Promise<SelfReportEnvelope> {
  const observedAt = toRfc3339Seconds(input.observedAt);
  const idempotencyKey = await deriveIdempotencyKey([
    observedAt,
    input.capturePath,
    input.surface,
    input.contactRef ?? "",
  ]);

  // Keys in sorted order: `geo-core` serialises through a sorted map and the
  // contract example is compared byte-for-byte.
  const payload: SelfReportCaptured = {
    contact_ref: input.contactRef,
    observed_at: observedAt,
    schema_version: GEO_SCHEMA_VERSION,
    source: input.capturePath,
    surface: input.surface,
    tier: input.tier,
    verbatim: input.verbatim,
  };

  return {
    event_type: "geo.selfreport.captured",
    entity_id: `geo:selfreport:${idempotencyKey}`,
    payload,
    metadata: { emitter: GEO_EMITTER, idempotency_key: idempotencyKey },
  };
}
