import { test, expect } from "@playwright/test";

/**
 * Marketing & Public Pages E2E Tests — Production / Staging compatible
 *
 * Tests all public-facing pages that don't require authentication.
 * These pages should always be accessible and render correctly.
 *
 * Run against production:
 *   BASE_URL=https://www.all-source.xyz \
 *     bunx playwright test tests/smoke/marketing.spec.ts
 */

// ---------------------------------------------------------------------------
// Homepage
// ---------------------------------------------------------------------------

test.describe("Homepage (/)", () => {
  test("loads with hero section", async ({ page }) => {
    const response = await page.goto("/");
    expect(response?.status()).toBeLessThan(400);

    // Hero heading or main CTA should be visible
    await expect(
      page.getByRole("heading").first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("navigation links are present", async ({ page }) => {
    await page.goto("/");

    // Login/signup links in header
    await expect(
      page.getByRole("link", { name: /log ?in|sign ?in/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("key marketing sections render", async ({ page }) => {
    await page.goto("/");

    // Pricing section should exist on the page
    await expect(page.getByText(/pricing/i).first()).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

test.describe("Login (/login)", () => {
  test("renders without errors", async ({ page }) => {
    const response = await page.goto("/login");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading", { name: /welcome back/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("all auth options visible", async ({ page }) => {
    await page.goto("/login");

    await expect(page.getByRole("button", { name: /google/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("button", { name: /github/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /try demo/i })).toBeVisible();
    await expect(page.getByRole("button", { name: /email/i })).toBeVisible();
  });

  test("email form expands on click", async ({ page }) => {
    await page.goto("/login");

    await page.getByRole("button", { name: /email/i }).click();

    await expect(page.locator('input[type="email"]')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('input[type="password"]')).toBeVisible();
    await expect(page.getByRole("button", { name: /sign in/i })).toBeVisible();
  });

  test("has forgot password link", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("button", { name: /email/i }).click();

    await expect(
      page.getByRole("link", { name: /forgot/i })
    ).toBeVisible({ timeout: 5000 });
  });

  test("has sign up link", async ({ page }) => {
    await page.goto("/login");

    // May be a link or text — look for any reference to creating an account
    await expect(
      page.getByText(/sign up|create.*account|don't have an account/i).first()
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Signup
// ---------------------------------------------------------------------------

test.describe("Signup (/signup)", () => {
  test("renders without errors", async ({ page }) => {
    const response = await page.goto("/signup");
    expect(response?.status()).toBeLessThan(400);

    // Should have a heading about creating account or signing up
    await expect(
      page.getByRole("heading").first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("OAuth and email options visible", async ({ page }) => {
    await page.goto("/signup");

    await expect(page.getByRole("button", { name: /google/i })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole("button", { name: /github/i })).toBeVisible();
  });

  test("has login link for existing users", async ({ page }) => {
    await page.goto("/signup");

    await expect(
      page.getByRole("link", { name: /sign in|log in|already have/i })
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Forgot Password
// ---------------------------------------------------------------------------

test.describe("Forgot Password (/forgot-password)", () => {
  test("renders without errors", async ({ page }) => {
    const response = await page.goto("/forgot-password");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading").first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("has email input and submit button", async ({ page }) => {
    await page.goto("/forgot-password");

    await expect(
      page.locator('input[type="email"]')
    ).toBeVisible({ timeout: 10000 });

    // Submit button
    await expect(
      page.getByRole("button", { name: /reset|send|submit/i })
    ).toBeVisible();
  });

  test("has back to login link", async ({ page }) => {
    await page.goto("/forgot-password");

    await expect(
      page.getByRole("link", { name: /login|sign in|back/i })
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Blog
// ---------------------------------------------------------------------------

test.describe("Blog (/blog)", () => {
  test("page loads with heading", async ({ page }) => {
    const response = await page.goto("/blog");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading", { name: /articles/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("blog posts render as cards", async ({ page }) => {
    await page.goto("/blog");

    // At least one blog post card should be visible (or an empty state)
    const blogCard = page.locator("article, [class*='card'], a[href*='/blog/']").first();
    const emptyText = page.getByText(/no (articles|posts)/i);

    await expect(blogCard.or(emptyText)).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Changelog
// ---------------------------------------------------------------------------

test.describe("Changelog (/changelog)", () => {
  test("page loads with heading", async ({ page }) => {
    const response = await page.goto("/changelog");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading", { name: /changelog/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("version entries are visible", async ({ page }) => {
    await page.goto("/changelog");

    // At least one version heading should be present
    await expect(
      page.getByText(/v\d+\.\d+/i).first()
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// System Status
// ---------------------------------------------------------------------------

test.describe("Status (/status)", () => {
  test("page loads with heading", async ({ page }) => {
    const response = await page.goto("/status");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading", { name: /system status/i })
    ).toBeVisible({ timeout: 10000 });
  });

  test("service list shows all three services", async ({ page }) => {
    await page.goto("/status");

    await expect(page.getByText("Core (Event Store)")).toBeVisible({ timeout: 15000 });
    await expect(page.getByText("Query Service")).toBeVisible();
    await expect(page.getByText("Control Plane", { exact: true }).first()).toBeVisible();
  });

  test("status badges render for each service", async ({ page }) => {
    await page.goto("/status");

    // Each service should have a status badge (Operational, Degraded, Down, or Checking)
    const badges = page.getByText(/operational|degraded|down|checking/i);
    await expect(badges.first()).toBeVisible({ timeout: 20000 });
  });

  test("incident history section is present", async ({ page }) => {
    await page.goto("/status");

    await expect(page.getByText("Incident History")).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Privacy Policy
// ---------------------------------------------------------------------------

test.describe("Privacy (/privacy)", () => {
  test("page loads without error", async ({ page }) => {
    const response = await page.goto("/privacy");
    expect(response?.status()).toBeLessThan(400);

    // Should have a heading about privacy
    await expect(
      page.getByRole("heading", { name: /privacy/i }).first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("has substantive content", async ({ page }) => {
    await page.goto("/privacy");

    // Key privacy sections should be present
    await expect(
      page.getByText(/data collection|information we collect/i).first()
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Terms of Service
// ---------------------------------------------------------------------------

test.describe("Terms (/terms)", () => {
  test("page loads without error", async ({ page }) => {
    const response = await page.goto("/terms");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading", { name: /terms/i }).first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("has substantive content", async ({ page }) => {
    await page.goto("/terms");

    await expect(
      page.getByText(/acceptable use|account/i).first()
    ).toBeVisible({ timeout: 10000 });
  });
});

// ---------------------------------------------------------------------------
// Solutions - Quant Intelligence
// ---------------------------------------------------------------------------

test.describe("Quant Intelligence (/solutions/quant-intelligence)", () => {
  test("page loads without error", async ({ page }) => {
    const response = await page.goto("/solutions/quant-intelligence");
    expect(response?.status()).toBeLessThan(400);

    await expect(
      page.getByRole("heading").first()
    ).toBeVisible({ timeout: 10000 });
  });

  test("shows performance metrics", async ({ page }) => {
    await page.goto("/solutions/quant-intelligence");

    await expect(page.getByText("11.9μs").first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/469K/i).first()).toBeVisible();
  });
});
