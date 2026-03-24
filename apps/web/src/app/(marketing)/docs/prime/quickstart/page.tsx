import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Prime Quickstart",
  description: "Install AllSource Prime and store your first agent memory in 2 minutes.",
  canonical: "/docs/prime/quickstart",
});

export default function QuickstartPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        Quickstart
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        From zero to persistent agent memory in 2 minutes.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-8">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">1. Install</h2>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            cargo install allsource-prime
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Requires Rust 1.92+. Single static binary, no runtime dependencies.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            2. Configure Claude Desktop
          </h2>
          <p className="text-muted-foreground mb-3">
            Add to <code>~/.claude/claude_desktop_config.json</code>:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
{`{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/memory"]
    }
  }
}`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Restart Claude Desktop. You now have 13 tools for knowledge management.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            3. Store your first memory
          </h2>
          <p className="text-muted-foreground mb-3">
            Tell Claude something worth remembering:
          </p>
          <div className="rounded-lg border bg-muted/30 p-4 text-sm space-y-2">
            <p><strong>You:</strong> &quot;Alice is the lead engineer on the Prime project. She started in January 2026.&quot;</p>
            <p className="text-muted-foreground">
              Claude will call <code>prime_add_node</code> to create a &quot;person&quot; node with Alice&apos;s details,
              then <code>prime_add_edge</code> to connect her to the project.
            </p>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            4. Recall it later
          </h2>
          <div className="rounded-lg border bg-muted/30 p-4 text-sm space-y-2">
            <p><strong>You:</strong> &quot;Who works on the Prime project?&quot;</p>
            <p className="text-muted-foreground">
              Claude calls <code>prime_index</code> to see the knowledge map, then
              <code> prime_neighbors</code> to find everyone connected to the project node.
            </p>
            <p><strong>Claude:</strong> &quot;Alice is the lead engineer on Prime. She started in January 2026.&quot;</p>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            5. Check the compressed index
          </h2>
          <div className="rounded-lg border bg-muted/30 p-4 text-sm space-y-2">
            <p><strong>You:</strong> &quot;What do you know so far?&quot;</p>
            <p className="text-muted-foreground">
              Claude calls <code>prime_index</code> and returns:
            </p>
            <pre className="mt-2 rounded bg-black/60 p-3 text-xs text-green-400">
{`# Knowledge Index
_2 nodes, 1 domain_

## engineering
- **Nodes:** 2 (person, project)
- **Examples:** Alice, Prime`}
            </pre>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            What&apos;s next?
          </h2>
          <ul className="space-y-2 text-sm text-muted-foreground">
            <li>
              <a href="/docs/prime/concepts" className="text-foreground underline">Concepts</a>
              {" — "}understand domains, cross-references, and temporal queries
            </li>
            <li>
              <a href="/docs/prime/mcp" className="text-foreground underline">MCP Setup</a>
              {" — "}auto-inject, per-project memory, advanced configuration
            </li>
            <li>
              <a href="/docs/prime/http" className="text-foreground underline">HTTP API</a>
              {" — "}use Prime from any language via REST
            </li>
          </ul>
        </section>
      </div>
    </div>
  );
}
