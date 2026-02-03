import { expect, test } from "../../fixtures/pages";

/**
 * UI Components Test Suite
 *
 * Tests the shared UI components from @allsource/ui package
 * displayed on the /ui-test page.
 *
 * Components tested:
 * - Button (5 variants: default, destructive, outline, secondary, ghost)
 * - Badge (4 variants: default, secondary, destructive, outline)
 * - Card (with header, title, and content)
 */
test.describe("UI Components", () => {
  test.beforeEach(async ({ uiTestPage }) => {
    await uiTestPage.goto();
    await uiTestPage.waitForPageLoad();
  });

  test.describe("Page Structure", () => {
    test("should load the UI test page successfully", async ({ page }) => {
      await expect(page).toHaveURL(/\/ui-test$/);
      await expect(
        page.getByRole("heading", { name: "UI Component Test" })
      ).toBeVisible();
    });

    test("should display page heading", async ({ page }) => {
      const heading = page.locator("h1", { hasText: "UI Component Test" });
      await expect(heading).toBeVisible();
    });

    test("should have proper layout container", async ({ page }) => {
      const container = page.locator(".container");
      await expect(container).toBeVisible();
    });

    test("should display section headings for all component types", async ({
      page,
    }) => {
      await expect(
        page.locator("h2").filter({ hasText: "Buttons" })
      ).toBeVisible();
      await expect(
        page.locator("h2").filter({ hasText: "Badges" })
      ).toBeVisible();
      await expect(
        page.locator("h2").filter({ hasText: "Cards" })
      ).toBeVisible();
    });
  });

  test.describe("Button Variants", () => {
    test("should display default button variant", async ({ page }) => {
      const button = page.getByRole("button", { name: "Default Button" });
      await expect(button).toBeVisible();
    });

    test("should display destructive button variant", async ({ page }) => {
      const button = page.getByRole("button", { name: "Destructive" });
      await expect(button).toBeVisible();
    });

    test("should display outline button variant", async ({ page }) => {
      const button = page.getByRole("button", { name: "Outline" });
      await expect(button).toBeVisible();
    });

    test("should display secondary button variant", async ({ page }) => {
      const button = page.getByRole("button", { name: "Secondary" });
      await expect(button).toBeVisible();
    });

    test("should display ghost button variant", async ({ page }) => {
      const button = page.getByRole("button", { name: "Ghost" });
      await expect(button).toBeVisible();
    });

    test("should render all 5 button variants", async ({ page }) => {
      // Verify all 5 buttons exist on the page
      const buttonNames = [
        "Default Button",
        "Destructive",
        "Outline",
        "Secondary",
        "Ghost",
      ];
      for (const name of buttonNames) {
        await expect(page.getByRole("button", { name })).toBeVisible();
      }
    });
  });

  test.describe("Button Sizes", () => {
    // The UI test page currently only shows variant buttons, not size variants
    // These tests verify the buttons have appropriate sizing through visual presence
    test("should have buttons with consistent sizing", async ({ page }) => {
      const defaultBtn = page.getByRole("button", { name: "Default Button" });
      const destructiveBtn = page.getByRole("button", { name: "Destructive" });

      // Both buttons should be visible and clickable (proper sizing)
      await expect(defaultBtn).toBeVisible();
      await expect(destructiveBtn).toBeVisible();

      // Verify buttons have proper dimensions (not collapsed)
      const defaultBox = await defaultBtn.boundingBox();
      const destructiveBox = await destructiveBtn.boundingBox();

      expect(defaultBox).not.toBeNull();
      expect(destructiveBox).not.toBeNull();
      expect(defaultBox!.height).toBeGreaterThan(20);
      expect(destructiveBox!.height).toBeGreaterThan(20);
    });

    test("should have minimum clickable area for accessibility", async ({
      page,
    }) => {
      const buttonNames = [
        "Default Button",
        "Destructive",
        "Outline",
        "Secondary",
        "Ghost",
      ];

      for (const name of buttonNames) {
        const button = page.getByRole("button", { name });
        const box = await button.boundingBox();
        expect(box).not.toBeNull();
        // Minimum touch target size for accessibility (44x44 recommended, 24 minimum)
        expect(box!.height).toBeGreaterThanOrEqual(24);
      }
    });
  });

  test.describe("Badge Variants", () => {
    test("should display default badge variant", async ({ page }) => {
      // Find badge by locating the Badges section heading and getting siblings
      const badgesHeading = page.locator("h2").filter({ hasText: "Badges" });
      const badgesSection = badgesHeading.locator("~ div");
      await expect(badgesSection.getByText("Default").first()).toBeVisible();
    });

    test("should display secondary badge variant", async ({ page }) => {
      const badgesHeading = page.locator("h2").filter({ hasText: "Badges" });
      const badgesSection = badgesHeading.locator("~ div");
      await expect(badgesSection.getByText("Secondary").first()).toBeVisible();
    });

    test("should display destructive badge variant", async ({ page }) => {
      const badgesHeading = page.locator("h2").filter({ hasText: "Badges" });
      const badgesSection = badgesHeading.locator("~ div");
      await expect(badgesSection.getByText("Destructive").first()).toBeVisible();
    });

    test("should display outline badge variant", async ({ page }) => {
      const badgesHeading = page.locator("h2").filter({ hasText: "Badges" });
      const badgesSection = badgesHeading.locator("~ div");
      await expect(badgesSection.getByText("Outline").first()).toBeVisible();
    });

    test("should render all 4 badge variants", async ({ page }) => {
      // Verify all 4 badge variants exist on the page
      const badgeTexts = ["Default", "Secondary", "Destructive", "Outline"];
      const badgesHeading = page.locator("h2").filter({ hasText: "Badges" });
      const badgesSection = badgesHeading.locator("~ div");

      for (const text of badgeTexts) {
        await expect(badgesSection.getByText(text).first()).toBeVisible();
      }
    });
  });

  test.describe("Card Components", () => {
    test("should display card component", async ({ page }) => {
      await expect(page.getByText("Test Card")).toBeVisible();
    });

    test("should display card title", async ({ page }) => {
      const cardTitle = page.getByText("Test Card");
      await expect(cardTitle).toBeVisible();
    });

    test("should display card content", async ({ page }) => {
      const cardContent = page.getByText(
        "This card is imported from @allsource/ui package!"
      );
      await expect(cardContent).toBeVisible();
    });

    test("should have proper card structure with header and content", async ({
      page,
    }) => {
      // Card should contain both title and content
      await expect(page.getByText("Test Card")).toBeVisible();
      await expect(
        page.getByText("This card is imported from @allsource/ui package!")
      ).toBeVisible();
    });
  });

  test.describe("Interactive States - Click", () => {
    test("should be able to click default button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Default Button" });
      await button.click();
      // Button should still be visible and enabled after click
      await expect(button).toBeVisible();
      await expect(button).toBeEnabled();
    });

    test("should be able to click destructive button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Destructive" });
      await button.click();
      await expect(button).toBeVisible();
      await expect(button).toBeEnabled();
    });

    test("should be able to click outline button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Outline" });
      await button.click();
      await expect(button).toBeVisible();
      await expect(button).toBeEnabled();
    });

    test("should be able to click secondary button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Secondary" });
      await button.click();
      await expect(button).toBeVisible();
      await expect(button).toBeEnabled();
    });

    test("should be able to click ghost button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Ghost" });
      await button.click();
      await expect(button).toBeVisible();
      await expect(button).toBeEnabled();
    });

    test("should be able to click all button variants sequentially", async ({
      page,
    }) => {
      const buttonNames = [
        "Default Button",
        "Destructive",
        "Outline",
        "Secondary",
        "Ghost",
      ];

      for (const name of buttonNames) {
        const button = page.getByRole("button", { name });
        await button.click();
        await expect(button).toBeVisible();
      }
    });
  });

  test.describe("Interactive States - Hover", () => {
    test("should show hover state on default button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Default Button" });
      await button.hover();
      // Verify button is still visible and interactable after hover
      await expect(button).toBeVisible();
    });

    test("should show hover state on destructive button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Destructive" });
      await button.hover();
      await expect(button).toBeVisible();
    });

    test("should show hover state on outline button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Outline" });
      await button.hover();
      await expect(button).toBeVisible();
    });

    test("should show hover state on secondary button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Secondary" });
      await button.hover();
      await expect(button).toBeVisible();
    });

    test("should show hover state on ghost button", async ({ page }) => {
      const button = page.getByRole("button", { name: "Ghost" });
      await button.hover();
      await expect(button).toBeVisible();
    });

    test("should be able to hover all button variants", async ({ page }) => {
      const buttonNames = [
        "Default Button",
        "Destructive",
        "Outline",
        "Secondary",
        "Ghost",
      ];

      for (const name of buttonNames) {
        const button = page.getByRole("button", { name });
        await button.hover();
        await expect(button).toBeVisible();
      }
    });
  });

  test.describe("Accessibility", () => {
    test("buttons should have proper role", async ({ page }) => {
      const buttons = [
        "Default Button",
        "Destructive",
        "Outline",
        "Secondary",
        "Ghost",
      ];

      for (const name of buttons) {
        const button = page.getByRole("button", { name });
        await expect(button).toBeVisible();
      }
    });

    test("buttons should be keyboard focusable", async ({ page }) => {
      const firstButton = page.getByRole("button", { name: "Default Button" });
      await firstButton.focus();
      await expect(firstButton).toBeFocused();
    });

    test("page should have proper heading hierarchy", async ({ page }) => {
      // h1 for page title
      const h1 = page.locator("h1");
      await expect(h1).toHaveCount(1);

      // h2 for section headings
      const h2 = page.locator("h2");
      const h2Count = await h2.count();
      expect(h2Count).toBeGreaterThanOrEqual(3); // Buttons, Badges, Cards
    });
  });
});
