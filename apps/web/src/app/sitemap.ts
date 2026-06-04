import type { MetadataRoute } from "next";
import { headers } from "next/headers";
import { getBlogPosts } from "@/lib/blog";
import { integrations } from "@/lib/integrations";

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const allPosts = await getBlogPosts();
  const headersList = await headers();
  const domain = headersList.get("host") as string;
  const protocol = "https";
  const base = `${protocol}://${domain}`;
  const now = new Date();

  // Static marketing pages
  const staticPages = [
    { url: base, priority: 1.0, changeFrequency: "weekly" as const },
    { url: `${base}/pricing`, priority: 0.9, changeFrequency: "monthly" as const },
    { url: `${base}/blog`, priority: 0.8, changeFrequency: "weekly" as const },
    { url: `${base}/docs`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/docs/api`, priority: 0.7, changeFrequency: "monthly" as const },
    { url: `${base}/docs/mcp`, priority: 0.7, changeFrequency: "monthly" as const },
    { url: `${base}/docs/tenant-setup`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/docs/chronis`, priority: 0.6, changeFrequency: "monthly" as const },
    { url: `${base}/docs/prime`, priority: 0.9, changeFrequency: "weekly" as const },
    { url: `${base}/docs/prime/quickstart`, priority: 0.9, changeFrequency: "monthly" as const },
    { url: `${base}/docs/prime/concepts`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/docs/prime/mcp`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/docs/prime/http`, priority: 0.7, changeFrequency: "monthly" as const },
    { url: `${base}/docs/prime/embedded`, priority: 0.7, changeFrequency: "monthly" as const },
    { url: `${base}/use-cases`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/compare/eventstoredb`, priority: 0.7, changeFrequency: "monthly" as const },
    {
      url: `${base}/event-sourcing-for-ai-agents`,
      priority: 0.9,
      changeFrequency: "weekly" as const,
    },
    { url: `${base}/vs/mem0`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/vs/letta`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/vs/zep`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/platform/event-sourcing`, priority: 0.9, changeFrequency: "monthly" as const },
    {
      url: `${base}/platform/stream-processing`,
      priority: 0.8,
      changeFrequency: "monthly" as const,
    },
    { url: `${base}/platform/prime`, priority: 0.8, changeFrequency: "monthly" as const },
    { url: `${base}/prime`, priority: 0.9, changeFrequency: "weekly" as const },
    { url: `${base}/install`, priority: 0.9, changeFrequency: "weekly" as const },
    { url: `${base}/solutions/agent-memory`, priority: 0.9, changeFrequency: "monthly" as const },
    {
      url: `${base}/solutions/audit-compliance`,
      priority: 0.8,
      changeFrequency: "monthly" as const,
    },
    {
      url: `${base}/solutions/real-time-analytics`,
      priority: 0.8,
      changeFrequency: "monthly" as const,
    },
    {
      url: `${base}/solutions/financial-services`,
      priority: 0.7,
      changeFrequency: "monthly" as const,
    },
    { url: `${base}/solutions/iot-telemetry`, priority: 0.7, changeFrequency: "monthly" as const },
    {
      url: `${base}/solutions/multi-tenant-saas`,
      priority: 0.7,
      changeFrequency: "monthly" as const,
    },
    {
      url: `${base}/solutions/quant-intelligence`,
      priority: 0.7,
      changeFrequency: "monthly" as const,
    },
    { url: `${base}/changelog`, priority: 0.5, changeFrequency: "weekly" as const },
    { url: `${base}/privacy`, priority: 0.3, changeFrequency: "yearly" as const },
    { url: `${base}/terms`, priority: 0.3, changeFrequency: "yearly" as const },
    { url: `${base}/status`, priority: 0.4, changeFrequency: "daily" as const },
  ];

  // Per-tool install pages — derived from the integration data module so a new
  // tool lands in the sitemap with no edit here.
  const installPages = integrations.map((integration) => ({
    url: `${base}/install/${integration.slug}`,
    priority: 0.8,
    changeFrequency: "monthly" as const,
  }));

  // Blog posts
  const blogPages = allPosts.map((post) => ({
    url: `${base}/blog/${post.slug}`,
    lastModified: new Date(post.publishedAt),
    priority: 0.7,
    changeFrequency: "monthly" as const,
  }));

  return [
    ...staticPages.map((p) => ({ ...p, lastModified: now })),
    ...installPages.map((p) => ({ ...p, lastModified: now })),
    ...blogPages,
  ];
}
