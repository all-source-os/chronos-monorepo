import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
  Section,
} from "@allsource/ui";
import { siteConfig } from "@/lib/config";
import { type FaqItem, faqPageSchema } from "@/lib/structured-data";

type FaqProps = {
  /** Defaults to the site-wide FAQ set. Pass a page-specific set to avoid
   * emitting the same FAQPage graph on two different URLs — duplicate schema
   * across URLs splits the signal instead of reinforcing it. */
  items?: FaqItem[];
  title?: string;
  subtitle?: string;
};

export default function FAQ({
  items = siteConfig.faqs,
  title = "FAQ",
  subtitle = "Frequently asked questions",
}: FaqProps) {
  // Built from the SAME array the accordion renders, so the schema and the
  // visible text can never disagree — answer engines discount pages whose
  // markup claims more than the page shows.
  const faqJsonLd = faqPageSchema(items);

  return (
    <Section title={title} subtitle={subtitle}>
      <script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqJsonLd) }}
      />
      <div className="mx-auto my-12 md:max-w-[800px]">
        <Accordion
          type="single"
          collapsible
          className="flex w-full flex-col items-center justify-center space-y-2"
        >
          {items.map((faq) => (
            <AccordionItem
              key={faq.question}
              value={faq.question}
              className="w-full border rounded-lg overflow-hidden"
            >
              <AccordionTrigger className="px-4">{faq.question}</AccordionTrigger>
              <AccordionContent className="px-4">{faq.answer}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
      </div>
    </Section>
  );
}
