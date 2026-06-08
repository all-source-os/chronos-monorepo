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
import { Calendar, Check, CreditCard, ExternalLink, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { PlanCards } from "@/components/billing/plan-cards";
import { UsageChart } from "@/components/billing/usage-chart";
import { useDashboardStats } from "@/hooks/use-dashboard-stats";
import { apiClient } from "@/lib/api/client";
import { siteConfig } from "@/lib/config";
import { type Catalog, indexByTier } from "@/lib/pricing-catalog";
import { useAuthStore } from "@/lib/stores/auth-store";

function getPlanConfig(tier: string) {
  // `tier` is the backend `subscription_tier`. It may be a canonical 011 tier
  // (self-host/indie/studio/scale/enterprise) or a legacy billingTier
  // (free/starter/growth/enterprise) — match either.
  return (
    siteConfig.pricing.find((p) => p.tier === tier || p.billingTier === tier) ??
    siteConfig.pricing[0]!
  );
}

export default function BillingPage() {
  const { tenant, user } = useAuthStore();
  const { stats } = useDashboardStats();
  const [isYearly, setIsYearly] = useState(false);
  const [isLoadingPortal, setIsLoadingPortal] = useState(false);
  // Live LemonSqueezy prices (source of truth) fetched via the catalog proxy.
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  useEffect(() => {
    fetch("/api/billing/catalog")
      .then((r) => (r.ok ? r.json() : null))
      .then(setCatalog)
      .catch(() => {});
  }, []);

  const currentTier = tenant?.subscription_tier || "free";
  const planConfig = getPlanConfig(currentTier);
  const eventsUsed = stats.events.used || tenant?.events_used || 0;
  const eventsQuota = stats.events.quota || tenant?.events_quota || 10000;
  const queriesUsed = stats.queries.used || tenant?.queries_used || 0;
  const queriesQuota = stats.queries.quota || tenant?.queries_quota || 10000;
  const trialEndsAt = tenant?.trial_ends_at;
  const subscriptionEndsAt = tenant?.subscription_ends_at;

  const currentCat = indexByTier(catalog)[planConfig.tier];
  const displayPrice =
    tenant?.billing_period === "annual"
      ? (currentCat?.annual?.per_month ?? planConfig.yearlyPrice)
      : (currentCat?.monthly?.formatted ?? planConfig.price);

  const handleManageSubscription = async () => {
    setIsLoadingPortal(true);
    try {
      const response = await apiClient.getBillingPortal(tenant?.id);
      if (response.data?.portal_url) {
        window.open(response.data.portal_url, "_blank");
      }
    } catch (error) {
      console.error("Failed to get billing portal:", error);
    } finally {
      setIsLoadingPortal(false);
    }
  };

  const handleUpgrade = async (
    planTier: string,
    billingPeriod: "monthly" | "annual" = "monthly"
  ) => {
    if (planTier === "enterprise") {
      window.open("mailto:sales@all-source.xyz?subject=Enterprise%20Plan%20Inquiry", "_blank");
      return;
    }
    try {
      const response = await apiClient.createCheckout(planTier, billingPeriod, {
        tenantId: tenant?.id,
        email: user?.email,
        redirectUrl: `${window.location.origin}/dashboard/billing`,
      });
      if (response.data?.checkout_url) {
        window.location.href = response.data.checkout_url;
      }
    } catch (error) {
      console.error("Failed to create checkout:", error);
    }
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
          {currentTier !== "free" && (
            <Button variant="outline" onClick={handleManageSubscription} disabled={isLoadingPortal}>
              {isLoadingPortal ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <CreditCard className="mr-2 h-4 w-4" />
              )}
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
            <Badge variant={currentTier === "free" ? "secondary" : "default"} className="text-sm">
              {planConfig.name}
            </Badge>
          </CardHeader>
          <CardContent>
            <div className="grid gap-6 md:grid-cols-3">
              {/* Plan details */}
              <div className="space-y-4">
                <div>
                  <p className="text-sm text-muted-foreground">
                    {tenant?.billing_period === "annual" ? "Annual Price" : "Monthly Price"}
                  </p>
                  <p className="text-2xl font-bold">
                    {displayPrice}
                    {displayPrice !== "Custom" && (
                      <span className="text-base font-normal text-muted-foreground">
                        /{planConfig.period}
                      </span>
                    )}
                  </p>
                  {tenant?.billing_period === "annual" && currentTier !== "free" && (
                    <p className="text-xs text-muted-foreground">billed annually</p>
                  )}
                </div>
                {tenant?.billing_period && currentTier !== "free" && (
                  <div>
                    <p className="text-sm text-muted-foreground">Billing Period</p>
                    <p className="font-medium capitalize">{tenant.billing_period}</p>
                  </div>
                )}
                {subscriptionEndsAt && (
                  <div>
                    <p className="text-sm text-muted-foreground">Billing Period Ends</p>
                    <p className="font-medium">{formatDate(subscriptionEndsAt)}</p>
                  </div>
                )}
                {currentTier === "free" && (
                  <Button size="sm" onClick={() => handleUpgrade("growth")}>
                    Upgrade
                  </Button>
                )}
              </div>

              {/* Plan features from config */}
              <div className="col-span-2 space-y-2">
                <p className="text-sm font-medium">Plan Features</p>
                <div className="grid grid-cols-2 gap-2">
                  {planConfig.features.map((feature) => (
                    <div key={feature} className="flex items-center gap-2 text-sm">
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
                type="button"
                onClick={() => setIsYearly(false)}
                className={cn(
                  "rounded-md px-3 py-1.5 text-sm font-medium transition-all",
                  !isYearly ? "bg-primary text-primary-foreground" : "hover:bg-muted"
                )}
              >
                Monthly
              </button>
              <button
                type="button"
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

          <PlanCards
            currentPlan={currentTier}
            isYearly={isYearly}
            catalog={catalog}
            onUpgrade={handleUpgrade}
          />
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
