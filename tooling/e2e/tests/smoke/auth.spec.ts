import { test, expect } from "../../fixtures/auth";

test.describe("Authenticated smoke test", () => {
  test("login succeeds and redirects to /dashboard", async ({
    authenticatedPage,
  }) => {
    // The fixture already logged in — navigate to dashboard explicitly
    await authenticatedPage.goto("/dashboard");
    await authenticatedPage.waitForURL(/\/dashboard/, { timeout: 10_000 });

    // Verify we are on the dashboard
    expect(authenticatedPage.url()).toContain("/dashboard");

    // Verify session is valid
    const resp = await authenticatedPage.request.get("/api/auth/session");
    expect(resp.ok()).toBeTruthy();
  });
});
