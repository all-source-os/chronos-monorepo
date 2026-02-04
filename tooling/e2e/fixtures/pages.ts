import { test as base } from "@playwright/test";
import { UITestPage } from "../page-objects";

/**
 * Custom fixtures for page objects
 * This allows tests to use page objects directly via dependency injection
 */
type PageFixtures = {
  uiTestPage: UITestPage;
};

export const test = base.extend<PageFixtures>({
  uiTestPage: async ({ page }, use) => {
    const uiTestPage = new UITestPage(page);
    await use(uiTestPage);
  },
});

export { expect } from "@playwright/test";
