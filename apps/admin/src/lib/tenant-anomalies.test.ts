/**
 * Unit tests for the pure client-side anomaly heuristics (tenant-anomalies.ts).
 *
 * Run with: `bun test src/lib/tenant-anomalies.test.ts` (from apps/admin).
 *
 * Mirrors prompt-047 verification #3: a synthetic list with one capped tenant, 8
 * same-date tenants, a trial-named tenant, and an empty tenant — each flagged with
 * the right code. Plus the plan-validity + degraded-path edges.
 */

/// <reference path="../../../../node_modules/bun-types/test.d.ts" />
import { describe, expect, test } from "bun:test";
import {
  analyzeTenant,
  analyzeTenants,
  CANONICAL_TIERS,
  calendarDate,
  detectFakeCreatedAt,
  EVENT_COUNT_CAP,
  isCapArtifact,
  isEmptyTenant,
  isLitterName,
  isPlanNotInCatalog,
  isPlanShapeDrift,
  SAME_DATE_THRESHOLD,
  worseSeverity,
  worstSeverityOf,
} from "./tenant-anomalies";
import type { Tenant } from "./tenants-api";

// Minimal tenant factory — only the fields the heuristics read.
function tenant(overrides: Partial<Tenant> = {}): Tenant {
  return {
    id: overrides.id ?? "t-1",
    name: overrides.name ?? "Acme Corp",
    plan: overrides.plan ?? "studio",
    status: overrides.status ?? "active",
    event_count: overrides.event_count ?? 1000,
    member_count: overrides.member_count ?? 3,
    created_at: overrides.created_at ?? "2026-01-15T10:00:00Z",
  };
}

describe("calendarDate", () => {
  test("slices an RFC3339 date", () => {
    expect(calendarDate("2026-06-26T17:33:00Z")).toBe("2026-06-26");
  });
  test("returns null for missing/garbage", () => {
    expect(calendarDate(undefined)).toBeNull();
    expect(calendarDate(null)).toBeNull();
    expect(calendarDate("")).toBeNull();
    expect(calendarDate("not-a-date")).toBeNull();
  });
});

describe("severity helpers", () => {
  test("worseSeverity picks the higher rank", () => {
    expect(worseSeverity("info", "warn")).toBe("warn");
    expect(worseSeverity("warn", "critical")).toBe("critical");
    expect(worseSeverity("info", "info")).toBe("info");
  });
  test("worstSeverityOf reduces a finding set", () => {
    expect(worstSeverityOf([])).toBeNull();
    expect(
      worstSeverityOf([{ severity: "info" }, { severity: "warn" }, { severity: "info" }])
    ).toBe("warn");
  });
});

describe("cap artifact (count_cap_artifact)", () => {
  test("flags event_count === 1,000,000 exactly", () => {
    expect(isCapArtifact({ event_count: EVENT_COUNT_CAP })).toBe(true);
    expect(EVENT_COUNT_CAP).toBe(1_000_000);
  });
  test("does not flag a near value or undefined", () => {
    expect(isCapArtifact({ event_count: 999_999 })).toBe(false);
    expect(isCapArtifact({ event_count: 1_000_001 })).toBe(false);
    expect(isCapArtifact({ event_count: undefined })).toBe(false);
  });
  test("analyzeTenant emits a warn count_cap_artifact finding", () => {
    const f = analyzeTenant(tenant({ event_count: EVENT_COUNT_CAP }));
    expect(f.some((x) => x.code === "count_cap_artifact" && x.severity === "warn")).toBe(true);
  });
});

describe("plan validity", () => {
  test("canonical tiers are the five expected", () => {
    expect([...CANONICAL_TIERS].sort()).toEqual(["enterprise", "free", "indie", "scale", "studio"]);
  });
  test("isPlanShapeDrift flags a non-string plan (the React #31 shape)", () => {
    expect(isPlanShapeDrift({ name: "Studio", tier: "studio" })).toBe(true);
    expect(isPlanShapeDrift("studio")).toBe(false);
    expect(isPlanShapeDrift(null)).toBe(false);
    expect(isPlanShapeDrift(undefined)).toBe(false);
  });
  test("isPlanNotInCatalog flags a retired alias but not a canonical tier", () => {
    expect(isPlanNotInCatalog("pro")).toBe(true); // retired 010 alias
    expect(isPlanNotInCatalog("starter")).toBe(true);
    expect(isPlanNotInCatalog("studio")).toBe(false);
    expect(isPlanNotInCatalog("STUDIO")).toBe(false); // case-insensitive
    expect(isPlanNotInCatalog(null)).toBe(false); // "no plan" is legit
    expect(isPlanNotInCatalog(undefined)).toBe(false);
  });
  test("a non-string plan yields plan_shape_drift (not plan_not_in_catalog)", () => {
    const f = analyzeTenant(tenant({ plan: { name: "Studio", tier: "studio" } }));
    const codes = f.map((x) => x.code);
    expect(codes).toContain("plan_shape_drift");
    expect(codes).not.toContain("plan_not_in_catalog");
  });
  test("a retired-alias string plan yields plan_not_in_catalog", () => {
    const f = analyzeTenant(tenant({ plan: "pro" }));
    expect(f.some((x) => x.code === "plan_not_in_catalog")).toBe(true);
  });
});

describe("litter", () => {
  test("isLitterName matches trial/demo/smoke/epoch-suffix", () => {
    expect(isLitterName("anonymous-trial-x9")).toBe(true);
    expect(isLitterName("Demo User")).toBe(true);
    expect(isLitterName("nightly-smoke-test")).toBe(true);
    expect(isLitterName("workspace-1718900000")).toBe(true); // 10-digit epoch suffix
    expect(isLitterName("Acme Corp")).toBe(false);
    expect(isLitterName("")).toBe(false);
    expect(isLitterName(undefined)).toBe(false);
  });
  test("isEmptyTenant flags 0 events AND 0 members", () => {
    expect(isEmptyTenant({ event_count: 0, member_count: 0 })).toBe(true);
    expect(isEmptyTenant({ event_count: 1, member_count: 0 })).toBe(false);
    expect(isEmptyTenant({ event_count: 0, member_count: 1 })).toBe(false);
    expect(isEmptyTenant({ event_count: undefined, member_count: undefined })).toBe(true);
  });
  test("a trial NAME yields trial_or_test_tenant (preferred over empty_tenant)", () => {
    const f = analyzeTenant(tenant({ name: "Demo User", event_count: 0, member_count: 0 }));
    const codes = f.map((x) => x.code);
    expect(codes).toContain("trial_or_test_tenant");
    expect(codes).not.toContain("empty_tenant");
  });
  test("a plain empty tenant yields empty_tenant", () => {
    const f = analyzeTenant(tenant({ name: "Acme", event_count: 0, member_count: 0 }));
    expect(f.some((x) => x.code === "empty_tenant" && x.severity === "info")).toBe(true);
  });
});

describe("detectFakeCreatedAt", () => {
  test("flags when ≥8 tenants share one calendar date", () => {
    const list = Array.from({ length: SAME_DATE_THRESHOLD }, (_, i) =>
      tenant({ id: `same-${i}`, created_at: "2026-06-26T12:00:00.7074990Z" })
    );
    const r = detectFakeCreatedAt(list);
    expect(r.suspicious).toBe(true);
    expect(r.date).toBe("2026-06-26");
    expect(r.count).toBe(SAME_DATE_THRESHOLD);
  });
  test("does not flag when dates are spread out", () => {
    const list = Array.from({ length: 10 }, (_, i) =>
      tenant({ id: `d-${i}`, created_at: `2026-0${(i % 9) + 1}-15T00:00:00Z` })
    );
    const r = detectFakeCreatedAt(list);
    expect(r.suspicious).toBe(false);
    expect(r.date).toBeNull();
  });
  test("7 same-date tenants is under the threshold", () => {
    const list = Array.from({ length: 7 }, (_, i) =>
      tenant({ id: `s-${i}`, created_at: "2026-06-26T00:00:00Z" })
    );
    expect(detectFakeCreatedAt(list).suspicious).toBe(false);
  });
});

describe("analyzeTenants (the synthetic list — verification #3)", () => {
  const capped = tenant({ id: "capped", name: "Busy Co", event_count: EVENT_COUNT_CAP });
  const trial = tenant({
    id: "trial",
    name: "anonymous-trial-aaa",
    event_count: 5,
    member_count: 1,
  });
  const empty = tenant({ id: "empty", name: "Ghost Co", event_count: 0, member_count: 0 });
  // 8 tenants (incl. the above three) share one created_at to trip the banner.
  const sameDate = Array.from({ length: 5 }, (_, i) =>
    tenant({ id: `same-${i}`, name: `Co ${i}`, created_at: "2026-06-26T01:02:03.7074990Z" })
  );
  const list: Tenant[] = [
    { ...capped, created_at: "2026-06-26T01:02:03.7074991Z" },
    { ...trial, created_at: "2026-06-26T01:02:03.7074992Z" },
    { ...empty, created_at: "2026-06-26T01:02:03.7074993Z" },
    ...sameDate,
  ];

  const result = analyzeTenants(list);

  test("each special tenant is flagged with the right code", () => {
    expect(result.byTenant.get("capped")?.some((f) => f.code === "count_cap_artifact")).toBe(true);
    expect(result.byTenant.get("trial")?.some((f) => f.code === "trial_or_test_tenant")).toBe(true);
    expect(result.byTenant.get("empty")?.some((f) => f.code === "empty_tenant")).toBe(true);
  });

  test("flaggedCount counts only tenants with findings", () => {
    // capped + trial + empty are flagged; the 5 plain same-date "Co N" are not.
    expect(result.flaggedCount).toBe(3);
  });

  test("the fake-created_at banner fires (8 tenants, one date)", () => {
    expect(result.fakeCreatedAt.suspicious).toBe(true);
    expect(result.fakeCreatedAt.date).toBe("2026-06-26");
    expect(result.fakeCreatedAt.count).toBe(8);
  });

  test("bySeverity tallies findings (1 warn from the cap, 2 info from litter)", () => {
    expect(result.bySeverity.warn).toBeGreaterThanOrEqual(1);
    expect(result.bySeverity.info).toBeGreaterThanOrEqual(2);
  });

  test("worstByTenant records the worst severity per flagged tenant", () => {
    expect(result.worstByTenant.get("capped")).toBe("warn");
    expect(result.worstByTenant.get("trial")).toBe("info");
  });
});

describe("degraded inputs never throw", () => {
  test("a tenant with a non-string plan + missing counts is safe", () => {
    const weird = {
      id: "weird",
      name: "",
      plan: { name: "Mystery" },
      status: "active",
      created_at: "",
    } as unknown as Tenant;
    expect(() => analyzeTenant(weird)).not.toThrow();
    const f = analyzeTenant(weird);
    // empty name → not litter-by-name; 0/0 (undefined coalesced) → empty_tenant;
    // object plan → plan_shape_drift.
    const codes = f.map((x) => x.code);
    expect(codes).toContain("plan_shape_drift");
    expect(codes).toContain("empty_tenant");
  });
  test("analyzeTenants on an empty list returns a clean zero result", () => {
    const r = analyzeTenants([]);
    expect(r.flaggedCount).toBe(0);
    expect(r.fakeCreatedAt.suspicious).toBe(false);
    expect(r.bySeverity).toEqual({ critical: 0, warn: 0, info: 0 });
  });
});
