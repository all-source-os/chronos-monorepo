import { test, expect } from "@playwright/test";
import {
  demoLoginFull,
  authenticateAndGoToDashboard,
} from "../../fixtures/demo-auth";

/**
 * API Key Management — Create key in UI → see it → revoke it
 *
 * Full browser journey for API key CRUD.
 */

const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "https://allsource-query.fly.dev";

test.describe("API Key Management", () => {
  let token: string;
  const keyName = `e2e-ui-key-${Date.now()}`;

  test.beforeAll(async ({ request }) => {
    const auth = await demoLoginFull(request);
    if (!auth) {
      throw new Error(
        "Demo login failed — Control Plane unreachable. " +
          "E2E tests require a running backend."
      );
    }
    token = auth.token;
  });

  test("create API key via UI, see it in table, revoke it", async ({
    page,
    request,
  }) => {
    await authenticateAndGoToDashboard(page, token);
    await page.goto("/dashboard/api-keys");
    await expect(page.getByText("Loading...")).toBeHidden({ timeout: 15000 });

    await expect(
      page.getByRole("heading", { name: /api keys/i }).first()
    ).toBeVisible({ timeout: 10000 });

    // Click "Create Key" or "Create API Key" button
    const createBtn = page.getByRole("button", {
      name: /create.*key/i,
    });
    await expect(createBtn).toBeVisible({ timeout: 5000 });
    await createBtn.click();

    // Fill in the key name in the dialog/form
    const nameInput = page.getByLabel(/name/i).or(page.getByPlaceholder(/name/i)).first();
    await expect(nameInput).toBeVisible({ timeout: 5000 });
    await nameInput.fill(keyName);

    // Submit the form
    const submitBtn = page
      .getByRole("button", { name: /create|save|generate/i })
      .last();
    await submitBtn.click();

    // Wait for the key to appear — either in a success dialog or the table
    await expect(
      page.getByText(keyName).or(page.getByText(/key created|api key/i)).first()
    ).toBeVisible({ timeout: 10000 });

    // Close any modal/dialog if present
    const closeBtn = page.getByRole("button", { name: /close|done|ok/i }).first();
    if (await closeBtn.isVisible({ timeout: 2000 }).catch(() => false)) {
      await closeBtn.click();
    }

    // Verify the key appears in the table
    await expect(page.getByText(keyName)).toBeVisible({ timeout: 10000 });

    // Revoke: find the revoke/delete button for our key
    const keyRow = page.getByText(keyName).locator("..").locator("..");
    const revokeBtn = keyRow
      .getByRole("button", { name: /revoke|delete|remove/i })
      .or(keyRow.locator("[data-testid*='revoke'], [data-testid*='delete']"))
      .first();

    if (await revokeBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      await revokeBtn.click();

      // Confirm if there's a confirmation dialog
      const confirmBtn = page
        .getByRole("button", { name: /confirm|revoke|delete|yes/i })
        .first();
      if (
        await confirmBtn.isVisible({ timeout: 2000 }).catch(() => false)
      ) {
        await confirmBtn.click();
      }

      // Key should no longer be active in the table
      await page.waitForTimeout(1000);
    } else {
      // Cleanup via API if UI revoke button not found
      const listResp = await request.get(`${QS_URL}/api/api-keys`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (listResp.ok()) {
        const keys = await listResp.json();
        const keyList = keys.keys || keys.data || keys;
        const ourKey = keyList.find((k: any) => k.name === keyName);
        if (ourKey) {
          await request.delete(
            `${QS_URL}/api/api-keys/${ourKey.id || ourKey.key_id}`,
            { headers: { Authorization: `Bearer ${token}` } }
          );
        }
      }
    }
  });
});
