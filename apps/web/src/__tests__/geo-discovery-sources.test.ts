/**
 * GEO layer 4 — the cross-language vocabulary and envelope contract.
 *
 * Three sides speak the discovery-source vocabulary and none of them can
 * import the others: this file's module (the web form + its route handler),
 * `apps/control-plane/geo_selfreport.go`, and `tooling/geo/geo-core`. They
 * agree through one committed, generated file. If they drifted — one writing
 * `"ChatGPT"` and another `"chatgpt"` — the report would show two channels
 * where there is one, and the AI-sourced share, the headline number of the
 * whole layer, would be silently halved.
 *
 * So nothing here is asserted against a hand-written fixture (the mistake that
 * produced gh#250). Both assertions compare against files that `geo-core`
 * generates from its own types.
 */

import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  buildSelfReportEnvelope,
  CAPTURE_PATHS,
  DISCOVERY_SOURCES,
  isDiscoverySource,
  promptsForVerbatim,
} from "@/lib/geo-discovery-sources";

const CONTRACT_ROOT = path.resolve(__dirname, "../../../../docs/contracts/geo-events");
const VOCABULARY = path.join(CONTRACT_ROOT, "discovery-sources.json");
const EXAMPLE = path.join(CONTRACT_ROOT, "examples/geo.selfreport.captured.json");

interface Vocabulary {
  capture_paths: string[];
  sources: { id: string; label: string; ai: boolean }[];
}

function vocabulary(): Vocabulary {
  return JSON.parse(readFileSync(VOCABULARY, "utf-8")) as Vocabulary;
}

describe("discovery-source vocabulary", () => {
  it("matches the committed contract exactly, id for id and label for label", () => {
    expect(DISCOVERY_SOURCES.map((s) => ({ ai: s.ai, id: s.id, label: s.label }))).toEqual(
      vocabulary().sources
    );
  });

  it("matches the committed capture paths", () => {
    expect([...CAPTURE_PATHS]).toEqual(vocabulary().capture_paths);
  });

  it("agrees with the contract about which sources are AI", () => {
    // The AI flag is the one field that changes a reported number, so it is
    // asserted on its own rather than only inside the deep-equal above.
    for (const source of vocabulary().sources) {
      expect(promptsForVerbatim(source.id)).toBe(source.ai);
    }
  });

  it("refuses an id that is not in the vocabulary", () => {
    expect(isDiscoverySource("chatgpt")).toBe(true);
    expect(isDiscoverySource("ChatGPT")).toBe(false);
    expect(isDiscoverySource("chatgpt-6")).toBe(false);
    expect(isDiscoverySource("")).toBe(false);
  });
});

describe("geo.selfreport.captured envelope", () => {
  it("is byte-identical to the envelope geo-core generates", async () => {
    // The committed example IS the Rust emitter's own output. Reproducing it
    // from the TypeScript side proves both producers derive the same
    // idempotency key — without which one signup would land as two entities.
    const committed = JSON.parse(readFileSync(EXAMPLE, "utf-8")) as Record<string, unknown>;
    const payload = committed.payload as Record<string, unknown>;

    const envelope = await buildSelfReportEnvelope({
      observedAt: new Date(payload.observed_at as string),
      capturePath: payload.source as "signup-form",
      surface: payload.surface as string,
      verbatim: payload.verbatim as string | null,
      contactRef: payload.contact_ref as string | null,
      tier: payload.tier as string | null,
    });

    expect(JSON.parse(JSON.stringify(envelope))).toEqual(committed);
  });

  it("keeps a follow-up on the same entity when the timestamp is replayed", async () => {
    // The free text arrives after the source. Replaying the first answer's
    // timestamp must derive the SAME natural key, so Core appends a version
    // rather than counting the signup twice.
    const observedAt = new Date("2026-08-11T11:30:00Z");
    const base = {
      observedAt,
      capturePath: "signup-form" as const,
      surface: "chatgpt",
      contactRef: "tenant-x",
    };
    const first = await buildSelfReportEnvelope({ ...base, verbatim: null, tier: "trial" });
    const second = await buildSelfReportEnvelope({
      ...base,
      verbatim: "how do I give my agent memory?",
      tier: "indie",
    });

    expect(second.entity_id).toBe(first.entity_id);
    expect(second.metadata.idempotency_key).toBe(first.metadata.idempotency_key);
    expect(second.payload.verbatim).toBe("how do I give my agent memory?");
  });

  it("gives a different entity to a different person and a different source", async () => {
    const base = {
      observedAt: new Date("2026-08-11T11:30:00Z"),
      capturePath: "signup-form" as const,
      surface: "chatgpt",
      contactRef: "tenant-x",
      verbatim: null,
      tier: null,
    };
    const mine = await buildSelfReportEnvelope(base);
    const theirs = await buildSelfReportEnvelope({ ...base, contactRef: "tenant-y" });
    const other = await buildSelfReportEnvelope({ ...base, surface: "claude" });
    const api = await buildSelfReportEnvelope({ ...base, capturePath: "onboard-api" });

    expect(new Set([mine, theirs, other, api].map((e) => e.entity_id)).size).toBe(4);
  });

  it("never puts an email address in contact_ref", () => {
    // A guard on the payload shape, not on this call site: `contact_ref` is
    // the tenant id by contract, and GEO telemetry is not a place to
    // accumulate PII.
    const example = JSON.parse(readFileSync(EXAMPLE, "utf-8")) as {
      payload: { contact_ref: string | null };
    };
    expect(example.payload.contact_ref).not.toContain("@");
  });
});
