import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";
import Link from "next/link";

export const metadata = constructMetadata({
  title: "Prime — Agent Memory Engine",
  description:
    "AllSource Prime: knowledge graph + vector search + compressed index in one binary. Documentation for AI agent memory.",
  canonical: "/docs/prime",
});

const sections = [
  {
    title: "Quickstart",
    href: "/docs/prime/quickstart",
    description:
      "Install, configure Claude Desktop, and store your first memory in 2 minutes.",
  },
  {
    title: "Concepts",
    href: "/docs/prime/concepts",
    description:
      "Compressed index, domains, cross-domain reasoning, temporal queries, and event provenance.",
  },
  {
    title: "MCP Setup",
    href: "/docs/prime/mcp",
    description:
      "Configure Prime as an MCP server for Claude Desktop with auto-inject and per-project memory.",
  },
  {
    title: "HTTP API",
    href: "/docs/prime/http",
    description:
      "REST endpoints for nodes, edges, vectors, recall, and the compressed index.",
  },
  {
    title: "Embedded (Rust Library)",
    href: "/docs/prime/embedded",
    description:
      "Use allsource-core as a Rust dependency with the prime feature for in-process agent memory.",
  },
];

export default function PrimeDocsPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        Prime — Agent Memory Engine
      </h1>
      <p className="text-lg text-muted-foreground mb-4">
        Knowledge graph + vector search + compressed index in one binary. Give
        your AI agent persistent, cross-domain memory with temporal reasoning.
      </p>
      <div className="mb-10 rounded-lg border bg-muted/30 px-4 py-3 font-mono text-sm">
        cargo install allsource-prime
      </div>

      <div className="grid gap-4">
        {sections.map((section) => (
          <Link
            key={section.title}
            href={section.href}
            className="group rounded-xl border border-border p-5 transition-all hover:border-primary/50 hover:bg-muted/50"
          >
            <h2 className="text-lg font-semibold text-foreground group-hover:text-primary transition-colors">
              {section.title}
            </h2>
            <p className="text-sm text-muted-foreground mt-1">
              {section.description}
            </p>
          </Link>
        ))}
      </div>
    </div>
  );
}
