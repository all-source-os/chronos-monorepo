export const BLUR_FADE_DELAY = 0.15;

export const siteConfig = {
  name: "all.source",
  description: "Time-travel your data. Let AI agents manage it.",
  url: process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000",
  keywords: [
    "Event Sourcing",
    "Event Store",
    "CQRS",
    "Time Travel",
    "AI-native",
    "MCP Server",
    "Rust",
    "High Performance",
  ],
  links: {
    email: "hello@all.source",
    twitter: "https://x.com/allsource",
    github: "https://github.com/all-source-os/all-source",
    discord: "https://discord.gg/allsource",
  },
  header: [
    {
      trigger: "Features",
      content: {
        main: {
          icon: "logo" as const,
          title: "Event Store for the AI Era",
          description: "469K events/sec throughput with 11.9μs p99 latency.",
          href: "#features",
        },
        items: [
          {
            href: "#features",
            title: "Time-Travel Queries",
            description: "Reconstruct any entity state at any point in time.",
          },
          {
            href: "#features",
            title: "Real-time Streaming",
            description: "WebSocket subscriptions for live event feeds.",
          },
          {
            href: "#features",
            title: "Schema Registry",
            description: "Versioned schemas with evolution and compatibility.",
          },
        ],
      },
    },
    {
      trigger: "Solutions",
      content: {
        items: [
          {
            title: "AI Agents",
            href: "#solutions",
            description: "27 MCP tools for Claude, GPT, and custom agents.",
          },
          {
            title: "Event Sourcing",
            href: "#solutions",
            description: "Complete event sourcing with projections and replay.",
          },
          {
            title: "Audit & Compliance",
            href: "#solutions",
            description: "Immutable audit logs with full data lineage.",
          },
          {
            title: "Real-time Analytics",
            href: "#solutions",
            description: "Stream processing pipelines and analytics engine.",
          },
          {
            title: "Multi-tenant SaaS",
            href: "#solutions",
            description: "Built-in tenant isolation with RBAC and quotas.",
          },
          {
            title: "Self-hosted",
            href: "https://github.com/all-source-os/all-source",
            description: "MIT licensed. Deploy anywhere. ~129MB footprint.",
          },
        ],
      },
    },
    {
      href: "/blog",
      label: "Blog",
    },
  ],
  pricing: [
    {
      name: "FREE",
      href: "/signup",
      price: "$0",
      period: "month",
      yearlyPrice: "$0",
      features: [
        "10K events/month",
        "1 GB storage",
        "1K queries/hour",
        "2 API keys",
        "5 projections",
        "Community support",
      ],
      description: "Perfect for side projects and learning",
      buttonText: "Get API Key",
      isPopular: false,
    },
    {
      name: "PRO",
      href: "/signup",
      price: "$29",
      period: "month",
      yearlyPrice: "$23",
      features: [
        "500K events/month",
        "10 GB storage",
        "50K queries/hour",
        "10 API keys",
        "25 projections",
        "MCP server access",
        "Priority support",
      ],
      description: "For developers shipping production apps",
      buttonText: "Get API Key",
      isPopular: true,
    },
    {
      name: "TEAM",
      href: "/signup",
      price: "$99",
      period: "month",
      yearlyPrice: "$79",
      features: [
        "5M events/month",
        "100 GB storage",
        "Unlimited queries",
        "Unlimited API keys",
        "Unlimited projections",
        "MCP server access",
        "Dedicated support",
      ],
      description: "For growing teams building at scale",
      buttonText: "Get API Key",
      isPopular: false,
    },
  ],
  faqs: [
    {
      question: "What is all.source?",
      answer:
        "all.source is a high-performance event store built in Rust, designed for event sourcing, CQRS, and AI-native workflows. It achieves 469K events/sec throughput with 11.9μs p99 query latency, and includes 27 MCP tools for AI agent integration.",
    },
    {
      question: "How does time-travel querying work?",
      answer:
        "all.source stores events immutably with timestamps. You can reconstruct any entity's state at any point in time using the `as_of` parameter. This enables debugging, auditing, and replaying historical states without any additional infrastructure.",
    },
    {
      question: "Can I use this with AI agents like Claude?",
      answer:
        "Yes! all.source includes a built-in MCP (Model Context Protocol) server with 27 tools. AI agents can query events, create projections, manage schemas, and perform analytics autonomously. It's designed for AI-native workflows.",
    },
    {
      question: "What's the difference between self-hosted and cloud?",
      answer:
        "Self-hosted is MIT licensed - you deploy it yourself with a ~129MB binary. Cloud is our managed service with automatic backups, scaling, and support. Both have the same features; cloud removes operational overhead.",
    },
    {
      question: "How does multi-tenancy work?",
      answer:
        "Every event, projection, and schema is scoped to a tenant. Queries are automatically filtered by tenant ID. Each tenant has configurable quotas for events, storage, and API calls. Enterprise tiers get dedicated infrastructure.",
    },
    {
      question: "Is there vendor lock-in?",
      answer:
        "No. all.source is MIT licensed and stores data in standard Parquet files. You can export your data anytime, self-host the entire system, or migrate to another solution. We believe in data freedom.",
    },
  ],
  footer: [
    {
      title: "Product",
      links: [
        { href: "#features", text: "Features", icon: null },
        { href: "#pricing", text: "Pricing", icon: null },
        { href: "https://docs.all.source", text: "Documentation", icon: null },
        { href: "https://docs.all.source/api", text: "API Reference", icon: null },
      ],
    },
    {
      title: "Company",
      links: [
        { href: "/about", text: "About", icon: null },
        { href: "/blog", text: "Blog", icon: null },
        { href: "https://github.com/all-source-os/all-source", text: "Open Source", icon: null },
        { href: "https://maps.app.goo.gl/CpwBqWZKx7aaLX5t5", text: "Location", icon: null },
      ],
    },
    {
      title: "Resources",
      links: [
        {
          href: "https://github.com/all-source-os/all-source/discussions",
          text: "Community",
          icon: null,
        },
        { href: "/contact", text: "Contact", icon: null },
        { href: "https://status.all.source", text: "Status", icon: null },
        {
          href: "https://github.com/all-source-os/all-source/releases",
          text: "Changelog",
          icon: null,
        },
      ],
    },
    {
      title: "Connect",
      links: [
        {
          href: "https://x.com/allsource",
          text: "X (Twitter)",
          icon: "twitter" as const,
        },
        {
          href: "https://github.com/all-source-os/all-source",
          text: "GitHub",
          icon: "github" as const,
        },
        {
          href: "https://discord.gg/allsource",
          text: "Discord",
          icon: "discord" as const,
        },
      ],
    },
  ],
};

export type SiteConfig = typeof siteConfig;
