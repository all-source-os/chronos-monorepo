import { Badge, Section } from "@allsource/ui";
import { Check, Clock3, DatabaseZap, History, ShieldCheck } from "lucide-react";
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

const benefits = [
  "60 days of hosted Scale access",
  "Founder-led integration session",
  "Two focused product feedback calls",
  "Direct engineering help through first working recall",
];

const evidence = [
  {
    icon: ShieldCheck,
    label: "Provenance",
    value: "Trace recalled facts to source events",
  },
  {
    icon: History,
    label: "Time travel",
    value: "Reconstruct what agent knew at any moment",
  },
  {
    icon: DatabaseZap,
    label: "Durability",
    value: "WAL + Parquet history survives restarts",
  },
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
    <div className="relative overflow-hidden bg-[#0E1A2A] text-white">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 opacity-45 [background-image:linear-gradient(rgba(56,214,200,0.07)_1px,transparent_1px),linear-gradient(90deg,rgba(47,140,255,0.07)_1px,transparent_1px)] [background-size:42px_42px]"
      />
      <div
        aria-hidden="true"
        className="pointer-events-none absolute -left-48 top-20 h-[34rem] w-[34rem] rounded-full bg-[#2F8CFF]/15 blur-[120px]"
      />

      <Section className="relative py-14 sm:py-20 lg:py-24">
        <div className="grid items-start gap-12 lg:grid-cols-[minmax(0,0.9fr)_minmax(34rem,1.1fr)] lg:gap-16">
          <div className="lg:sticky lg:top-28">
            <Badge className="border border-[#38D6C8]/35 bg-[#38D6C8]/10 font-mono text-[11px] uppercase tracking-[0.18em] text-[#72EFE2]">
              Founding cohort · 5 teams
            </Badge>
            <h1 className="mt-6 max-w-3xl text-4xl font-semibold leading-[1.02] tracking-[-0.045em] sm:text-6xl lg:text-7xl">
              Build agent memory that can explain itself.
            </h1>
            <p className="mt-6 max-w-2xl text-lg leading-8 text-slate-300 sm:text-xl">
              AllSource is recruiting five builders whose AI agents need memory to survive restarts,
              preserve provenance, and reconstruct prior state.
            </p>

            <div className="relative mt-10 border-l border-[#38D6C8]/40 pl-8">
              <span className="absolute -left-1.5 top-1 h-3 w-3 rounded-full border-2 border-[#0E1A2A] bg-[#38D6C8]" />
              <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#72EFE2]">
                design_partner.offer_created
              </p>
              <ul className="mt-5 space-y-3">
                {benefits.map((benefit) => (
                  <li key={benefit} className="flex items-start gap-3 text-slate-100">
                    <Check className="mt-1 h-4 w-4 shrink-0 text-[#38D6C8]" aria-hidden="true" />
                    <span>{benefit}</span>
                  </li>
                ))}
              </ul>
              <span className="absolute -bottom-1.5 -left-1.5 h-3 w-3 rounded-full border-2 border-[#0E1A2A] bg-[#2F8CFF]" />
            </div>

            <div className="mt-10 grid gap-3 sm:grid-cols-3 lg:grid-cols-1 xl:grid-cols-3">
              {evidence.map((item) => (
                <div key={item.label} className="border-t border-white/15 pt-4">
                  <item.icon className="h-5 w-5 text-[#2F8CFF]" aria-hidden="true" />
                  <p className="mt-3 font-mono text-[10px] uppercase tracking-[0.17em] text-slate-400">
                    {item.label}
                  </p>
                  <p className="mt-1 text-sm leading-6 text-slate-200">{item.value}</p>
                </div>
              ))}
            </div>

            <p className="mt-8 text-sm leading-6 text-slate-400">
              This is product research, not a review program. No endorsement, testimonial, or public
              mention required.
            </p>
          </div>

          <DesignPartnerForm campaignSource={campaignSource} />
        </div>
      </Section>

      <Section className="relative border-t border-white/10 py-16 sm:py-20">
        <div className="grid gap-10 lg:grid-cols-2 lg:gap-20">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.18em] text-[#72EFE2]">
              Fit check
            </p>
            <h2 className="mt-3 text-3xl font-semibold tracking-tight">
              Bring a real agent system.
            </h2>
            <p className="mt-4 max-w-xl leading-7 text-slate-300">
              Best fit: production or serious pre-production agents where a lost decision, stale
              summary, or untraceable recall creates real engineering cost.
            </p>
            <ul className="mt-6 space-y-3 text-slate-200">
              <li className="flex gap-3">
                <Check className="mt-1 h-4 w-4 text-[#38D6C8]" />
                You can integrate within 60 days.
              </li>
              <li className="flex gap-3">
                <Check className="mt-1 h-4 w-4 text-[#38D6C8]" />
                You can share concrete memory failure cases.
              </li>
              <li className="flex gap-3">
                <Check className="mt-1 h-4 w-4 text-[#38D6C8]" />
                You can join one integration session and two feedback calls.
              </li>
            </ul>
          </div>
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.18em] text-[#F0B44D]">
              What happens next
            </p>
            <ol className="mt-5 space-y-6">
              {[
                ["01", "Application review", "We reply within five business days."],
                [
                  "02",
                  "Technical fit call",
                  "We map your agent's write, recall, and provenance path.",
                ],
                [
                  "03",
                  "Working integration",
                  "We help reach one measurable cross-session memory flow.",
                ],
              ].map(([step, title, body]) => (
                <li key={step} className="grid grid-cols-[2.5rem_1fr] gap-4">
                  <span className="font-mono text-sm text-[#F0B44D]">{step}</span>
                  <div>
                    <h3 className="font-medium text-white">{title}</h3>
                    <p className="mt-1 text-sm leading-6 text-slate-400">{body}</p>
                  </div>
                </li>
              ))}
            </ol>
          </div>
        </div>
      </Section>

      <Section className="relative border-t border-white/10 py-14">
        <div className="flex flex-col gap-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <div className="flex items-center gap-2 text-slate-300">
              <Clock3 className="h-4 w-4 text-[#38D6C8]" aria-hidden="true" />
              <span className="font-mono text-xs uppercase tracking-[0.16em]">About 3 minutes</span>
            </div>
            <p className="mt-2 text-sm text-slate-400">Questions? Email hello@all-source.xyz.</p>
          </div>
          <div className="flex gap-5 text-sm">
            <Link
              href="/docs/prime"
              className="text-slate-300 underline decoration-white/25 underline-offset-4 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#38D6C8]"
            >
              Technical docs
            </Link>
            <Link
              href="/privacy#design-partner-applications"
              className="text-slate-300 underline decoration-white/25 underline-offset-4 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#38D6C8]"
            >
              Application privacy
            </Link>
          </div>
        </div>
      </Section>
    </div>
  );
}
