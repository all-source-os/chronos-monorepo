import type { Locator, Page } from "@playwright/test";
import { BasePage } from "./BasePage";

/**
 * Page Object Model for AllSource Event Store Demo Page
 * Updated for new interactive demo UI with feature cards
 */
export class DemoPage extends BasePage {
  // Feature cards
  private readonly eventsCard: Locator;
  private readonly queriesCard: Locator;
  private readonly metricsCard: Locator;
  private readonly projectionsCard: Locator;
  private readonly securityCard: Locator;
  private readonly timeTravelCard: Locator;
  private readonly analyticsCard: Locator;
  private readonly pipelinesCard: Locator;

  // Hero section elements
  private readonly heroTitle: Locator;
  private readonly heroDescription: Locator;
  private readonly performanceStats: Locator;

  // Event Ingestion Demo
  private readonly generateEcommerceBtn: Locator;
  private readonly generateIoTBtn: Locator;
  private readonly eventStream: Locator;
  private readonly eventStreamTitle: Locator;
  private readonly totalBatchesStat: Locator;
  private readonly totalEventsStat: Locator;
  private readonly successRateStat: Locator;

  // Query Demo
  private readonly queryByEntityBtn: Locator;
  private readonly queryByTypeBtn: Locator;
  private readonly queryByTimeRangeBtn: Locator;
  private readonly queryResults: Locator;
  private readonly queryResultsTitle: Locator;

  // Metrics Demo
  private readonly systemMetricsTitle: Locator;
  private readonly refreshMetricsBtn: Locator;
  private readonly metricCards: Locator;
  private readonly ingestionRateMetric: Locator;
  private readonly queryLatencyMetric: Locator;
  private readonly activeConnectionsMetric: Locator;
  private readonly storageUsedMetric: Locator;
  private readonly totalEventsMetric: Locator;

  constructor(page: Page) {
    super(page);

    // Feature cards - using button roles
    this.eventsCard = page.getByRole("button", { name: /Event Ingestion/ });
    this.queriesCard = page.getByRole("button", { name: /Powerful Queries/ });
    this.metricsCard = page.getByRole("button", { name: /Real-time Metrics/ });
    this.projectionsCard = page.getByRole("button", { name: /Projections/ });
    this.securityCard = page.getByRole("button", { name: /Enterprise Security/ });
    this.timeTravelCard = page.getByRole("button", { name: /Time Travel/ });
    this.analyticsCard = page.getByRole("button", { name: /Event Analytics/ });
    this.pipelinesCard = page.getByRole("button", { name: /Event Pipelines/ });

    // Hero section
    this.heroTitle = page.getByRole("heading", { name: /AllSource Event Store/ });
    this.heroDescription = page.getByText(/Experience the next generation of event streaming/);
    this.performanceStats = page.locator(".grid.grid-cols-3.gap-6.mt-12");

    // Event Ingestion Demo
    this.generateEcommerceBtn = page.getByRole("button", {
      name: /Generate E-Commerce Events/i,
    });
    this.generateIoTBtn = page.getByRole("button", {
      name: /Generate IoT Sensor Data/i,
    });
    this.eventStream = page
      .getByRole("heading", { name: /^Event Stream$/i, level: 3 })
      .locator("..");
    this.eventStreamTitle = page.getByRole("heading", { name: /^Event Stream$/i, level: 3 });
    this.totalBatchesStat = page.getByText(/Total Batches/i);
    this.totalEventsStat = page.getByText(/Total Events/i);
    this.successRateStat = page.getByText(/Success Rate/i);

    // Query Demo
    this.queryByEntityBtn = page.getByRole("button", { name: /^By Entity$/i });
    this.queryByTypeBtn = page.getByRole("button", { name: /^By Type$/i });
    this.queryByTimeRangeBtn = page.getByRole("button", { name: /^Time Range$/i });
    this.queryResults = page.getByRole("heading", { name: /Query Results/i });
    this.queryResultsTitle = page.getByRole("heading", { name: /Query Results/i });

    // Metrics Demo
    this.systemMetricsTitle = page.getByRole("heading", { name: /System Metrics/i });
    this.refreshMetricsBtn = page.getByRole("button", { name: /Refresh/i });
    this.metricCards = page.locator(".grid.grid-cols-1.md\\:grid-cols-2.lg\\:grid-cols-3");
    this.ingestionRateMetric = page.getByText(/Ingestion Rate/i);
    this.queryLatencyMetric = page.getByText(/Query Latency/i);
    this.activeConnectionsMetric = page.getByText(/Active Connections/i);
    this.storageUsedMetric = page.getByText(/Storage Used/i);
    this.totalEventsMetric = page.getByText(/Total Events/i);
  }

  async goto(): Promise<void> {
    await this.page.goto(`${this.baseURL}/demo`);
  }

  async waitForPageLoad(): Promise<void> {
    await this.page.waitForLoadState("domcontentloaded");
    await this.heroTitle.waitFor({ state: "visible", timeout: 10000 });
  }

  // Feature card navigation methods
  async clickEventsCard(): Promise<void> {
    await this.eventsCard.click();
    await this.page.waitForTimeout(500); // Wait for animation
  }

  async clickQueriesCard(): Promise<void> {
    await this.queriesCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickMetricsCard(): Promise<void> {
    await this.metricsCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickProjectionsCard(): Promise<void> {
    await this.projectionsCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickSecurityCard(): Promise<void> {
    await this.securityCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickTimeTravelCard(): Promise<void> {
    await this.timeTravelCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickAnalyticsCard(): Promise<void> {
    await this.analyticsCard.click();
    await this.page.waitForTimeout(500);
  }

  async clickPipelinesCard(): Promise<void> {
    await this.pipelinesCard.click();
    await this.page.waitForTimeout(500);
  }

  // Hero section methods
  async getPerformanceStats(): Promise<{ [key: string]: string }> {
    const stats = await this.performanceStats.locator("div").allTextContents();
    return {
      eventsIngested: stats[0] || "",
      queryLatency: stats[1] || "",
      compression: stats[2] || "",
    };
  }

  // Event Ingestion Demo methods
  async generateEcommerceEvents(): Promise<void> {
    await this.generateEcommerceBtn.click();
    await this.page.waitForTimeout(1000); // Wait for API call
  }

  async generateIoTEvents(): Promise<void> {
    await this.generateIoTBtn.click();
    await this.page.waitForTimeout(1000);
  }

  async isEventStreamVisible(): Promise<boolean> {
    return this.isVisible(this.eventStreamTitle);
  }

  async getEventStreamItemCount(): Promise<number> {
    // Count the event stream items in the list
    const items = await this.page.locator(".space-y-2 > div").count();
    return items;
  }

  async areEventStatsVisible(): Promise<boolean> {
    const batchesVisible = await this.isVisible(this.totalBatchesStat);
    const eventsVisible = await this.isVisible(this.totalEventsStat);
    const successVisible = await this.isVisible(this.successRateStat);
    return batchesVisible && eventsVisible && successVisible;
  }

  // Query Demo methods
  async queryByEntity(): Promise<void> {
    await this.queryByEntityBtn.click();
    await this.page.waitForTimeout(1500); // Wait for query execution
  }

  async queryByType(): Promise<void> {
    await this.queryByTypeBtn.click();
    await this.page.waitForTimeout(1500);
  }

  async queryByTimeRange(): Promise<void> {
    await this.queryByTimeRangeBtn.click();
    await this.page.waitForTimeout(1500);
  }

  async areQueryResultsVisible(): Promise<boolean> {
    return this.isVisible(this.queryResults);
  }

  async getQueryResultsCount(): Promise<number> {
    // Count the query result cards
    try {
      const count = await this.page.locator(".grid.grid-cols-1.gap-3 > div").count();
      return count;
    } catch {
      return 0;
    }
  }

  // Metrics Demo methods
  async isSystemMetricsTitleVisible(): Promise<boolean> {
    return this.isVisible(this.systemMetricsTitle);
  }

  async refreshMetrics(): Promise<void> {
    await this.refreshMetricsBtn.click();
    await this.page.waitForTimeout(1000);
  }

  async getMetricsCount(): Promise<number> {
    try {
      const count = await this.metricCards.locator("> div").count();
      return count;
    } catch {
      return 0;
    }
  }

  async areAllMetricsVisible(): Promise<boolean> {
    const checks = await Promise.all([
      this.isVisible(this.ingestionRateMetric),
      this.isVisible(this.queryLatencyMetric),
      this.isVisible(this.activeConnectionsMetric),
      this.isVisible(this.storageUsedMetric),
      this.isVisible(this.totalEventsMetric),
    ]);
    return checks.every((check) => check);
  }

  // Utility method to check if a card is active (has gradient background)
  async isFeatureCardActive(cardName: string): Promise<boolean> {
    let card: Locator;
    switch (cardName.toLowerCase()) {
      case "events":
        card = this.eventsCard;
        break;
      case "queries":
        card = this.queriesCard;
        break;
      case "metrics":
        card = this.metricsCard;
        break;
      case "projections":
        card = this.projectionsCard;
        break;
      case "security":
        card = this.securityCard;
        break;
      case "timetravel":
        card = this.timeTravelCard;
        break;
      case "analytics":
        card = this.analyticsCard;
        break;
      case "pipelines":
        card = this.pipelinesCard;
        break;
      default:
        return false;
    }

    const classes = await card.getAttribute("class");
    return classes?.includes("bg-gradient-to-br") || false;
  }
}
