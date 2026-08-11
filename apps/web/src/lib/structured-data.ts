import { siteConfig } from "@/lib/config";

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
      "AI-native event store for temporal data intelligence. 469K events/sec, 11.9us queries, 73 MCP tools.",
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
    inLanguage: "en-US",
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
export function faqPageSchema(items: FaqItem[]) {
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
 * Parses a display price string ("$19", "Free", "Custom") into the numeric
 * amount + ISO currency schema.org expects.
 *
 * Deliberately driven off the SAME `siteConfig.pricing` strings the pricing
 * page renders: schema that disagrees with the visible page is a spam signal to
 * answer engines and gets the whole graph discounted. Returns null for
 * non-numeric tiers ("Custom") so they are omitted rather than guessed at.
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
 * Every offer flows from `siteConfig.pricing`, so a tier change updates the
 * schema and the page together. Tiers with non-numeric prices (Enterprise
 * "Custom") are omitted from `offers` rather than invented.
 */
export function softwareApplicationSchema() {
  const offers = siteConfig.pricing
    // Self-Host and Enterprise are both excluded, for opposite reasons.
    // Self-Host would emit a $0 Offer while /pricing deliberately advertises no
    // free plan — schema contradicting the visible page is a spam signal, and
    // the Apache-2.0 run-it-yourself story is told in prose in the FAQ instead.
    // Enterprise has no numeric price to state.
    .filter((tier) => !tier.isEnterprise && !tier.isSelfHost)
    .map((tier) => {
      const parsed = parseDisplayPrice(tier.price);
      if (!parsed) return null;
      return {
        "@type": "Offer",
        name: tier.name,
        price: parsed.value,
        priceCurrency: parsed.currency,
        url: tier.href.startsWith("http") ? tier.href : `${siteConfig.url}${tier.href}`,
        category: tier.isSelfHost ? "Self-hosted" : "SaaS",
        ...(parsed.value !== "0" && {
          priceSpecification: {
            "@type": "UnitPriceSpecification",
            price: parsed.value,
            priceCurrency: parsed.currency,
            unitText: tier.period === "month" ? "MONTH" : tier.period,
          },
        }),
      };
    })
    .filter((offer) => offer !== null);

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
      "AllSource is an AI-native event store: it records every state change as an immutable event and lets an agent query any point in its own history. Durable by design — a write-ahead log with CRC32 checksums, Parquet columnar persistence, and an in-memory concurrent map for reads.",
    publisher: { "@id": ORG_ID },
    offers,
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
    inLanguage: "en-US",
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
