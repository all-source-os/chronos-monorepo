"use client";

import {
  Badge,
  BlurFade,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Calendar, Check, CreditCard, ExternalLink } from "lucide-react";
import { useState } from "react";
import { PlanCards } from "@/components/billing/plan-cards";
import { UsageChart } from "@/components/billing/usage-chart";
import { useAuthStore } from "@/lib/stores/auth-store";

export default function BillingPage() {
  const { tenant } = useAuthStore();
  const [isYearly, setIsYearly] = useState(false);

  // Demo data
  const currentPlan = tenant?.subscription_tier || "free";
  const eventsUsed = tenant?.events_used || 45000;
  const eventsQuota = tenant?.events_quota || 100000;
  const queriesUsed = tenant?.queries_used || 3200;
  const queriesQuota = tenant?.queries_quota || 10000;
  const trialEndsAt = tenant?.trial_ends_at;
  const subscriptionEndsAt = tenant?.subscription_ends_at;

  const handleManageSubscription = () => {
    // In production, this would redirect to LemonSqueezy portal
    window.open("https://billing.lemonsqueezy.com/", "_blank");
  };

  const handleUpgrade = (planName: string) => {
    // In production, this would initiate checkout
    console.log("Upgrading to:", planName);
  };

  const formatDate = (dateStr: string | null) => {
    if (!dateStr) return null;
    return new Date(dateStr).toLocaleDateString("en-US", {
      month: "long",
      day: "numeric",
      year: "numeric",
    });
  };

  return (
    <div className="space-y-8">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Billing & Usage</h1>
            <p className="mt-1 text-muted-foreground">Manage your subscription and monitor usage</p>
          </div>
          {currentPlan !== "free" && (
            <Button variant="outline" onClick={handleManageSubscription}>
              <CreditCard className="mr-2 h-4 w-4" />
              Manage Subscription
              <ExternalLink className="ml-2 h-4 w-4" />
            </Button>
          )}
        </div>
      </BlurFade>

      {/* Trial/Subscription Banner */}
      {trialEndsAt && (
        <BlurFade delay={0.15} inView>
          <Card className="border-yellow-500/50 bg-yellow-500/10">
            <CardContent className="flex items-center justify-between p-4">
              <div className="flex items-center gap-3">
                <Calendar className="h-5 w-5 text-yellow-600" />
                <div>
                  <p className="font-medium text-yellow-600">Trial Period</p>
                  <p className="text-sm text-yellow-600/80">
                    Your trial ends on {formatDate(trialEndsAt)}. Upgrade to continue.
                  </p>
                </div>
              </div>
              <Button size="sm" onClick={() => handleUpgrade("growth")}>
                Upgrade Now
              </Button>
            </CardContent>
          </Card>
        </BlurFade>
      )}

      {/* Current Plan Card */}
      <BlurFade delay={0.2} inView>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between">
            <div>
              <CardTitle>Current Plan</CardTitle>
              <CardDescription>Your active subscription details</CardDescription>
            </div>
            <Badge
              variant={currentPlan === "free" ? "secondary" : "default"}
              className="text-sm capitalize"
            >
              {currentPlan === "free" ? "Developer" : currentPlan}
            </Badge>
          </CardHeader>
          <CardContent>
            <div className="grid gap-6 md:grid-cols-3">
              {/* Plan details */}
              <div className="space-y-4">
                <div>
                  <p className="text-sm text-muted-foreground">Monthly Price</p>
                  <p className="text-2xl font-bold">
                    {currentPlan === "free" ? "$0" : currentPlan === "growth" ? "$99" : "Custom"}
                  </p>
                </div>
                {subscriptionEndsAt && (
                  <div>
                    <p className="text-sm text-muted-foreground">Billing Period Ends</p>
                    <p className="font-medium">{formatDate(subscriptionEndsAt)}</p>
                  </div>
                )}
              </div>

              {/* Plan features */}
              <div className="col-span-2 space-y-2">
                <p className="text-sm font-medium">Plan Features</p>
                <div className="grid grid-cols-2 gap-2">
                  {[
                    `${(eventsQuota / 1000).toFixed(0)}K events/month`,
                    currentPlan === "free" ? "1 Stream" : "Unlimited Streams",
                    currentPlan === "free" ? "Community Support" : "Priority Support",
                    currentPlan === "free" ? "7-day retention" : "90-day retention",
                    "Basic Analytics",
                    currentPlan !== "free" && "MCP Server Access",
                  ]
                    .filter(Boolean)
                    .map((feature) => (
                      <div key={feature as string} className="flex items-center gap-2 text-sm">
                        <Check className="h-4 w-4 text-primary" />
                        <span>{feature}</span>
                      </div>
                    ))}
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </BlurFade>

      {/* Usage Charts */}
      <BlurFade delay={0.3} inView>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold">Usage This Month</h2>
            <span className="text-sm text-muted-foreground">Resets on the 1st of each month</span>
          </div>
          <div className="grid gap-6 md:grid-cols-2">
            <UsageChart title="Events" used={eventsUsed} quota={eventsQuota} />
            <UsageChart title="Queries" used={queriesUsed} quota={queriesQuota} />
          </div>
        </div>
      </BlurFade>

      {/* Upgrade Section */}
      <BlurFade delay={0.4} inView>
        <div className="space-y-4">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <h2 className="text-lg font-semibold">Available Plans</h2>
              <p className="text-sm text-muted-foreground">Choose the plan that fits your needs</p>
            </div>

            {/* Billing toggle */}
            <div className="flex items-center gap-2 rounded-lg border border-border p-1">
              <button
                onClick={() => setIsYearly(false)}
                className={cn(
                  "rounded-md px-3 py-1.5 text-sm font-medium transition-all",
                  !isYearly ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                )}
              >
                Monthly
              </button>
              <button
                onClick={() => setIsYearly(true)}
                className={cn(
                  "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-all",
                  isYearly ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                )}
              >
                Yearly
                <Badge variant="secondary" className="text-[10px]">
                  Save 20%
                </Badge>
              </button>
            </div>
          </div>

          <PlanCards currentPlan={currentPlan} onUpgrade={handleUpgrade} />
        </div>
      </BlurFade>

      {/* FAQ */}
      <BlurFade delay={0.5} inView>
        <Card>
          <CardHeader>
            <CardTitle>Billing FAQ</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <h3 className="font-medium">What happens if I exceed my quota?</h3>
              <p className="text-sm text-muted-foreground">
                You can enable overage billing to continue using the service beyond your quota.
                Overages are charged at a per-unit rate based on your plan.
              </p>
            </div>
            <div>
              <h3 className="font-medium">Can I downgrade my plan?</h3>
              <p className="text-sm text-muted-foreground">
                Yes, you can downgrade at any time. The change will take effect at the end of your
                current billing period.
              </p>
            </div>
            <div>
              <h3 className="font-medium">Do you offer refunds?</h3>
              <p className="text-sm text-muted-foreground">
                We offer a 14-day money-back guarantee for all paid plans. Contact support if you'd
                like a refund.
              </p>
            </div>
          </CardContent>
        </Card>
      </BlurFade>
    </div>
  );
}
