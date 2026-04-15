export const BLUR_FADE_DELAY = 0.15;

export const siteConfig = {
  name: "AllSource",
  description: "AI-native event store for temporal data intelligence",
  url: process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000",
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
          href: "#",
        },
        items: [
          {
            href: "#",
            title: "Event Sourcing",
            description: "Immutable event logs with time-travel queries.",
          },
          {
            href: "#",
            title: "Stream Processing",
            description: "Real-time pipelines with filter, map, and reduce.",
          },
          {
            href: "/docs/mcp",
            title: "AI-Native Tools",
            description: "43 MCP tools for Claude Desktop integration.",
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
            href: "#",
            description: "Complete audit trails with immutable event history.",
          },
          {
            title: "Real-time Analytics",
            href: "#",
            description: "Sub-microsecond queries for instant insights.",
          },
          {
            title: "AI Agents",
            href: "#",
            description: "MCP server integration for autonomous workflows.",
          },
          {
            title: "Financial Services",
            href: "#",
            description: "Transaction logs with temporal consistency.",
          },
          {
            title: "IoT & Telemetry",
            href: "#",
            description: "High-throughput ingestion for sensor data.",
          },
          {
            title: "Multi-tenant SaaS",
            href: "#",
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
      href: "/blog",
      label: "Blog",
    },
  ],
  pricing: [
    {
      name: "DEVELOPER",
      tier: "free" as const,
      href: "#",
      price: "$0",
      period: "month",
      yearlyPrice: "$0",
      features: [
        "100K events/month",
        "1 Stream",
        "Community Support",
        "7-day retention",
        "Basic Analytics",
      ],
      description: "Perfect for learning and prototyping",
      buttonText: "Start Free",
      isPopular: false,
    },
    {
      name: "PRO",
      tier: "pro" as const,
      href: "#",
      price: "$29",
      period: "month",
      yearlyPrice: "$24",
      features: [
        "x402 Agent Endpoints",
        "1M events/month",
        "5 Streams",
        "30-day retention",
        "MCP Server (read-only)",
        "Email Support (48h)",
      ],
      description: "For solo operators running one production system",
      buttonText: "Start Pro",
      isPopular: false,
    },
    {
      name: "GROWTH",
      tier: "growth" as const,
      href: "#",
      price: "$79",
      period: "month, billed yearly",
      yearlyPrice: "$79",
      features: [
        "10M events/month",
        "Unlimited Streams",
        "Priority Support",
        "90-day retention",
        "Advanced Analytics",
        "MCP Server Access",
      ],
      description: "For teams building production systems",
      buttonText: "Start Trial",
      isPopular: true,
    },
    {
      name: "ENTERPRISE",
      tier: "enterprise" as const,
      href: "#",
      price: "Custom",
      period: "month",
      yearlyPrice: "Custom",
      features: [
        "Unlimited events",
        "Dedicated infrastructure",
        "24/7 Premium Support",
        "Unlimited retention",
        "Custom Integrations",
        "SLA & Compliance",
      ],
      description: "For high-volume, mission-critical deployments",
      buttonText: "Contact Sales",
      isPopular: false,
    },
  ],
  faqs: [
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
        { href: "#", text: "Event Store", icon: null },
        { href: "#", text: "Stream Processing", icon: null },
        { href: "/docs/mcp", text: "MCP Server", icon: null },
        { href: "/docs", text: "Documentation", icon: null },
      ],
    },
    {
      title: "Developers",
      links: [
        { href: "/docs", text: "Getting Started", icon: null },
        { href: "/docs/api", text: "API Reference", icon: null },
        { href: "#", text: "SDKs", icon: null },
        { href: "#", text: "Examples", icon: null },
        { href: "/changelog", text: "Changelog", icon: null },
      ],
    },
    {
      title: "Company",
      links: [
        { href: "#", text: "About", icon: null },
        { href: "/blog", text: "Blog", icon: null },
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
