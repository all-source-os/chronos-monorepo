import type React from "react";
import { colors, fonts } from "../components/styles";

interface BlogHeaderProps {
  title: string;
  icon: string;
  gradient: string;
  accentColor: string;
}

const BlogHeader: React.FC<BlogHeaderProps> = ({ title, icon, gradient, accentColor }) => (
  <div
    style={{
      width: "100%",
      height: "100%",
      background: `linear-gradient(135deg, ${colors.bg} 0%, #0d0d18 100%)`,
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      fontFamily: fonts.sans,
      padding: 80,
      position: "relative",
      overflow: "hidden",
    }}
  >
    {/* Background pattern */}
    <div style={{
      position: "absolute",
      inset: 0,
      backgroundImage: `radial-gradient(circle at 25% 25%, ${accentColor}15 0%, transparent 50%), radial-gradient(circle at 75% 75%, ${accentColor}10 0%, transparent 50%)`,
    }} />

    {/* Grid lines */}
    <div style={{
      position: "absolute",
      inset: 0,
      backgroundImage: `linear-gradient(${colors.border}30 1px, transparent 1px), linear-gradient(90deg, ${colors.border}30 1px, transparent 1px)`,
      backgroundSize: "60px 60px",
      opacity: 0.3,
    }} />

    {/* Icon */}
    <div style={{
      fontSize: 64,
      marginBottom: 32,
      filter: "drop-shadow(0 0 40px " + accentColor + "60)",
      position: "relative",
    }}>
      {icon}
    </div>

    {/* Title */}
    <h1 style={{
      fontSize: 48,
      fontWeight: 800,
      color: colors.text,
      textAlign: "center",
      lineHeight: 1.2,
      maxWidth: 900,
      margin: 0,
      position: "relative",
      letterSpacing: -1,
    }}>
      {title}
    </h1>

    {/* AllSource branding */}
    <div style={{
      position: "absolute",
      bottom: 40,
      display: "flex",
      alignItems: "center",
      gap: 10,
    }}>
      <div style={{
        width: 24,
        height: 24,
        borderRadius: 7,
        background: `linear-gradient(135deg, ${colors.primary}, ${colors.cyan})`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize: 12,
        fontWeight: 800,
        color: "white",
      }}>A</div>
      <span style={{ fontSize: 14, fontWeight: 600, color: colors.textDim }}>AllSource Blog</span>
    </div>
  </div>
);

// Each blog post gets a themed header
export const blogHeaders: Record<string, BlogHeaderProps> = {
  "introducing-allsource": {
    title: "Introducing AllSource",
    icon: "🚀",
    gradient: "from-indigo-500 to-purple-500",
    accentColor: "#6366f1",
  },
  "time-travel-queries": {
    title: "Time-Travel Queries",
    icon: "⏳",
    gradient: "from-blue-500 to-cyan-500",
    accentColor: "#3b82f6",
  },
  "ai-agents-need-memory": {
    title: "AI Agents Need Memory",
    icon: "🧠",
    gradient: "from-purple-500 to-pink-500",
    accentColor: "#a855f7",
  },
  "event-store-vs-database": {
    title: "Event Store vs Database",
    icon: "⚡",
    gradient: "from-orange-500 to-red-500",
    accentColor: "#f97316",
  },
  "mcp-tools-claude-integration": {
    title: "43 MCP Tools for Claude",
    icon: "🔧",
    gradient: "from-cyan-500 to-blue-500",
    accentColor: "#06b6d4",
  },
  "building-agent-memory-in-rust": {
    title: "Building Agent Memory in Rust",
    icon: "🦀",
    gradient: "from-orange-500 to-amber-500",
    accentColor: "#f97316",
  },
  "12-microsecond-agent-memory": {
    title: "12-Microsecond Agent Memory",
    icon: "⚡",
    gradient: "from-yellow-500 to-orange-500",
    accentColor: "#eab308",
  },
  "temporal-ai-future-of-rag": {
    title: "Temporal AI: The Future of RAG",
    icon: "🔮",
    gradient: "from-violet-500 to-purple-500",
    accentColor: "#8b5cf6",
  },
  "why-event-sourcing-2026": {
    title: "Why Event Sourcing in 2026",
    icon: "📐",
    gradient: "from-green-500 to-emerald-500",
    accentColor: "#22c55e",
  },
  "compressed-index-doubles-cross-domain-recall": {
    title: "Compressed Index Doubles Recall",
    icon: "📊",
    gradient: "from-teal-500 to-cyan-500",
    accentColor: "#14b8a6",
  },
  "from-zerodex-to-allsource": {
    title: "From Zer0dex to AllSource",
    icon: "🔄",
    gradient: "from-blue-500 to-indigo-500",
    accentColor: "#3b82f6",
  },
  "zer0dex-vs-allsource-recall": {
    title: "Zer0dex vs AllSource Recall",
    icon: "⚖️",
    gradient: "from-slate-500 to-zinc-500",
    accentColor: "#64748b",
  },
  "tiered-context-loading-for-agent-loops": {
    title: "Tiered Context Loading",
    icon: "🔄",
    gradient: "from-amber-500 to-orange-500",
    accentColor: "#f59e0b",
  },
  "connecting-without-an-sdk": {
    title: "Connecting Without an SDK",
    icon: "🔌",
    gradient: "from-green-500 to-teal-500",
    accentColor: "#22c55e",
  },
  "connection-path": {
    title: "The Connection Path",
    icon: "🛤️",
    gradient: "from-indigo-500 to-blue-500",
    accentColor: "#6366f1",
  },
  "audit-trails-soc2-event-sourcing": {
    title: "Audit Trails That Pass SOC2",
    icon: "🛡️",
    gradient: "from-blue-500 to-indigo-500",
    accentColor: "#3b82f6",
  },
  "event-sourcing-ai-agent-memory-guide": {
    title: "Event Sourcing for AI Agent Memory",
    icon: "🧠",
    gradient: "from-purple-500 to-violet-500",
    accentColor: "#a855f7",
  },
  "why-saas-needs-event-sourcing": {
    title: "Why Your SaaS Needs Event Sourcing",
    icon: "🏗️",
    gradient: "from-emerald-500 to-green-500",
    accentColor: "#10b981",
  },
  "how-allsource-core-works": {
    title: "How AllSource Core Works",
    icon: "🔧",
    gradient: "from-slate-500 to-zinc-500",
    accentColor: "#64748b",
  },
  "allsource-for-startups-dev-to-prod": {
    title: "From Dev to Prod in 15 Minutes",
    icon: "🚀",
    gradient: "from-pink-500 to-rose-500",
    accentColor: "#ec4899",
  },
  "real-time-dashboards-without-etl": {
    title: "Real-Time Dashboards Without ETL",
    icon: "📈",
    gradient: "from-cyan-500 to-blue-500",
    accentColor: "#06b6d4",
  },
};

// Factory: create a component for a specific blog slug
export const makeBlogHeader = (slug: string): React.FC => {
  const props = blogHeaders[slug];
  if (!props) return () => <BlogHeader title={slug} icon="📝" gradient="from-gray-500 to-gray-600" accentColor="#64748b" />;
  return () => <BlogHeader {...props} />;
};
