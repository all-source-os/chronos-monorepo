import { Section } from "@allsource/ui";
import { ArrowRight, Check, Clock3 } from "lucide-react";
import Link from "next/link";
import { constructMetadata } from "@/lib/utils";
import { DesignPartnerForm } from "./design-partner-form";

export const metadata = constructMetadata({
  title: "AI Agent Memory Design Partner Program",
  description:
    "Five teams. Sixty hosted days. Founder-led integration for AI agents that need durable memory, provenance, and time travel.",
  canonical: "/design-partners",
  image: "/design-partners/opengraph-image",
  imageAlt: "AllSource design partner program for durable AI agent memory",
});

type SearchValue = string | string[] | undefined;

interface DesignPartnerPageProps {
  searchParams: Promise<Record<string, SearchValue>>;
}

function first(value: SearchValue): string {
  return Array.isArray(value) ? value[0] || "" : value || "";
}

const offerTerms = [
  ["60 days", "Hosted AllSource Scale access"],
  ["1 working session", "Founder-led integration on your memory flow"],
  ["2 feedback calls", "Direct product and engineering support"],
  ["5 business days", "Application decision by email"],
];

const fitSignals = [
  "A production or serious pre-production agent",
  "One concrete memory failure you can reproduce",
  "Capacity to integrate within 60 days",
];

const nextSteps = [
  ["01", "Application review", "We check technical fit and reply within five business days."],
  ["02", "Memory-flow call", "We map the write, recall, provenance, and replay path."],
  ["03", "Working integration", "We help ship one measurable cross-session memory flow."],
];

export default async function DesignPartnerPage({ searchParams }: DesignPartnerPageProps) {
  const params = await searchParams;
  const campaignSource = {
    source: first(params.utm_source),
    medium: first(params.utm_medium),
    campaign: first(params.utm_campaign),
    content: first(params.utm_content),
    term: first(params.utm_term),
  };

  return (
    <div className="bg-[#0B1522] text-white">
      <Section className="py-8 sm:py-14 lg:py-16">
        <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,0.88fr)_minmax(32rem,1.12fr)] lg:gap-12 xl:gap-16">
          <div className="contents lg:sticky lg:top-24 lg:block">
            <div className="order-1">
              <p className="border-l-2 border-[#33C6D0] pl-3 text-sm font-semibold text-[#72E7E2]">
                Design partner program <span className="text-slate-300">· Five teams</span>
              </p>
              <h1 className="mt-5 max-w-[12ch] text-4xl font-semibold leading-[1.02] tracking-[-0.035em] text-white sm:mt-6 sm:text-5xl lg:text-6xl">
                Fix the memory failures your agent already has.
              </h1>
              <p className="mt-4 max-w-2xl text-lg leading-8 text-slate-300 sm:mt-6">
                Bring one failing recall flow. We&apos;ll help replace fragile summaries, vectors,
                or local files with durable memory that keeps its source and can reconstruct past
                state.
              </p>

              <dl className="mt-6 border-y border-white/15 sm:mt-8">
                {offerTerms.map(([term, detail]) => (
                  <div
                    key={term}
                    className="grid grid-cols-[7.75rem_1fr] gap-4 border-b border-white/10 py-2.5 last:border-b-0 sm:grid-cols-[9rem_1fr] sm:py-3"
                  >
                    <dt className="font-semibold text-white">{term}</dt>
                    <dd className="text-sm leading-6 text-slate-300">{detail}</dd>
                  </div>
                ))}
              </dl>
            </div>

            <div className="order-3 lg:mt-7">
              <h2 className="text-base font-semibold text-white">Good fit when you have:</h2>
              <ul className="mt-3 space-y-2">
                {fitSignals.map((signal) => (
                  <li
                    key={signal}
                    className="flex items-start gap-3 text-sm leading-6 text-slate-300"
                  >
                    <Check className="mt-1 h-4 w-4 shrink-0 text-[#33C6D0]" aria-hidden="true" />
                    <span>{signal}</span>
                  </li>
                ))}
              </ul>

              <section
                className="mt-8 border border-white/15 bg-[#111E2E] p-5"
                aria-labelledby="integration-result-title"
              >
                <h2 id="integration-result-title" className="text-sm font-semibold text-white">
                  Target integration result
                </h2>
                <div className="mt-3 flex flex-wrap items-center gap-2 font-mono text-[11px] text-[#72E7E2]">
                  <span>decision.recorded</span>
                  <ArrowRight className="h-3.5 w-3.5 text-slate-500" aria-hidden="true" />
                  <span>memory.recalled</span>
                  <ArrowRight className="h-3.5 w-3.5 text-slate-500" aria-hidden="true" />
                  <span>state.reconstructed</span>
                </div>
                <p className="mt-3 text-sm leading-6 text-slate-300">
                  Recalled facts point back to source events; replay answers what agent knew before
                  later correction.
                </p>
              </section>

              <p className="mt-6 text-sm leading-6 text-slate-400">
                Product research only. No review, endorsement, testimonial, or public mention
                required.
              </p>
            </div>
          </div>

          <div className="order-2 lg:order-none">
            <DesignPartnerForm campaignSource={campaignSource} />
          </div>
        </div>
      </Section>

      <Section className="border-t border-white/10 py-12 sm:py-16">
        <div className="flex flex-col gap-4 border-b border-white/10 pb-7 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="text-sm font-semibold text-[#72E7E2]">After you apply</p>
            <h2 className="mt-2 text-3xl font-semibold tracking-[-0.025em] text-white">
              One path from failure to working recall.
            </h2>
          </div>
          <div className="flex items-center gap-2 text-sm text-slate-300">
            <Clock3 className="h-4 w-4 text-[#33C6D0]" aria-hidden="true" />
            <span>Application takes about three minutes</span>
          </div>
        </div>

        <ol className="grid sm:grid-cols-3 sm:divide-x sm:divide-white/10">
          {nextSteps.map(([step, title, body]) => (
            <li
              key={step}
              className="border-b border-white/10 py-6 last:border-b-0 sm:border-b-0 sm:px-6 sm:first:pl-0 sm:last:pr-0"
            >
              <span className="font-mono text-xs text-[#C99A46]">{step}</span>
              <h3 className="mt-3 font-semibold text-white">{title}</h3>
              <p className="mt-2 text-sm leading-6 text-slate-400">{body}</p>
            </li>
          ))}
        </ol>

        <div className="flex flex-col gap-4 border-t border-white/10 pt-7 text-sm sm:flex-row sm:items-center sm:justify-between">
          <p className="text-slate-400">Questions? Email hello@all-source.xyz.</p>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <Link
              href="/docs/prime"
              className="min-h-6 text-slate-200 underline decoration-white/30 underline-offset-4 hover:text-white focus-visible:rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#33C6D0] focus-visible:ring-offset-2 focus-visible:ring-offset-[#0B1522]"
            >
              Read technical docs
            </Link>
            <Link
              href="/privacy#design-partner-applications"
              className="min-h-6 text-slate-200 underline decoration-white/30 underline-offset-4 hover:text-white focus-visible:rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#33C6D0] focus-visible:ring-offset-2 focus-visible:ring-offset-[#0B1522]"
            >
              Review application privacy
            </Link>
          </div>
        </div>
      </Section>
    </div>
  );
}
