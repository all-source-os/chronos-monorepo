/**
 * GEO layer 1 — which AI surface sent this arrival, and the `geo.referral.observed`
 * envelope that records it.
 *
 * This module is the ONE place the AI-surface taxonomy lives. Prompt 027's
 * optimization loop extends {@link AI_SURFACES}; nothing else should grow a
 * second copy of these hostnames.
 *
 * ## The honest caveat, stated where the code is
 *
 * Most AI-referred traffic never reaches this module at all. ChatGPT, Claude
 * and most assistant surfaces strip or omit the `Referer` on outbound links,
 * so the majority of AI-sourced sessions arrive with an empty referrer and are
 * indistinguishable from Direct. Everything measured here is therefore a
 * **floor**, not a measurement: a small number means "the referrer survived
 * rarely", never "the channel is small". Layer 4 (self-report) is the only
 * layer that sees through a stripped referrer.
 *
 * ## No credentials here
 *
 * Nothing in this file touches an API key. The client classifies only to
 * decide whether to send a beacon; the server re-classifies from scratch and
 * is the only side that talks to AllSource. See
 * `src/app/api/geo/referral/route.ts`.
 */

/** One AI assistant surface people arrive from. */
export interface AiSurface {
  /** Stable id stored in `geo.referral.observed.surface`. Never renamed — a rename would split a historical series in two. */
  id: string;
  /** Human label for reports. */
  label: string;
  /**
   * Hostnames that identify the surface. A host matches when it equals an
   * entry or is a subdomain of one, so `gemini.google.com` is listed in full
   * and plain `google.com` search traffic can never match it.
   */
  hosts: readonly string[];
  /**
   * Values some surfaces append as `?utm_source=` / `?ref=` instead of (or as
   * well as) sending a referrer. ChatGPT in particular tags outbound links
   * this way, which is often the only trace left after the referrer is
   * stripped.
   */
  campaignTokens: readonly string[];
}

/**
 * The AI-surface map. Extend here and nowhere else.
 *
 * Ordering is by how much traffic we expect, then alphabetical; matching does
 * not depend on order.
 */
export const AI_SURFACES: readonly AiSurface[] = [
  {
    id: "chatgpt.com",
    label: "ChatGPT",
    hosts: ["chatgpt.com", "chat.openai.com", "chatgpt.co.uk"],
    campaignTokens: ["chatgpt.com", "chatgpt", "openai"],
  },
  {
    id: "perplexity.ai",
    label: "Perplexity",
    hosts: ["perplexity.ai", "perplexity.com"],
    campaignTokens: ["perplexity", "perplexity.ai"],
  },
  {
    id: "claude.ai",
    label: "Claude",
    hosts: ["claude.ai", "claude.com"],
    campaignTokens: ["claude", "claude.ai", "anthropic"],
  },
  {
    id: "gemini.google.com",
    label: "Gemini",
    hosts: ["gemini.google.com", "bard.google.com", "aistudio.google.com"],
    campaignTokens: ["gemini"],
  },
  {
    id: "copilot.microsoft.com",
    label: "Microsoft Copilot",
    hosts: ["copilot.microsoft.com", "m365.cloud.microsoft", "edgeservices.bing.com"],
    campaignTokens: ["copilot"],
  },
  {
    id: "you.com",
    label: "You.com",
    hosts: ["you.com"],
    campaignTokens: ["youchat"],
  },
  {
    id: "phind.com",
    label: "Phind",
    hosts: ["phind.com"],
    campaignTokens: ["phind"],
  },
  {
    id: "grok.com",
    label: "Grok",
    hosts: ["grok.com", "grok.x.ai"],
    campaignTokens: ["grok"],
  },
  {
    id: "deepseek.com",
    label: "DeepSeek",
    hosts: ["chat.deepseek.com", "deepseek.com"],
    campaignTokens: ["deepseek"],
  },
  {
    id: "mistral.ai",
    label: "Le Chat (Mistral)",
    hosts: ["chat.mistral.ai", "mistral.ai"],
    campaignTokens: ["lechat", "mistral"],
  },
  {
    id: "poe.com",
    label: "Poe",
    hosts: ["poe.com"],
    campaignTokens: ["poe"],
  },
] as const;

/** How an arrival was matched to a surface. */
export type MatchKind = "referrer" | "campaign";

/** A classified arrival from an AI surface. */
export interface ClassifiedArrival {
  /** The surface's stable id. */
  surface: string;
  /** How we knew — a real referrer, or only a campaign tag. */
  matchedBy: MatchKind;
  /** The referrer as sent, when there was one. */
  referrerUrl: string | null;
  /** Site-relative landing path, query and fragment stripped. */
  landingPath: string;
}

/** Query parameters a surface might tag an outbound link with. */
const CAMPAIGN_PARAMS = ["utm_source", "ref", "source", "via"] as const;

function hostMatches(host: string, entry: string): boolean {
  return host === entry || host.endsWith(`.${entry}`);
}

/** The surface a hostname belongs to, or `null`. */
export function surfaceForHost(rawHost: string): AiSurface | null {
  const host = rawHost
    .trim()
    .toLowerCase()
    .replace(/^www\./, "");
  if (!host) return null;
  return (
    AI_SURFACES.find((surface) => surface.hosts.some((entry) => hostMatches(host, entry))) ?? null
  );
}

/** The surface a campaign token names, or `null`. */
export function surfaceForCampaignToken(rawToken: string): AiSurface | null {
  const token = rawToken.trim().toLowerCase();
  if (!token) return null;
  return AI_SURFACES.find((surface) => surface.campaignTokens.includes(token)) ?? null;
}

/** Site-relative path with query and fragment removed, always leading-slashed. */
export function normaliseLandingPath(rawUrl: string): string {
  let path = rawUrl;
  try {
    path = new URL(rawUrl, "https://www.all-source.xyz").pathname;
  } catch {
    path = rawUrl.split(/[?#]/)[0] ?? "/";
  }
  if (!path.startsWith("/")) path = `/${path}`;
  // Collapse a trailing slash except at the root, so /docs and /docs/ are one
  // path in the report rather than two.
  return path.length > 1 ? path.replace(/\/+$/, "") || "/" : "/";
}

/**
 * Classify an arrival.
 *
 * Checks the referrer first (strongest signal), then the landing URL's
 * campaign parameters (what survives when the referrer is stripped). Returns
 * `null` for everything that is not a known AI surface — including plain
 * search engines and social. This function never guesses.
 */
export function classifyArrival(input: {
  referrer?: string | null;
  url: string;
}): ClassifiedArrival | null {
  const landingPath = normaliseLandingPath(input.url);

  const referrer = input.referrer?.trim() ?? "";
  if (referrer) {
    try {
      const referrerHost = new URL(referrer).hostname;
      const surface = surfaceForHost(referrerHost);
      if (surface) {
        return { surface: surface.id, matchedBy: "referrer", referrerUrl: referrer, landingPath };
      }
    } catch {
      // A malformed referrer is not an error — browsers and extensions send
      // odd values. Fall through to the campaign check.
    }
  }

  try {
    const url = new URL(input.url, "https://www.all-source.xyz");
    for (const param of CAMPAIGN_PARAMS) {
      const value = url.searchParams.get(param);
      if (!value) continue;
      const surface = surfaceForCampaignToken(value);
      if (surface) {
        return {
          surface: surface.id,
          matchedBy: "campaign",
          referrerUrl: referrer || null,
          landingPath,
        };
      }
    }
  } catch {
    // Unparseable landing URL — nothing more to try.
  }

  return null;
}

// ───────────────────────────────────────────────────────────────────────────
// The geo.referral.observed envelope
// ───────────────────────────────────────────────────────────────────────────

/**
 * Payload schema version. Must track `SCHEMA_VERSION` in
 * `tooling/geo/geo-core/src/events.rs`.
 */
export const GEO_SCHEMA_VERSION = 1;

/** Stamped into `metadata.emitter`, matching `geo-core`'s `EMITTER_NAME`. */
export const GEO_EMITTER = "tooling/geo";

/** What a conversion beacon may claim. A closed set — free text here would become an unqueryable mess. */
export const CONVERSION_KINDS = ["signup_started", "api_key_minted"] as const;
export type ConversionKind = (typeof CONVERSION_KINDS)[number];

/** The `geo.referral.observed` payload, mirroring `geo_core::ReferralObserved`. */
export interface ReferralObserved {
  schema_version: number;
  observed_at: string;
  surface: string;
  referrer_url: string | null;
  landing_path: string;
  session_id: string | null;
  user_agent: string | null;
  converted: boolean;
  conversion_kind: string | null;
}

/** A Core ingest envelope, mirroring `geo_core::IngestEnvelope`. */
export interface IngestEnvelope {
  event_type: string;
  entity_id: string;
  payload: ReferralObserved;
  metadata: { emitter: string; idempotency_key: string };
}

/**
 * RFC 3339 at whole-second resolution, UTC.
 *
 * Second resolution on purpose: the idempotency key is derived from the same
 * timestamp truncated to seconds, and a payload carrying milliseconds the key
 * does not see would be quietly confusing. Sub-second precision on a page view
 * measures nothing anyway. This also makes the rendering byte-identical to
 * `chrono`'s, which the contract test relies on.
 */
export function toRfc3339Seconds(date: Date): string {
  return `${new Date(Math.floor(date.getTime() / 1000) * 1000).toISOString().slice(0, 19)}Z`;
}

/** Field separator for idempotency keys — matches `geo_core::idempotency::SEP`. */
const KEY_SEPARATOR = 0x1f;
/** Hex characters kept from the digest — matches `geo_core::idempotency::KEY_LEN`. */
const KEY_LENGTH = 32;

/**
 * Derive an idempotency key exactly as `geo_core::derive_key` does: SHA-256
 * over the parts joined by a Unit Separator byte, first 16 bytes as hex.
 *
 * Byte-compatibility with the Rust side is not cosmetic. Both producers write
 * `geo.referral.observed`, and if their keys disagreed the same session would
 * land as two entities and the counts would silently inflate. A vitest asserts
 * this against the committed contract example.
 */
export async function deriveIdempotencyKey(parts: readonly string[]): Promise<string> {
  const encoder = new TextEncoder();
  const chunks: Uint8Array[] = [];
  parts.forEach((part, index) => {
    if (index > 0) chunks.push(new Uint8Array([KEY_SEPARATOR]));
    chunks.push(encoder.encode(part));
  });
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const joined = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    joined.set(chunk, offset);
    offset += chunk.length;
  }
  const digest = await crypto.subtle.digest("SHA-256", joined);
  return Array.from(new Uint8Array(digest))
    .slice(0, KEY_LENGTH / 2)
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Build the ingest envelope for one arrival.
 *
 * `converted`/`conversion_kind` are deliberately absent from the key inputs: a
 * conversion is the *same arrival, later*. Re-emitting with the same natural
 * key appends version 2 to the same Core entity, so the session is counted
 * once and its latest version is its current state.
 */
export async function buildReferralEnvelope(input: {
  observedAt: Date;
  surface: string;
  referrerUrl: string | null;
  landingPath: string;
  sessionId: string | null;
  userAgent: string | null;
  conversionKind?: ConversionKind | null;
}): Promise<IngestEnvelope> {
  const observedAt = toRfc3339Seconds(input.observedAt);
  const idempotencyKey = await deriveIdempotencyKey([
    observedAt,
    input.surface,
    input.landingPath,
    input.sessionId ?? "",
    input.referrerUrl ?? "",
  ]);

  // Keys are emitted in sorted order because `geo-core` serialises the payload
  // through a sorted map; the contract test compares the two renderings
  // byte-for-byte.
  const payload: ReferralObserved = {
    conversion_kind: input.conversionKind ?? null,
    converted: Boolean(input.conversionKind),
    landing_path: input.landingPath,
    observed_at: observedAt,
    referrer_url: input.referrerUrl,
    schema_version: GEO_SCHEMA_VERSION,
    session_id: input.sessionId,
    surface: input.surface,
    user_agent: input.userAgent,
  };

  return {
    event_type: "geo.referral.observed",
    entity_id: `geo:referral:${idempotencyKey}`,
    payload,
    metadata: { emitter: GEO_EMITTER, idempotency_key: idempotencyKey },
  };
}
