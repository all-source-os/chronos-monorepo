"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { ArrowDown, Check, Sparkles } from "lucide-react";
import { siteConfig } from "@/lib/config";

const TIER_RANK: Record<string, number> = { free: 0, growth: 1, enterprise: 2 };

interface PlanCardsProps {
  currentPlan?: string;
  isYearly?: boolean;
  onUpgrade?: (planName: string, billingPeriod: "monthly" | "annual") => void;
}

export function PlanCards({ currentPlan = "free", isYearly = false, onUpgrade }: PlanCardsProps) {
  const plans = siteConfig.pricing;
  const currentRank = TIER_RANK[currentPlan] ?? 0;

  return (
    <div className="grid gap-6 md:grid-cols-3">
      {plans.map((plan) => {
        const planRank = TIER_RANK[plan.tier] ?? 0;
        const isCurrent = plan.tier === currentPlan;
        const isAbove = planRank > currentRank;
        const isBelow = planRank < currentRank;
        const isPopular = plan.isPopular;
        const displayPrice = isYearly ? plan.yearlyPrice : plan.price;

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
                  billed annually ({plan.yearlyPrice}/mo &times; 12)
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
              ) : plan.tier === "enterprise" ? (
                <Button
                  className="w-full"
                  variant="outline"
                  onClick={() => onUpgrade?.(plan.tier, isYearly ? "annual" : "monthly")}
                >
                  Contact Sales
                </Button>
              ) : isAbove ? (
                <Button
                  className="w-full"
                  variant={isPopular ? "default" : "outline"}
                  onClick={() => onUpgrade?.(plan.tier, isYearly ? "annual" : "monthly")}
                >
                  {isPopular && <Sparkles className="mr-2 h-4 w-4" />}
                  Upgrade
                </Button>
              ) : isBelow && plan.tier !== "free" ? (
                <Button
                  className="w-full"
                  variant="ghost"
                  onClick={() => onUpgrade?.(plan.tier, isYearly ? "annual" : "monthly")}
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
