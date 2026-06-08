"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { ArrowDown, Check, Sparkles } from "lucide-react";
import { siteConfig } from "@/lib/config";
import { type Catalog, indexByTier } from "@/lib/pricing-catalog";

// Ranked by the backend `subscription_tier` value (billingTier), not the public
// marketing id. `scale` has no backend tier yet (011 owns it) so it ranks above growth.
// Ranks both the canonical 011 tiers AND the legacy billingTier aliases, since
// a tenant's subscription_tier may be either.
const TIER_RANK: Record<string, number> = {
  "self-host": 0,
  free: 0,
  indie: 1,
  starter: 1,
  studio: 2,
  growth: 2,
  scale: 3,
  enterprise: 4,
};

interface PlanCardsProps {
  currentPlan?: string;
  isYearly?: boolean;
  catalog?: Catalog | null;
  onUpgrade?: (planName: string, billingPeriod: "monthly" | "annual") => void;
}

export function PlanCards({
  currentPlan = "free",
  isYearly = false,
  catalog,
  onUpgrade,
}: PlanCardsProps) {
  // Dashboard only offers checkout for tiers with a backend billing tier.
  // Self-Host has no checkout; surface it on /pricing instead.
  const plans = siteConfig.pricing.filter((p) => !p.isSelfHost);
  const currentRank = TIER_RANK[currentPlan] ?? 0;
  // Live LemonSqueezy prices (source of truth) keyed by public tier id.
  const prices = indexByTier(catalog ?? null);

  return (
    <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-4">
      {plans.map((plan) => {
        // Match the live `subscription_tier` against either the canonical tier
        // id or the legacy billingTier.
        const planBillingTier = plan.billingTier ?? plan.tier;
        const planRank = TIER_RANK[plan.tier] ?? TIER_RANK[planBillingTier] ?? 0;
        const isCurrent = plan.tier === currentPlan || planBillingTier === currentPlan;
        const isAbove = planRank > currentRank;
        const isBelow = planRank < currentRank;
        const isPopular = plan.isPopular;
        const cat = prices[plan.tier];
        const displayPrice = isYearly
          ? (cat?.annual?.per_month ?? plan.yearlyPrice)
          : (cat?.monthly?.formatted ?? plan.price);
        const annualTotal = cat?.annual?.formatted;

        return (
          <Card
            key={plan.name}
            className={cn(
              "relative overflow-hidden transition-all",
              isPopular && "border-primary shadow-lg",
              isCurrent && "ring-2 ring-primary"
            )}
          >
            {/* Popular badge */}
            {isPopular && !isCurrent && (
              <div className="absolute -right-8 top-4 rotate-45 bg-primary px-10 py-1 text-xs font-medium text-primary-foreground">
                Popular
              </div>
            )}

            {/* Current badge */}
            {isCurrent && (
              <div className="absolute left-4 top-4">
                <span className="rounded-full bg-primary/10 px-3 py-1 text-xs font-medium text-primary">
                  Current Plan
                </span>
              </div>
            )}

            <CardHeader className={cn("pb-2", isCurrent && "pt-12")}>
              <CardTitle className="text-sm font-medium text-muted-foreground">
                {plan.name}
              </CardTitle>
              <div className="flex items-baseline gap-1">
                <span className="text-4xl font-bold">{displayPrice}</span>
                {displayPrice !== "Custom" && (
                  <span className="text-muted-foreground">/{plan.period}</span>
                )}
              </div>
              {isYearly && plan.price !== "$0" && plan.price !== "Custom" && (
                <p className="text-xs text-muted-foreground">
                  {annualTotal
                    ? `billed annually (${annualTotal}/yr)`
                    : `billed annually (${plan.yearlyPrice}/mo × 12)`}
                </p>
              )}
              <p className="text-sm text-muted-foreground">{plan.description}</p>
            </CardHeader>

            <CardContent className="space-y-4">
              {/* Features */}
              <ul className="space-y-2">
                {plan.features.map((feature) => (
                  <li key={feature} className="flex items-center gap-2 text-sm">
                    <Check className="h-4 w-4 shrink-0 text-primary" />
                    <span>{feature}</span>
                  </li>
                ))}
              </ul>

              {/* CTA */}
              {isCurrent ? (
                <Button className="w-full" variant="outline" disabled>
                  Current Plan
                </Button>
              ) : plan.isEnterprise ? (
                <Button
                  className="w-full"
                  variant="outline"
                  onClick={() => onUpgrade?.(planBillingTier, isYearly ? "annual" : "monthly")}
                >
                  Contact Sales
                </Button>
              ) : isAbove ? (
                <Button
                  className="w-full"
                  variant={isPopular ? "default" : "outline"}
                  onClick={() => onUpgrade?.(planBillingTier, isYearly ? "annual" : "monthly")}
                >
                  {isPopular && <Sparkles className="mr-2 h-4 w-4" />}
                  Upgrade
                </Button>
              ) : isBelow && planBillingTier !== "free" ? (
                <Button
                  className="w-full"
                  variant="ghost"
                  onClick={() => onUpgrade?.(planBillingTier, isYearly ? "annual" : "monthly")}
                >
                  <ArrowDown className="mr-2 h-4 w-4" />
                  Downgrade
                </Button>
              ) : (
                // Free plan when user is on a paid plan — no button, cancel via portal
                <p className="text-center text-xs text-muted-foreground">
                  Cancel via Manage Subscription
                </p>
              )}
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
