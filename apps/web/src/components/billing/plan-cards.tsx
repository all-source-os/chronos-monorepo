"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Check, Sparkles } from "lucide-react";
import { siteConfig } from "@/lib/config";

interface PlanCardsProps {
  currentPlan?: string;
  isYearly?: boolean;
  onUpgrade?: (planName: string, billingPeriod: "monthly" | "annual") => void;
}

export function PlanCards({ currentPlan = "free", isYearly = false, onUpgrade }: PlanCardsProps) {
  const plans = siteConfig.pricing;

  const getPlanTier = (name: string) => {
    const lower = name.toLowerCase();
    if (lower.includes("enterprise")) return "enterprise";
    if (lower.includes("team")) return "growth";
    return "free";
  };

  return (
    <div className="grid gap-6 md:grid-cols-3">
      {plans.map((plan) => {
        const planTier = getPlanTier(plan.name);
        const isCurrent = planTier === currentPlan;
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
            {isPopular && (
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
              <Button
                className="w-full"
                variant={isPopular ? "default" : "outline"}
                disabled={isCurrent}
                onClick={() => onUpgrade?.(planTier, isYearly ? "annual" : "monthly")}
              >
                {isCurrent ? (
                  "Current Plan"
                ) : plan.price === "Custom" ? (
                  "Contact Sales"
                ) : (
                  <>
                    {isPopular && <Sparkles className="mr-2 h-4 w-4" />}
                    {plan.buttonText}
                  </>
                )}
              </Button>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}
