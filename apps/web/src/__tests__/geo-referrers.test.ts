/**
 * GEO layer 1 — surface classification and the cross-language envelope
 * contract.
 *
 * The important test in this file is the last one. `apps/web` and
 * `tooling/geo` BOTH write `geo.referral.observed`, and if their idempotency
 * keys disagreed the same session would land in Core as two entities and every
 * layer-1 count would silently inflate. So rather than asserting against a
 * hand-written fixture — the mistake that produced gh#250, where SDK mocks
 * honoured a contract the server did not implement — this compares our
 * TypeScript envelope against the committed example that
 * `tooling/geo/geo-core` generates from its own emitter.
 */

import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  AI_SURFACES,
  buildReferralEnvelope,
  CONVERSION_KINDS,
  classifyArrival,
  deriveIdempotencyKey,
  normaliseLandingPath,
  toRfc3339Seconds,
} from "@/lib/geo-referrers";

const CONTRACT_EXAMPLE = path.resolve(
  __dirname,
  "../../../../docs/contracts/geo-events/examples/geo.referral.observed.json"
);

describe("AI surface map", () => {
  it("covers every surface the GEO framework names", () => {
    const required = [
      "chatgpt.com",
      "chat.openai.com",
      "perplexity.ai",
      "claude.ai",
      "gemini.google.com",
      "copilot.microsoft.com",
      "you.com",
      "phind.com",
    ];
    for (const host of required) {
      expect(
        classifyArrival({ referrer: `https://${host}/`, url: "https://www.all-source.xyz/" })
      ).not.toBeNull();
    }
  });

  it("gives every surface a unique id and at least one host", () => {
    const ids = new Set<string>();
    for (const surface of AI_SURFACES) {
      expect(ids.has(surface.id)).toBe(false);
      ids.add(surface.id);
      expect(surface.hosts.length).toBeGreaterThan(0);
    }
  });

  it("does not claim ordinary search or social traffic", () => {
    // The failure that would poison layer 1: over-matching. `google.com` is
    // not `gemini.google.com`, and treating it as one would report organic
    // search as AI referral.
    for (const referrer of [
      "https://www.google.com/",
      "https://duckduckgo.com/",
      "https://news.ycombinator.com/",
      "https://x.com/someone/status/1",
      "https://github.com/all-source-os",
      "",
    ]) {
      expect(classifyArrival({ referrer, url: "https://www.all-source.xyz/pricing" })).toBeNull();
    }
  });

  it("matches subdomains of a listed surface", () => {
    const result = classifyArrival({
      referrer: "https://www.chatgpt.com/c/abc",
      url: "https://www.all-source.xyz/pricing",
    });
    expect(result?.surface).toBe("chatgpt.com");
    expect(result?.matchedBy).toBe("referrer");
  });

  it("falls back to the campaign tag when the referrer was stripped", () => {
    // This is the common case. ChatGPT tags outbound links even when it sends
    // no Referer, so the tag is often the only trace of an AI arrival.
    const result = classifyArrival({
      referrer: "",
      url: "https://www.all-source.xyz/pricing?utm_source=chatgpt.com",
    });
    expect(result?.surface).toBe("chatgpt.com");
    expect(result?.matchedBy).toBe("campaign");
    expect(result?.referrerUrl).toBeNull();
  });

  it("prefers the referrer over the campaign tag", () => {
    const result = classifyArrival({
      referrer: "https://perplexity.ai/search/x",
      url: "https://www.all-source.xyz/?utm_source=chatgpt.com",
    });
    expect(result?.surface).toBe("perplexity.ai");
    expect(result?.matchedBy).toBe("referrer");
  });

  it("survives a malformed referrer instead of throwing", () => {
    expect(() =>
      classifyArrival({ referrer: "android-app://com.example", url: "https://www.all-source.xyz/" })
    ).not.toThrow();
    expect(
      classifyArrival({ referrer: "not a url", url: "https://www.all-source.xyz/" })
    ).toBeNull();
  });

  it("normalises landing paths to one form", () => {
    expect(normaliseLandingPath("https://www.all-source.xyz/docs?a=1#x")).toBe("/docs");
    expect(normaliseLandingPath("/docs/")).toBe("/docs");
    expect(normaliseLandingPath("https://www.all-source.xyz/")).toBe("/");
    expect(normaliseLandingPath("/")).toBe("/");
  });
});

describe("timestamps", () => {
  it("renders whole seconds with a Z, matching chrono", () => {
    expect(toRfc3339Seconds(new Date("2026-08-11T09:15:00.987Z"))).toBe("2026-08-11T09:15:00Z");
  });
});

describe("cross-language contract with tooling/geo", () => {
  const committed = JSON.parse(readFileSync(CONTRACT_EXAMPLE, "utf8")) as {
    event_type: string;
    entity_id: string;
    payload: Record<string, unknown>;
    metadata: { emitter: string; idempotency_key: string };
  };

  it("derives byte-identical idempotency keys", async () => {
    // The exact natural key `geo_core::GeoEvent::idempotency_key` hashes for
    // the canonical referral sample.
    const key = await deriveIdempotencyKey([
      "2026-08-11T09:15:00Z",
      "chatgpt.com",
      "/pricing",
      "sess_01J8Z9QK3M",
      "https://chatgpt.com/",
    ]);
    expect(key).toBe(committed.metadata.idempotency_key);
  });

  it("builds an envelope byte-identical to the Rust emitter's", async () => {
    const envelope = await buildReferralEnvelope({
      observedAt: new Date("2026-08-11T09:15:00Z"),
      surface: "chatgpt.com",
      referrerUrl: "https://chatgpt.com/",
      landingPath: "/pricing",
      sessionId: "sess_01J8Z9QK3M",
      userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
    });

    expect(JSON.stringify(envelope, null, 2)).toBe(
      readFileSync(CONTRACT_EXAMPLE, "utf8").trimEnd()
    );
  });

  it("changes the key when the natural key changes, and not otherwise", async () => {
    const base = {
      observedAt: new Date("2026-08-11T09:15:00Z"),
      surface: "chatgpt.com",
      referrerUrl: "https://chatgpt.com/",
      landingPath: "/pricing",
      sessionId: "sess_01J8Z9QK3M",
      userAgent: "ua",
    };
    const arrival = await buildReferralEnvelope(base);
    const otherPath = await buildReferralEnvelope({ ...base, landingPath: "/docs" });
    expect(otherPath.entity_id).not.toBe(arrival.entity_id);

    // A conversion is the SAME arrival, later — same entity, so Core appends a
    // version instead of inventing a second session. This is the property the
    // whole conversion design rests on.
    const converted = await buildReferralEnvelope({ ...base, conversionKind: "signup_started" });
    expect(converted.entity_id).toBe(arrival.entity_id);
    expect(converted.payload.converted).toBe(true);
    expect(converted.payload.conversion_kind).toBe("signup_started");
    expect(arrival.payload.converted).toBe(false);
  });

  it("keeps the conversion vocabulary closed", () => {
    expect([...CONVERSION_KINDS]).toEqual(["signup_started", "api_key_minted"]);
  });
});
