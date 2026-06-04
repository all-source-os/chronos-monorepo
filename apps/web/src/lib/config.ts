export const BLUR_FADE_DELAY = 0.15;

export const siteConfig = {
  name: "AllSource",
  description: "AI-native event store for temporal data intelligence",
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
    { display: "43", numeric: 43, suffix: "", label: "MCP tools" },
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
  twitterHandle: "allsourcedev",
  links: {
    email: "hello@all-source.xyz",
    twitter: "https://twitter.com/allsourcedev",
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
            title: "AI-Native Tools",
            description: "43 MCP tools for Claude Desktop integration.",
          },
          {
            href: "/prime",
            title: "Prime — Memory for Claude",
            description:
              "Persistent agent memory via MCP. Install in 30 seconds, no embedding API needed.",
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
  //   tier         — stable PUBLIC id (self-host | indie | studio | scale | enterprise).
  //                  Prompt 011 maps this to a Stripe price id. DO NOT rename casually.
  //   billingTier  — legacy backend `subscription_tier` value this tier corresponds to
  //                  (free | starter | growth | enterprise) so the dashboard "current plan"
  //                  match keeps working until 011 reconciles the backend tiers. `null`
  //                  means there is no checkout for this tier (Self-Host) or no backend
  //                  tier exists yet (Scale — 011 owns adding it).
  //   mcp          — explicit MCP verbs so a buyer can price the upgrade at a glance.
  //   x402         — per-tier micropayment allowance, rendered as a single line.
  //                  `null` for tiers without metered x402 (Self-Host runs its own;
  //                  Enterprise is negotiated).
  pricing: [
    {
      name: "Self-Host",
      tier: "self-host" as const,
      billingTier: "free" as const,
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
        "MIT licensed",
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
      billingTier: "starter" as const,
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
      billingTier: "growth" as const,
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
      // No legacy backend tier yet — 011 owns adding `scale` to subscription_tier.
      billingTier: null,
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
      billingTier: "enterprise" as const,
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
  faqs: [
    {
      question: "Why no free plan?",
      answer:
        "We're MIT-licensed. Free already exists — run it yourself. Hosted pricing reflects what it costs us to run it for you.",
    },
    {
      question: "What is AllSource?",
      answer:
        "AllSource is a high-performance, AI-native event store designed for temporal data intelligence. It provides event sourcing capabilities with 469K events/sec ingestion and sub-microsecond query latency, making it ideal for audit trails, real-time analytics, and AI agent workflows.",
    },
    {
      question: "How does AllSource compare to traditional databases?",
      answer:
        "Unlike traditional databases that store current state, AllSource stores immutable events over time. This enables time-travel queries, complete audit trails, and the ability to replay history. Combined with our distributed architecture (Rust core, Go control plane, Elixir query service), you get both performance and flexibility.",
    },
    {
      question: "What is the MCP Server integration?",
      answer:
        "AllSource includes a 43-tool MCP (Model Context Protocol) Server that integrates directly with Claude Desktop. This allows AI agents to manage events, run analytics, detect anomalies, manage backups, monitor health, and perform complex operations autonomously - making it truly AI-native from the ground up.",
    },
    {
      question: "How secure is my data?",
      answer:
        "AllSource is enterprise-ready with multi-tenancy, RBAC (4 roles, 7 permissions), JWT authentication, policy enforcement, and comprehensive audit logging. All operations include dry-run preview capabilities and full audit trails.",
    },
    {
      question: "Can I self-host AllSource?",
      answer:
        "Yes! AllSource is open-source (MIT licensed) with minimal footprint (~129 MB for all services). We provide Docker images, Helm charts, and Kubernetes manifests for easy deployment. The cloud offering handles infrastructure management for teams who prefer a managed solution.",
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
        { href: "/docs/mcp", text: "MCP Server", icon: null },
        { href: "/docs", text: "Documentation", icon: null },
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
          href: "https://twitter.com/allsourcedev",
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
