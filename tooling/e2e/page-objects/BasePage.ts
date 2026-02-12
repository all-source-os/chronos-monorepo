import type { Locator, Page } from "@playwright/test";

/**
 * Base Page Object Model class
 * Provides common functionality for all page objects
 */
export abstract class BasePage {
  protected readonly page: Page;
  protected readonly baseURL: string;

  constructor(page: Page, baseURL = "http://localhost:3000") {
    this.page = page;
    this.baseURL = baseURL;
  }

  /**
   * Navigate to the page
   */
  abstract goto(): Promise<void>;

  /**
   * Wait for the page to be loaded
   */
  abstract waitForPageLoad(): Promise<void>;

  /**
   * Get page title
   */
  async getTitle(): Promise<string> {
    return this.page.title();
  }

  /**
   * Wait for element to be visible
   */
  async waitForElement(locator: Locator, timeout = 5000): Promise<void> {
    await locator.waitFor({ state: "visible", timeout });
  }

  /**
   * Click element and wait for navigation
   */
  async clickAndWait(locator: Locator): Promise<void> {
    await Promise.all([this.page.waitForLoadState("networkidle"), locator.click()]);
  }

  /**
   * Fill input field
   */
  async fill(locator: Locator, value: string): Promise<void> {
    await locator.fill(value);
  }

  /**
   * Get text content
   */
  async getText(locator: Locator): Promise<string> {
    return (await locator.textContent()) || "";
  }

  /**
   * Check if element is visible
   */
  async isVisible(locator: Locator): Promise<boolean> {
    return locator.isVisible();
  }

  /**
   * Wait for success message
   */
  async waitForSuccessMessage(message?: string): Promise<void> {
    const successLocator = this.page.locator('[role="alert"]', {
      hasText: message,
    });
    await this.waitForElement(successLocator);
  }

  /**
   * Wait for error message
   */
  async waitForErrorMessage(message?: string): Promise<void> {
    const errorLocator = this.page.locator('[role="alert"]', {
      hasText: message,
    });
    await this.waitForElement(errorLocator);
  }

  /**
   * Take screenshot
   */
  async screenshot(name: string): Promise<void> {
    await this.page.screenshot({ path: `screenshots/${name}.png`, fullPage: true });
  }

  /**
   * Wait for network idle
   */
  async waitForNetworkIdle(): Promise<void> {
    await this.page.waitForLoadState("networkidle");
  }

  /**
   * Reload page
   */
  async reload(): Promise<void> {
    await this.page.reload();
    await this.waitForPageLoad();
  }
}
