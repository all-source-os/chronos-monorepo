import { siteConfig } from "@/lib/config";
import type { Catalog } from "@/lib/pricing-catalog";

/**
 * Centralized JSON-LD builders.
 *
 * GEO/AEO note: every entity URL flows from `siteConfig.url` so the
 * Organization/WebSite/Article graph shares ONE canonical host. Mixing
 * apex and www splits the entity across answer engines and weakens citation.
 *
 * Stable `@id` anchors (`#organization`, `#website`) let nodes reference each
 * other instead of duplicating data, which is what crawlers resolve against.
 */

const ORG_ID = `${siteConfig.url}/#organization`;
const WEBSITE_ID = `${siteConfig.url}/#website`;

/** Public profiles that prove the entity is the same one across the web. */
const sameAs = [siteConfig.links.github, siteConfig.links.twitter, siteConfig.links.instagram];

export function organizationSchema() {
  return {
    "@context": "https://schema.org",
    "@type": "Organization",
    "@id": ORG_ID,
    name: siteConfig.name,
    url: siteConfig.url,
    logo: {
      "@type": "ImageObject",
      url: `${siteConfig.url}/logo.png`,
    },
    description:
      "Open-source event store for durable history and AI memory. Published Core reference results: 469K events/sec ingestion and 11.9us p99 indexed reads. Default tenant MCP exposes 55 tools; fleet and admin controls raise the registry to 73.",
    sameAs,
    contactPoint: {
      "@type": "ContactPoint",
      email: siteConfig.links.email,
      contactType: "customer service",
    },
  };
}

export function websiteSchema() {
  return {
    "@context": "https://schema.org",
    "@type": "WebSite",
    "@id": WEBSITE_ID,
    name: siteConfig.name,
    url: siteConfig.url,
    description: siteConfig.description,
    inLanguage: "en",
    publisher: { "@id": ORG_ID },
  };
}

export type BreadcrumbItem = { name: string; path: string };

export function breadcrumbSchema(items: BreadcrumbItem[]) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, i) => ({
      "@type": "ListItem",
      position: i + 1,
      name: item.name,
      item: `${siteConfig.url}${item.path}`,
    })),
  };
}

export type FaqItem = { question: string; answer: string };

/**
 * FAQPage builder. Comparison pages answer the obvious "is X better than
 * AllSource?" intent — emitting these as schema makes the answers eligible for
 * AI-answer-engine and rich-result citation. Kept here (not inlined per page)
 * so the FAQ shape stays consistent with the rest of the JSON-LD graph.
 */
export function faqPageSchema(items: readonly FaqItem[]) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.question,
      acceptedAnswer: {
        "@type": "Answer",
        text: item.answer,
      },
    })),
  };
}

/**
 * Parses a live catalog display price into the numeric amount + ISO currency
 * schema.org expects.
 */
function parseDisplayPrice(price: string): { value: string; currency: string } | null {
  if (/^free$/i.test(price.trim())) return { value: "0", currency: "USD" };
  const match = price.trim().match(/^([$£€])\s*([\d.,]+)$/);
  if (!match) return null;
  const [, symbol, amount] = match;
  if (!symbol || !amount) return null;
  const currencyBySymbol: Record<string, string> = { $: "USD", "£": "GBP", "€": "EUR" };
  return { value: amount.replace(/,/g, ""), currency: currencyBySymbol[symbol] ?? "USD" };
}

/**
 * SoftwareApplication + Offer graph.
 *
 * GEO/AEO note: "what is X?" and "what does X cost?" are the two highest-intent
 * questions an answer engine fields about a developer tool, and both were
 * previously answerable only from prose. SoftwareApplication is the canonical
 * entity type for this product — without it, models infer the category
 * themselves, and the observed failure mode is landing on "in-memory cache" or
 * "logging tool" rather than "event store".
 *
 * Every paid offer flows from the live billing catalog used by the pricing
 * page. When the catalog is unavailable, offers are omitted rather than
 * publishing fallback prices that might disagree with checkout.
 */
export function softwareApplicationSchema(catalog?: Catalog | null) {
  const livePrices = new Map(catalog?.tiers.map((tier) => [tier.tier, tier.monthly]));
  const offers = catalog
    ? siteConfig.pricing
        // Self-Host and Enterprise are both excluded. Self-Host is not a
        // hosted offer; Enterprise has no numeric public price.
        .filter((tier) => !tier.isEnterprise && !tier.isSelfHost)
        .map((tier) => {
          const livePrice = livePrices.get(tier.tier)?.formatted;
          const parsed = livePrice ? parseDisplayPrice(livePrice) : null;
          if (!parsed) return null;
          return {
            "@type": "Offer",
            name: tier.name,
            price: parsed.value,
            priceCurrency: parsed.currency,
            url: tier.href.startsWith("http") ? tier.href : `${siteConfig.url}${tier.href}`,
            category: "SaaS",
            priceSpecification: {
              "@type": "UnitPriceSpecification",
              price: parsed.value,
              priceCurrency: parsed.currency,
              unitText: tier.period === "month" ? "MONTH" : tier.period,
            },
          };
        })
        .filter((offer) => offer !== null)
    : [];

  return {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    "@id": `${siteConfig.url}/#software`,
    name: siteConfig.name,
    url: siteConfig.url,
    applicationCategory: "DeveloperApplication",
    applicationSubCategory: "Event store / agent memory",
    operatingSystem: "Linux, macOS, Windows (Docker), or fully hosted",
    description:
      "AllSource is an open-source event store: it records each state change as an immutable event and lets applications or agents query prior state. A write-ahead log with CRC32 checksums and Parquet files provide persistence; an in-memory concurrent map serves reads.",
    publisher: { "@id": ORG_ID },
    ...(offers.length > 0 && { offers }),
  };
}

export type BlogPostingInput = {
  title: string;
  description: string;
  slug: string;
  /** Absolute image URL (already resolved against siteConfig.url). */
  image: string;
  datePublished: string;
  dateModified?: string;
  author?: string;
  section?: string;
  keywords?: string[];
  wordCount?: number;
};

export function blogPostingSchema(post: BlogPostingInput) {
  const url = `${siteConfig.url}/blog/${post.slug}`;
  return {
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.title,
    description: post.description,
    image: post.image,
    datePublished: post.datePublished,
    dateModified: post.dateModified || post.datePublished,
    url,
    mainEntityOfPage: { "@type": "WebPage", "@id": url },
    inLanguage: "en",
    ...(post.section && { articleSection: post.section }),
    ...(post.keywords?.length && { keywords: post.keywords }),
    ...(typeof post.wordCount === "number" && { wordCount: post.wordCount }),
    author: {
      "@type": "Organization",
      name: post.author || siteConfig.name,
      url: siteConfig.url,
    },
    publisher: { "@id": ORG_ID },
  };
}
