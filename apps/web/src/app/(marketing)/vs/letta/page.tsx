import type { Metadata } from "next";
import { breadcrumbSchema, faqPageSchema } from "@/lib/structured-data";
import { constructMetadata } from "@/lib/utils";
import { VsPageBody } from "../_components/VsPageBody";
import { competitors } from "../_data/competitors";

const competitor = competitors.letta;

export const metadata: Metadata = constructMetadata({
  title: `AllSource vs ${competitor.name}`,
  description: competitor.metaDescription,
  canonical: `/vs/${competitor.slug}`,
});

export default function VsLettaPage() {
  const breadcrumb = breadcrumbSchema([
    { name: "Home", path: "/" },
    { name: "Compare", path: "/event-sourcing-for-ai-agents" },
    { name: `AllSource vs ${competitor.name}`, path: `/vs/${competitor.slug}` },
  ]);
  const faq = faqPageSchema(competitor.faqs);

  return (
    <>
      <script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumb) }}
      />
      <script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faq) }}
      />
      <VsPageBody competitor={competitor} />
    </>
  );
}
