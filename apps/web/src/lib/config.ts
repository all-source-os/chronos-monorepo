export const BLUR_FADE_DELAY = 0.15;

export const siteConfig = {
  name: "AllSource",
  description: "Open-source event store for durable system history and AI agent memory",
  // Single source of truth for the headline performance numbers. Both the
  // homepage demo chrome and the below-the-fold stat strip read from here so
  // the values can never desync — and so they can be rendered at their FINAL
  // value on first paint (no animate-from-zero "0K"/"0μs" flash).
  //   display — the exact string to paint (already formatted).
  //   numeric/suffix — used only by the optional count-up animation, which
  //                    must START from `display`, never from 0.
  stats: [
    { display: "469K", numeric: 469, suffix: "K", label: "events/sec" },
    { display: "11.9μs", numeric: 11.9, suffix: "μs", label: "p99 recall" },
    { display: "55+", numeric: 55, suffix: "+", label: "MCP tools" },
    { display: "129MB", numeric: 129, suffix: "MB", label: "footprint" },
  ],
  // The single µs figure the homepage demo stamps on the recalled answer.
  recallLatency: "11.2μs",
  // Falls back to the production URL — not localhost — so a missing
  // NEXT_PUBLIC_APP_URL on Vercel (or any other consumer of this config) can't
  // leak `http://localhost:3000` into og:url / canonical / share-sheet URLs.
  // Override in `.env.local` if you want localhost in dev OG tags.
  url: process.env.NEXT_PUBLIC_APP_URL || "https://www.all-source.xyz",
  keywords: [
    "Event Sourcing",
    "Event Store",
    "Temporal Data",
    "AI-Native",
    "Stream Processing",
    "Real-time Analytics",
    "CQRS",
    "Data Intelligence",
  ],
  // Bare X/Twitter handle (no @) — single source for cards, JSON-LD, and the
  // blog author chip. `links.twitter` is the profile URL built from it.
  twitterHandle: "ddonprogramming",
  links: {
    email: "hello@all-source.xyz",
    twitter: "https://x.com/ddonprogramming",
    discord: "https://github.com/all-source-os/all-source/discussions",
    github: "https://github.com/all-source-os/all-source",
    instagram: "https://instagram.com/allsourcedev",
  },
  header: [
    {
      trigger: "Platform",
      content: {
        main: {
          icon: "logo" as const,
          title: "Event Store Engine",
          description: "High-performance event sourcing with 469K events/sec throughput.",
          href: "/platform/event-sourcing",
        },
        items: [
          {
            href: "/platform/event-sourcing",
            title: "Event Sourcing",
            description: "Immutable event logs with time-travel queries.",
          },
          {
            href: "/platform/stream-processing",
            title: "Stream Processing",
            description: "Real-time pipelines with filter, map, and reduce.",
          },
          {
            href: "/docs/mcp",
            title: "MCP Tools for Agents",
            description: "55+ MCP tools for Claude Desktop integration (73 for fleet operators).",
          },
          {
            href: "/prime",
            title: "Prime — Memory for Claude",
            description: "Persistent agent memory over MCP with in-process embeddings.",
          },
          {
            href: "/solutions/quant-intelligence",
            title: "Quant Intelligence",
            description: "Probability-based market insights and AI queries.",
          },
        ],
      },
    },
    {
      trigger: "Solutions",
      content: {
        items: [
          {
            title: "Audit & Compliance",
            href: "/solutions/audit-compliance",
            description: "Complete audit trails with immutable event history.",
          },
          {
            title: "Real-time Analytics",
            href: "/solutions/real-time-analytics",
            description: "Sub-microsecond queries for instant insights.",
          },
          {
            title: "AI Agents",
            href: "/solutions/agent-memory",
            description: "MCP server integration for autonomous workflows.",
          },
          {
            title: "Financial Services",
            href: "/solutions/financial-services",
            description: "Transaction logs with temporal consistency.",
          },
          {
            title: "IoT & Telemetry",
            href: "/solutions/iot-telemetry",
            description: "High-throughput ingestion for sensor data.",
          },
          {
            title: "Multi-tenant SaaS",
            href: "/solutions/multi-tenant-saas",
            description: "Secure isolation with RBAC and policy enforcement.",
          },
          {
            title: "Quant Intelligence",
            href: "/solutions/quant-intelligence",
            description: "Probability-based analytics for trading strategies.",
          },
        ],
      },
    },
    {
      href: "/install",
      label: "Install",
    },
    {
      href: "/pricing",
      label: "Pricing",
    },
    {
      href: "/ecosystem",
      label: "Ecosystem",
    },
    {
      href: "/architecture",
      label: "Architecture",
    },
    {
      href: "/blog",
      label: "Blog",
    },
  ],
  // Single source of truth for pricing tiers. The /pricing page and the
  // dashboard billing page both map over this array.
  //
  // Field contract for downstream prompts:
  //   tier         — canonical tier id (self-host | indie | studio | scale | enterprise).
  //                  This is the ONE naming scheme used end-to-end (matches the backend
  //                  subscription_tier after canonicalTier() normalization). DO NOT rename
  //                  casually; it also keys the LemonSqueezy catalog + checkout.
  //   mcp          — explicit MCP verbs so a buyer can price the upgrade at a glance.
  //   x402         — per-tier micropayment allowance, rendered as a single line.
  //                  `null` for tiers without metered x402 (Self-Host runs its own;
  //                  Enterprise is negotiated).
  pricing: [
    {
      // Self-Host is NOT advertised on /pricing — the product has no free plan
      // (14-day trial, then paid/enterprise), so the public pricing cards and
      // matrix filter this entry out (it reads as "Free"). It stays in this array
      // ONLY so the authenticated dashboard can still render a tenant that's on
      // the legacy `self-host` tier (the web analog of admin's planLabel) and so
      // getPlanConfig's `pricing[0]` fallback resolves to a safe entry. The
      // run-it-yourself Apache-2.0 story lives in the "Why no free plan?" FAQ below.
      name: "Self-Host",
      tier: "self-host" as const,
      href: "https://github.com/all-source-os/all-source",
      price: "Free",
      period: "your infra",
      yearlyPrice: "Free",
      mcp: "Full MCP (self-host)",
      x402: null,
      features: [
        "Unlimited events (your hardware)",
        "Forever retention",
        "Unlimited streams",
        "Full MCP (self-host)",
        "GitHub community support",
        "Apache-2.0 licensed",
      ],
      description: "Tinkerers, OSS, on-prem. Run it yourself.",
      buttonText: "Self-host on GitHub",
      isPopular: false,
      isSelfHost: true,
      isEnterprise: false,
    },
    {
      name: "Indie",
      tier: "indie" as const,
      href: "/signup",
      price: "$19",
      period: "month",
      yearlyPrice: "$15",
      mcp: "Hosted MCP: read",
      // x402 overage rate per PRICING_EXPOSURE_PLAN.md §3.5 ("$0.0001/call after").
      x402: { included: "50K x402 calls", overage: "$0.0001/call after" },
      features: [
        "500K events/month",
        "14-day retention",
        "3 streams",
        "Hosted MCP: read",
        "Email support (48h)",
      ],
      description: "Solo builders running one agent.",
      buttonText: "Start Indie",
      isPopular: false,
      isSelfHost: false,
      isEnterprise: false,
    },
    {
      name: "Studio",
      tier: "studio" as const,
      href: "/signup",
      price: "$79",
      period: "month",
      yearlyPrice: "$63",
      mcp: "Hosted MCP: read + write",
      x402: { included: "500K x402 calls", overage: "$0.0001/call after" },
      features: [
        "5M events/month",
        "90-day retention",
        "Unlimited streams",
        "Hosted MCP: read + write",
        "Email support (24h) + Discord",
      ],
      description: "Teams running 1–5 agents.",
      buttonText: "Start Studio",
      isPopular: true,
      isSelfHost: false,
      isEnterprise: false,
    },
    {
      name: "Scale",
      tier: "scale" as const,
      href: "/signup",
      price: "$299",
      period: "month",
      yearlyPrice: "$239",
      mcp: "Hosted MCP: read + write + dedicated",
      x402: { included: "5M x402 calls", overage: "$0.0001/call after" },
      features: [
        "50M events/month",
        "365-day retention",
        "Unlimited streams",
        "Hosted MCP: read + write + dedicated",
        "Priority support + Slack",
      ],
      description: "Companies with 50+ agents.",
      buttonText: "Start Scale",
      isPopular: false,
      isSelfHost: false,
      isEnterprise: false,
    },
    {
      name: "Enterprise",
      tier: "enterprise" as const,
      href: "mailto:sales@all-source.xyz?subject=Enterprise%20Plan%20Inquiry",
      price: "Custom",
      period: "month",
      yearlyPrice: "Custom",
      mcp: "Dedicated MCP cluster",
      x402: null,
      features: [
        "Negotiated event volume",
        "Unlimited retention",
        "Unlimited streams",
        "Dedicated MCP cluster",
        "24/7 + dedicated SE",
        "SLA & compliance",
      ],
      description: "Regulated / SLA workloads.",
      buttonText: "Talk to us",
      isPopular: false,
      isSelfHost: false,
      isEnterprise: true,
    },
  ],
  /**
   * Pricing-page FAQ set.
   *
   * GEO/AEO note: kept separate from `faqs` so /pricing and / do not emit the
   * SAME FAQPage graph on two URLs (duplicate schema splits the signal). Each
   * answer is written to be extractable on its own — it names the product,
   * carries the real number, and makes sense with no surrounding page context,
   * because an answer engine lifts the answer, not the page.
   *
   * Every figure here must match `siteConfig.pricing` and the live catalog.
   * Wrong prices in schema are worse than no schema: they get cited.
   */
  pricingFaqs: [
    {
      question: "How much does AllSource cost?",
      answer:
        "Hosted AllSource starts at $19/month for Indie (500K events/month, 14-day retention, 3 streams). Studio is $79/month (5M events, 90-day retention, unlimited streams) and Scale is $299/month (50M events, 365-day retention). Enterprise is negotiated. Annual billing drops each tier to $15, $63, and $239/month.",
    },
    {
      question: "Does AllSource have a free tier?",
      answer:
        "No free hosted tier. AllSource is open source under Apache-2.0, so self-hosting is free forever on your own hardware with unlimited events and retention. Hosted plans start with a 14-day trial (1,000 events) and then require a paid tier — hosted pricing reflects what it costs to run the infrastructure for you.",
    },
    {
      question: "What happens if I exceed my monthly event quota?",
      answer:
        "Each paid tier includes metered x402 micropayment credits: 50K calls on Indie, 500K on Studio, 5M on Scale. Beyond the included allowance, usage bills at $0.0001 per call. Events themselves are quota'd per tier, so sustained overage is a signal to move up rather than an open-ended bill.",
    },
    {
      question: "Can I self-host AllSource instead of paying?",
      answer:
        "Yes. The core event store is Apache-2.0 licensed and runs anywhere Docker does, with unlimited events, forever retention, unlimited streams, and full MCP access on your own infrastructure. Some enterprise-specific features are licensed under BSL 1.1. Self-hosting is supported through GitHub rather than email or Slack.",
    },
    {
      question: "Is AllSource priced per seat or per user?",
      answer:
        "Neither. AllSource prices on events written and retained, not on people. A team of one and a team of thirty pay the same for the same event volume, because the cost driver is agent throughput and retention window — not headcount. Streams and MCP access level vary by tier.",
    },
  ],
  faqs: [
    {
      question: "Why no free plan?",
      answer:
        "We're open source (Apache-2.0). Free already exists — run it yourself. Hosted pricing reflects what it costs us to run it for you.",
    },
    {
      question: "What is AllSource?",
      answer:
        "AllSource is an open-source event store. It appends immutable events, rebuilds state from those events, and exposes history to applications and AI agents through HTTP, SDKs, and MCP.",
    },
    {
      question: "How does AllSource compare to traditional databases?",
      answer:
        "Unlike traditional databases that store current state, AllSource stores immutable events over time. This enables time-travel queries, complete audit trails, and the ability to replay history. Combined with our distributed architecture (Rust core, Go control plane, Elixir query service), you get both performance and flexibility.",
    },
    {
      question: "What is the MCP Server integration?",
      answer:
        "AllSource includes an MCP (Model Context Protocol) server for Claude Desktop and other MCP clients. A tenant connector exposes 55+ event and memory tools. Fleet operators can enable 73 tools by adding control-plane and system-administration access.",
    },
    {
      question: "How secure is my data?",
      answer:
        "Hosted AllSource separates tenant data and supports JWT authentication, role-based access, policy enforcement, and audit logs. Administrative operations include dry-run previews where the API supports them.",
    },
    {
      question: "Can I self-host AllSource?",
      answer:
        "Yes. The core is Apache-2.0 licensed, while designated enterprise features use BSL 1.1. Docker images, Helm charts, and Kubernetes manifests are available in the repository; hosted plans operate the services for you.",
    },
    {
      question: "What is Quant Intelligence?",
      answer:
        "Quant Intelligence is our premium analytics layer that transforms raw market data into probability-based insights. Instead of just showing charts, it reveals how markets tend to behave under specific conditions. Features include precomputed NQ/BTC distributions, point-in-time reproducibility for backtesting, and an upcoming AI query interface for natural language questions like 'What's the probability of NQ making new highs after a gap up?'",
    },
  ],
  footer: [
    {
      title: "Platform",
      links: [
        { href: "/platform/event-sourcing", text: "Event Store", icon: null },
        { href: "/platform/stream-processing", text: "Stream Processing", icon: null },
        { href: "/platform/prime", text: "Prime (agent memory)", icon: null },
        { href: "/docs/mcp", text: "MCP Server", icon: null },
        { href: "/docs", text: "Documentation", icon: null },
      ],
    },
    {
      // GEO/AEO note: every /solutions/* page was an orphan — present in
      // sitemap.ts, cross-linked between three of the pages, and reachable from
      // no site-wide navigation at all. Sitemap presence alone is a weak
      // discovery signal, and crawlers that follow links (including AI
      // indexers) had no path to them. This column is that path.
      title: "Solutions",
      links: [
        { href: "/solutions/agent-memory", text: "Agent Memory", icon: null },
        { href: "/solutions/real-time-analytics", text: "Real-Time Analytics", icon: null },
        { href: "/solutions/audit-compliance", text: "Audit & Compliance", icon: null },
        { href: "/solutions/financial-services", text: "Financial Services", icon: null },
        { href: "/solutions/multi-tenant-saas", text: "Multi-Tenant SaaS", icon: null },
        { href: "/solutions/iot-telemetry", text: "IoT Telemetry", icon: null },
        { href: "/solutions/quant-intelligence", text: "Quant Intelligence", icon: null },
      ],
    },
    {
      title: "Developers",
      links: [
        { href: "/docs", text: "Getting Started", icon: null },
        { href: "/docs/api", text: "API Reference", icon: null },
        { href: "/sdks", text: "SDKs", icon: null },
        { href: "/examples", text: "Examples", icon: null },
        { href: "/changelog", text: "Changelog", icon: null },
      ],
    },
    {
      title: "Compare",
      links: [
        { href: "/event-sourcing-for-ai-agents", text: "Event Sourcing for AI Agents", icon: null },
        { href: "/vs/mem0", text: "vs mem0", icon: null },
        { href: "/vs/letta", text: "vs Letta", icon: null },
        { href: "/vs/zep", text: "vs Zep", icon: null },
        { href: "/vs/stoolap", text: "vs stoolap", icon: null },
      ],
    },
    {
      title: "Company",
      links: [
        { href: "/about", text: "About", icon: null },
        { href: "/blog", text: "Blog", icon: null },
        { href: "/status", text: "Status", icon: null },
        { href: "/privacy", text: "Privacy Policy", icon: null },
        { href: "/terms", text: "Terms of Service", icon: null },
      ],
    },
    {
      title: "Connect",
      links: [
        {
          href: "https://github.com/all-source-os/all-source",
          text: "GitHub",
          icon: "github" as const,
        },
        {
          href: "https://x.com/ddonprogramming",
          text: "Twitter",
          icon: "twitter" as const,
        },
        {
          href: "https://github.com/all-source-os/all-source/discussions",
          text: "Community",
          icon: "discord" as const,
        },
      ],
    },
  ],
};

export type SiteConfig = typeof siteConfig;

// The Indie tier's monthly price, sourced from the pricing array so the
// homepage CTA ("Start Indie — $19") can never desync from /pricing.
export const indieTier = siteConfig.pricing.find((p) => p.tier === "indie");
export const indiePrice = indieTier?.price ?? "$19";
