import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "MCP Integration",
  description: `Connect ${siteConfig.name} to Claude Desktop with the Model Context Protocol server.`,
  canonical: "/docs/mcp",
});

export default function McpPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        MCP Integration
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        AllSource includes a Model Context Protocol (MCP) server that integrates
        directly with Claude Desktop, giving AI agents full access to your event
        store.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-8">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            What is MCP?
          </h2>
          <p className="text-muted-foreground leading-relaxed">
            The{" "}
            <a
              href="https://modelcontextprotocol.io"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              Model Context Protocol
            </a>{" "}
            is an open standard that lets AI assistants interact with external
            tools and data sources. AllSource&apos;s MCP server exposes 43 tools
            for event management, analytics, schema operations, and system
            administration.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Quick Setup
          </h2>

          <h3 className="text-lg font-medium text-foreground mt-4 mb-2">
            1. Start the MCP server
          </h3>
          <p className="text-muted-foreground leading-relaxed mb-3">
            If you&apos;re running the AllSource Docker stack, the MCP server is
            available at port 3904 by default. Otherwise, you can run it
            standalone:
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`docker run -p 3904:3904 \\
  -e CORE_URL=http://host.docker.internal:3900 \\
  ghcr.io/all-source-os/chronos-mcp:latest`}</code>
          </pre>

          <h3 className="text-lg font-medium text-foreground mt-6 mb-2">
            2. Configure Claude Desktop
          </h3>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Add the following to your{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">
              claude_desktop_config.json
            </code>
            :
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`{
  "mcpServers": {
    "allsource": {
      "url": "http://localhost:3904/sse"
    }
  }
}`}</code>
          </pre>

          <h3 className="text-lg font-medium text-foreground mt-6 mb-2">
            3. Start using it
          </h3>
          <p className="text-muted-foreground leading-relaxed">
            Once connected, Claude can ingest events, run queries, manage schemas,
            create projections, monitor health, and perform administrative tasks —
            all through natural language.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Example Tools
          </h2>
          <div className="space-y-3">
            {[
              { name: "ingest_event", desc: "Store a new event in a stream" },
              { name: "query_events", desc: "Query events by stream, type, time range, or entity" },
              { name: "create_projection", desc: "Create a materialized view over event streams" },
              { name: "get_stream_stats", desc: "Get event counts and throughput for a stream" },
              { name: "register_schema", desc: "Register a JSON schema for event validation" },
              { name: "health_check", desc: "Check Core and system health status" },
            ].map((tool) => (
              <div
                key={tool.name}
                className="flex items-start gap-3 rounded-lg border border-border p-3"
              >
                <code className="text-sm font-mono text-primary shrink-0">
                  {tool.name}
                </code>
                <span className="text-sm text-muted-foreground">{tool.desc}</span>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Full Tool Catalog
          </h2>
          <p className="text-muted-foreground leading-relaxed">
            For the complete list of MCP tools with parameters and examples, see
            the{" "}
            <a
              href="https://github.com/all-source-os/all-source/tree/main/docs"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              docs directory on GitHub
            </a>
            .
          </p>
        </section>
      </div>
    </div>
  );
}
