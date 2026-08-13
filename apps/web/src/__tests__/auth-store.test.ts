import { describe, expect, it } from "vitest";
import { sanitizePersistedAuthState } from "@/lib/stores/auth-store";

describe("auth store persistence", () => {
  it("drops legacy API credentials during migration", () => {
    const migrated = sanitizePersistedAuthState({
      user: { id: "user-1" },
      tenant: { id: "tenant-1" },
      isAuthenticated: true,
      coreApiKey: "must-not-survive",
    });

    expect(migrated).toEqual({
      user: { id: "user-1" },
      tenant: { id: "tenant-1" },
      isAuthenticated: true,
    });
    expect(migrated).not.toHaveProperty("coreApiKey");
  });

  it("does not preserve authenticated state without a user", () => {
    expect(sanitizePersistedAuthState({ isAuthenticated: true })).toEqual({
      user: null,
      tenant: null,
      isAuthenticated: false,
    });
  });
});
