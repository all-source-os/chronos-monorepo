import { test, expect } from "../../fixtures/pages";

test.describe("UI Components Test Page", () => {
  test.beforeEach(async ({ uiTestPage }) => {
    await uiTestPage.goto();
    await uiTestPage.waitForPageLoad();
  });

  test("should load the UI test page successfully", async ({ page }) => {
    await expect(page).toHaveTitle(/AllSource Event Store/i);
    await expect(page).toHaveURL(/\/ui-test$/);
  });

  test("should display page title", async ({ page }) => {
    const title = page.locator("h1", { hasText: "UI Component Test" });
    await expect(title).toBeVisible();
  });

  test.describe("Button Components", () => {
    test("should display all button variants", async ({ uiTestPage }) => {
      await expect(await uiTestPage.isButtonVisible("Default")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Primary")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Secondary")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Destructive")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Outline")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Ghost")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Link")).toBeTruthy();
    });

    test("should display button size variants", async ({ uiTestPage }) => {
      await expect(await uiTestPage.isButtonVisible("Small")).toBeTruthy();
      await expect(await uiTestPage.isButtonVisible("Large")).toBeTruthy();
    });

    test("should be able to click buttons", async ({ uiTestPage, page }) => {
      await uiTestPage.clickDefaultButton();
      // Button should still be visible after click
      await expect(page.getByRole("button", { name: "Default" })).toBeVisible();

      await uiTestPage.clickPrimaryButton();
      await expect(page.getByRole("button", { name: "Primary" })).toBeVisible();
    });
  });

  test.describe("Badge Components", () => {
    test("should display all badge variants", async ({ uiTestPage }) => {
      await expect(await uiTestPage.isBadgeVisible("Default")).toBeTruthy();
      await expect(await uiTestPage.isBadgeVisible("Primary")).toBeTruthy();
      await expect(await uiTestPage.isBadgeVisible("Secondary")).toBeTruthy();
      await expect(await uiTestPage.isBadgeVisible("Destructive")).toBeTruthy();
      await expect(await uiTestPage.isBadgeVisible("Outline")).toBeTruthy();
    });

    test("should display correct number of badges", async ({ uiTestPage }) => {
      const badgeCount = await uiTestPage.getBadgeCount();
      expect(badgeCount).toBeGreaterThan(0);
    });
  });

  test.describe("Card Components", () => {
    test("should display cards", async ({ uiTestPage }) => {
      await expect(await uiTestPage.isCard1Visible()).toBeTruthy();
      await expect(await uiTestPage.isCard2Visible()).toBeTruthy();
    });

    test("should display card titles", async ({ uiTestPage }) => {
      const card1Title = await uiTestPage.getCard1Title();
      const card2Title = await uiTestPage.getCard2Title();

      expect(card1Title).toBeTruthy();
      expect(card2Title).toBeTruthy();
      expect(card1Title.length).toBeGreaterThan(0);
      expect(card2Title.length).toBeGreaterThan(0);
    });

    test("should display correct number of cards", async ({ uiTestPage }) => {
      const cardsCount = await uiTestPage.getCardsCount();
      expect(cardsCount).toBeGreaterThanOrEqual(2);
    });
  });

  test("should have proper layout and styling", async ({ page }) => {
    // Check for container
    const container = page.locator(".container");
    await expect(container).toBeVisible();

    // Check for proper spacing
    const heading = page.locator("h1");
    await expect(heading).toHaveCSS("margin-bottom", /.+/);
  });
});
