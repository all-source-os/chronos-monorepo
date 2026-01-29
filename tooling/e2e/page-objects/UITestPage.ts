import type { Locator, Page } from "@playwright/test";
import { BasePage } from "./BasePage";

/**
 * Page Object Model for UI Component Test Page
 */
export class UITestPage extends BasePage {
  // Page title
  private readonly pageTitle: Locator;

  // Buttons section
  private readonly defaultButton: Locator;
  private readonly primaryButton: Locator;
  private readonly secondaryButton: Locator;
  private readonly destructiveButton: Locator;
  private readonly outlineButton: Locator;
  private readonly ghostButton: Locator;
  private readonly linkButton: Locator;
  private readonly smallButton: Locator;
  private readonly largeButton: Locator;

  // Badges section
  private readonly defaultBadge: Locator;
  private readonly primaryBadge: Locator;
  private readonly secondaryBadge: Locator;
  private readonly destructiveBadge: Locator;
  private readonly outlineBadge: Locator;

  // Cards section
  private readonly card1: Locator;
  private readonly card2: Locator;
  private readonly card1Title: Locator;
  private readonly card2Title: Locator;

  constructor(page: Page) {
    super(page);

    // Page title
    this.pageTitle = page.locator("h1", { hasText: "UI Component Test" });

    // Buttons - using getByRole for better accessibility
    this.defaultButton = page.getByRole("button", { name: "Default" });
    this.primaryButton = page.getByRole("button", { name: "Primary" });
    this.secondaryButton = page.getByRole("button", { name: "Secondary" });
    this.destructiveButton = page.getByRole("button", { name: "Destructive" });
    this.outlineButton = page.getByRole("button", { name: "Outline" });
    this.ghostButton = page.getByRole("button", { name: "Ghost" });
    this.linkButton = page.getByRole("button", { name: "Link" });
    this.smallButton = page.getByRole("button", { name: "Small" });
    this.largeButton = page.getByRole("button", { name: "Large" });

    // Badges - using text locators
    this.defaultBadge = page.locator('div[class*="badge"]', { hasText: "Default" }).first();
    this.primaryBadge = page.locator('div[class*="badge"]', { hasText: "Primary" }).first();
    this.secondaryBadge = page.locator('div[class*="badge"]', { hasText: "Secondary" }).first();
    this.destructiveBadge = page.locator('div[class*="badge"]', { hasText: "Destructive" }).first();
    this.outlineBadge = page.locator('div[class*="badge"]', { hasText: "Outline" }).first();

    // Cards
    this.card1 = page.locator('[class*="card"]').first();
    this.card2 = page.locator('[class*="card"]').nth(1);
    this.card1Title = this.card1.locator("h2");
    this.card2Title = this.card2.locator("h2");
  }

  async goto(): Promise<void> {
    await this.page.goto(`${this.baseURL}/ui-test`);
  }

  async waitForPageLoad(): Promise<void> {
    await this.page.waitForLoadState("domcontentloaded");
    await this.pageTitle.waitFor({ state: "visible" });
  }

  // Button interaction methods
  async clickDefaultButton(): Promise<void> {
    await this.defaultButton.click();
  }

  async clickPrimaryButton(): Promise<void> {
    await this.primaryButton.click();
  }

  async clickSecondaryButton(): Promise<void> {
    await this.secondaryButton.click();
  }

  async clickDestructiveButton(): Promise<void> {
    await this.destructiveButton.click();
  }

  async isButtonVisible(buttonName: string): Promise<boolean> {
    const button = this.page.getByRole("button", { name: buttonName });
    return this.isVisible(button);
  }

  // Badge verification methods
  async isBadgeVisible(badgeText: string): Promise<boolean> {
    const badge = this.page.locator('div[class*="badge"]', { hasText: badgeText }).first();
    return this.isVisible(badge);
  }

  async getBadgeCount(): Promise<number> {
    return this.page.locator('div[class*="badge"]').count();
  }

  // Card verification methods
  async isCard1Visible(): Promise<boolean> {
    return this.isVisible(this.card1);
  }

  async isCard2Visible(): Promise<boolean> {
    return this.isVisible(this.card2);
  }

  async getCard1Title(): Promise<string> {
    return this.getText(this.card1Title);
  }

  async getCard2Title(): Promise<string> {
    return this.getText(this.card2Title);
  }

  async getCardsCount(): Promise<number> {
    return this.page.locator('[class*="card"]').count();
  }
}
