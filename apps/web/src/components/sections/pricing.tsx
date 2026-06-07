"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import { Check } from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import { useState } from "react";
import { FaStar } from "react-icons/fa";
import { siteConfig } from "@/lib/config";
import { type Catalog, indexByTier } from "@/lib/pricing-catalog";

// Cards shown in the top row (everything except Enterprise, which renders as a
// full-width strip below the cards per PRICING_EXPOSURE_PLAN.md §3).
const cardTiers = siteConfig.pricing.filter((p) => !p.isEnterprise);
const enterpriseTier = siteConfig.pricing.find((p) => p.isEnterprise);

// `catalog` carries live LemonSqueezy prices (source of truth). When present,
// its prices win over the static config prices; config is only a fallback for
// when the catalog is unreachable.
export default function PricingSection({ catalog }: { catalog?: Catalog | null }) {
  const [isMonthly, setIsMonthly] = useState(true);
  const prices = indexByTier(catalog ?? null);

  return (
    <Section
      title="Pricing"
      subtitle="Pay for the events your agents write. Self-host for free if you'd rather."
    >
      {/* Billing toggle — single unambiguous label per state. */}
      <div className="flex justify-center mb-10">
        <div className="inline-flex items-center rounded-full border-2 border-primary/20 bg-muted/50 p-1">
          <button
            type="button"
            onClick={() => setIsMonthly(true)}
            className={cn(
              "px-4 py-2 rounded-full text-sm font-semibold transition-all duration-200",
              isMonthly
                ? "bg-primary text-primary-foreground shadow-md"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            Monthly
          </button>
          <button
            type="button"
            onClick={() => setIsMonthly(false)}
            className={cn(
              "px-4 py-2 rounded-full text-sm font-semibold transition-all duration-200 flex items-center gap-1",
              !isMonthly
                ? "bg-primary text-primary-foreground shadow-md"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            Yearly
            <span className="text-xs bg-green-500 text-white px-1.5 py-0.5 rounded-full">-20%</span>
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {cardTiers.map((plan, index) => {
          // LemonSqueezy price (source of truth) with config fallback.
          const cat = prices[plan.tier];
          const monthlyStr = cat?.monthly?.formatted ?? plan.price;
          const yearlyStr = cat?.annual?.per_month ?? plan.yearlyPrice;
          const annualTotal = cat?.annual?.formatted;
          const displayPrice = isMonthly ? monthlyStr : yearlyStr;
          const isNumericPrice = displayPrice.startsWith("$");

          return (
            <motion.div
              key={plan.name}
              initial={{ y: 50, opacity: 1 }}
              whileInView={{ y: 0, opacity: 1 }}
              viewport={{ once: true }}
              transition={{
                duration: 1.6,
                type: "spring",
                stiffness: 100,
                damping: 30,
                delay: 0.4 + index * 0.05,
                opacity: { duration: 0.5 },
              }}
              className={cn(
                "rounded-2xl border-[1px] p-6 text-center lg:flex lg:flex-col lg:justify-between relative",
                plan.isPopular
                  ? "border-primary border-[2px] bg-primary/[0.04]"
                  : "border-border bg-background",
                plan.isSelfHost && "border-dashed"
              )}
            >
              {plan.isPopular && (
                <div className="absolute top-0 right-0 bg-primary py-0.5 px-2 rounded-bl-xl rounded-tr-xl flex items-center">
                  <FaStar className="text-white" />
                  <span className="text-white ml-1 font-sans font-semibold">Popular</span>
                </div>
              )}
              <div>
                <p className="text-base font-semibold text-muted-foreground">{plan.name}</p>
                <p className="mt-6 flex items-end justify-center gap-x-1">
                  <span className="text-5xl font-bold tracking-tight text-foreground">
                    {displayPrice}
                  </span>
                  {isNumericPrice && (
                    <span className="mb-1 text-sm font-semibold leading-6 tracking-wide text-muted-foreground">
                      /mo
                    </span>
                  )}
                </p>
                <p className="text-xs leading-5 text-muted-foreground">
                  {plan.isSelfHost
                    ? plan.period
                    : isNumericPrice
                      ? isMonthly
                        ? "billed monthly"
                        : annualTotal
                          ? `billed annually (${annualTotal}/yr)`
                          : "billed yearly"
                      : ""}
                </p>

                {plan.x402 && (
                  <p className="mt-4 text-xs leading-5 text-muted-foreground">
                    Includes {plan.x402.included}. {plan.x402.overage}.
                  </p>
                )}

                <ul className="mt-5 gap-2 flex flex-col text-left">
                  {plan.features.map((feature) => (
                    <li key={feature} className="flex items-start">
                      <Check className="mr-2 mt-0.5 h-4 w-4 shrink-0 text-primary" />
                      <span className="text-sm">{feature}</span>
                    </li>
                  ))}
                </ul>
              </div>

              <div>
                <hr className="w-full my-4" />
                {/* 011: map tier -> price id. For now route to signup / github / mailto. */}
                <Link
                  href={plan.href}
                  className={cn(
                    buttonVariants({ variant: "outline" }),
                    "group relative w-full gap-2 overflow-hidden text-base font-semibold tracking-tight",
                    "transform-gpu ring-offset-current transition-all duration-300 ease-out hover:ring-2 hover:ring-primary hover:ring-offset-1 hover:bg-primary hover:text-white",
                    plan.isPopular ? "bg-primary text-white" : "bg-background text-foreground"
                  )}
                >
                  {plan.buttonText}
                </Link>
                <p className="mt-6 text-xs leading-5 text-muted-foreground">{plan.description}</p>
              </div>
            </motion.div>
          );
        })}
      </div>

      {/* Enterprise — quieter full-width dark strip below the cards. */}
      {enterpriseTier && (
        <div className="mt-6 flex flex-col items-start justify-between gap-4 rounded-2xl border border-border bg-foreground px-8 py-6 text-background sm:flex-row sm:items-center">
          <div>
            <p className="text-lg font-semibold">{enterpriseTier.name}</p>
            <p className="text-sm opacity-80">
              {enterpriseTier.description} {enterpriseTier.mcp}. Negotiated volume, unlimited
              retention, 24/7 + dedicated SE.
            </p>
          </div>
          {/* 011: map tier -> price id. Enterprise is sales-led, not self-serve checkout. */}
          <Link
            href={enterpriseTier.href}
            className={cn(
              buttonVariants({ variant: "outline" }),
              "shrink-0 bg-background text-foreground hover:bg-background/90"
            )}
          >
            {enterpriseTier.buttonText}
          </Link>
        </div>
      )}
    </Section>
  );
}
