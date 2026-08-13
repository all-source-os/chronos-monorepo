import { Analytics } from "@vercel/analytics/next";
import type { Metadata, Viewport } from "next";
import { EarlyAccessBanner } from "@/components/early-access-banner";
import { GeoReferralTracker } from "@/components/geo-referral-tracker";
import { TailwindIndicator } from "@/components/tailwind-indicator";
import { ThemeProvider } from "@/components/theme-provider";
import { ThemeToggle } from "@/components/theme-toggle";
import {
  organizationSchema,
  softwareApplicationSchema,
  websiteSchema,
} from "@/lib/structured-data";
import { cn, constructMetadata } from "@/lib/utils";
import "./globals.css";

/*
 * ADR — analytics choice for GEO layer 1 (prompt 024)
 *
 * The site had NO analytics of any kind before this. The constraint set was:
 * Vercel-hosted Next.js App Router; must expose the raw referrer and user
 * agent; must not force a cookie-banner rewrite.
 *
 * Chosen: **Vercel Web Analytics** (`@vercel/analytics`) for site-wide traffic,
 * plus a first-party `<GeoReferralTracker />` beacon for the GEO event stream.
 *
 * Why the pair rather than one product:
 * - Vercel Web Analytics is cookieless and privacy-preserving, so it adds no
 *   consent-banner obligation the site did not already have. It is served from
 *   `/_vercel/insights/*` on our own origin, so the existing strict CSP
 *   (`script-src 'self'`, `connect-src 'self'`) needs no widening — a
 *   third-party script host would have meant editing the CSP in
 *   `next.config.mjs` for every vendor domain. It needs no env var and no
 *   account setup beyond enabling it in the Vercel dashboard.
 * - It does NOT satisfy the second constraint. Vercel shows referrers in *its*
 *   dashboard; it gives us no programmatic access to the raw referrer and user
 *   agent, and it cannot join an arrival to a conversion inside AllSource.
 *   That is precisely what layer 1 is for, hence the beacon.
 *
 * Rejected:
 * - **PostHog** — the richest option, but it sets cookies by default and would
 *   pull the site into consent-banner work that is not this slice's job.
 * - **Plausible / Fathom** — cookieless and good, but a paid third-party
 *   script host (CSP widening) for aggregate numbers Vercel already gives us
 *   free, and still no raw referrer in our own pipeline.
 * - **GA4** — consent banner, cookie policy, and its channel grouping is the
 *   very thing the GEO framework says misattributes AI referrals as Direct.
 *
 * Env vars to set by hand: Vercel Web Analytics needs NONE (enable it in
 * Project Settings -> Analytics). The referral route needs `ALLSOURCE_API_KEY`
 * (and optionally `ALLSOURCE_API_URL`) set in the Vercel dashboard — server
 * scope only, never `NEXT_PUBLIC_`. See docs/runbooks/GEO_MEASUREMENT.md.
 */

export const metadata: Metadata = constructMetadata({
  title: "AllSource — AI-Native Event Store",
  description:
    "High-performance event sourcing with AI agent memory. 469K events/sec, 12μs queries. Time-travel, knowledge graphs, compressed index. Open source.",
  canonical: "/",
});

export const viewport: Viewport = {
  colorScheme: "dark",
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "black" },
    { media: "(prefers-color-scheme: light)", color: "white" },
  ],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
          dangerouslySetInnerHTML={{ __html: JSON.stringify(organizationSchema()) }}
        />
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
          dangerouslySetInnerHTML={{ __html: JSON.stringify(websiteSchema()) }}
        />
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
          dangerouslySetInnerHTML={{ __html: JSON.stringify(softwareApplicationSchema()) }}
        />
      </head>
      <body className={cn("min-h-screen bg-background antialiased w-full mx-auto scroll-smooth")}>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
          <EarlyAccessBanner />
          {children}
          <ThemeToggle />
          <TailwindIndicator />
          <GeoReferralTracker />
          <Analytics />
        </ThemeProvider>
      </body>
    </html>
  );
}
