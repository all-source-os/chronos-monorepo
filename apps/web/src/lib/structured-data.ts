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
      "AI-native event store for temporal data intelligence. 469K events/sec, 11.9us queries, 43 MCP tools.",
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
