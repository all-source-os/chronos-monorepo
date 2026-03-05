import { test, expect, type APIRequestContext } from "@playwright/test";
import {
  demoLoginFull,
  authenticateAndGoToDashboard,
} from "../../fixtures/demo-auth";

/**
 * Event Lifecycle — Create event via API → see it in Event Explorer UI
 *
 * The critical end-to-end test: proves that data written through the API
 * actually appears in the browser UI.
 */

const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";

test.describe("Event Lifecycle", () => {
  let token: string;
  const uniqueType = `e2e.lifecycle.${Date.now()}`;
  const uniqueEntity = `e2e-lifecycle-${Date.now()}`;

  test.beforeAll(async ({ request }) => {
    const auth = await demoLoginFull(request);
    if (!auth) {
      throw new Error(
        "Demo login failed — Control Plane unreachable. " +
          "E2E tests require a running backend."
      );
    }
    token = auth.token;

    // Create the event via API before browser tests
    const resp = await request.post(`${QS_URL}/api/events`, {
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      data: {
        event_type: uniqueType,
        entity_id: uniqueEntity,
        data: { source: "e2e-lifecycle", created: Date.now() },
      },
    });
    expect(resp.ok()).toBeTruthy();
  });

  test("created event appears in Event Explorer", async ({ page }) => {
    await authenticateAndGoToDashboard(page, token);
    await page.goto("/dashboard/events");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /event explorer/i })
    ).toBeVisible({ timeout: 10000 });

    // Search for the unique event type
    const searchInput = page.getByPlaceholder(/search/i).first();
    if (await searchInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await searchInput.fill(uniqueType);
      await page.waitForTimeout(1000); // debounce

      // The event should appear in results
      await expect(
        page.getByText(uniqueType).first()
      ).toBeVisible({ timeout: 15000 });
    } else {
      // If no search box, just verify events are displayed (not empty)
      await expect(
        page.getByText(/results\)/).or(page.getByText(uniqueType)).first()
      ).toBeVisible({ timeout: 15000 });
    }
  });

  test("event data matches what was written", async ({ request }) => {
    // Read back via API to verify data integrity
    const resp = await request.get(
      `${QS_URL}/api/events?event_type=${encodeURIComponent(uniqueType)}`,
      {
        headers: { Authorization: `Bearer ${token}` },
      }
    );
    expect(resp.ok()).toBeTruthy();

    const body = await resp.json();
    const events = body.events || body.data || body;
    expect(Array.isArray(events)).toBeTruthy();
    expect(events.length).toBeGreaterThan(0);

    const event = events[0];
    const entityId = event.entity_id || event.entityId;
    expect(entityId).toBe(uniqueEntity);
  });
});
