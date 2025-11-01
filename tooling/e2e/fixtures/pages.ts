import { test as base } from "@playwright/test";
import { DemoPage, UITestPage } from "../page-objects";

/**
 * Custom fixtures for page objects
 * This allows tests to use page objects directly via dependency injection
 */
type PageFixtures = {
  demoPage: DemoPage;
  uiTestPage: UITestPage;
};

export const test = base.extend<PageFixtures>({
  demoPage: async ({ page }, use) => {
    const demoPage = new DemoPage(page);
    await use(demoPage);
  },

  uiTestPage: async ({ page }, use) => {
    const uiTestPage = new UITestPage(page);
    await use(uiTestPage);
  },
});

export { expect } from "@playwright/test";
