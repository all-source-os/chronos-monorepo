import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Prime MCP Setup",
  description:
    "Configure AllSource Prime as an MCP server for Claude Desktop with auto-inject, per-project memory, and all 19 tools.",
  canonical: "/docs/prime/mcp",
});

export default function McpPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        Connect Prime to an MCP client
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        Configure Prime as an MCP server for Claude Desktop with auto-inject, per-project memory,
        and advanced options.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-10">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Install</h2>
          <p className="text-muted-foreground mb-3">
            Get the binary from crates.io. Builds standalone — no AllSource server required.
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            {`cargo install allsource-prime
allsource-prime --version  # allsource-prime 0.22.0`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Claude Desktop</h2>
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
            Restart Claude Desktop after saving. Prime will persist all knowledge to{" "}
            <code>~/.prime/memory</code>.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Claude Code</h2>
          <p className="text-muted-foreground mb-3">One command, no config file to edit:</p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            {`claude mcp add prime allsource-prime \\
  --data-dir ~/.prime/memory \\
  --auto-inject`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Run <code>claude mcp list</code> to confirm the server registered.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            v0.22.0 — text-only embedding
          </h2>
          <p className="text-muted-foreground leading-relaxed">
            <code>prime_embed</code> and <code>prime_recall</code> now accept plain text and embed
            it server-side via the bundled fastembed model (AllMiniLML6V2, 384 dims). Agents no
            longer need a precomputed vector — pass <code>text</code> and Prime handles the rest.
            The first call in a process downloads the model (~25 MB) into the fastembed cache;
            subsequent calls take ~1–3 ms.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Auto-Inject Configuration</h2>
          <p className="text-muted-foreground mb-3">
            Auto-inject feeds the compressed index into every conversation automatically, so the
            agent always knows what it knows. Add the <code>--auto-inject</code> flag:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            {`{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/memory",
        "--auto-inject"
      ]
    }
  }
}`}
          </pre>
          <p className="text-muted-foreground mt-3 leading-relaxed">
            When enabled, Prime exposes a <code>prime://auto-context</code> resource that Claude
            reads at conversation start. This resource contains the compressed index — a
            token-efficient markdown summary of all stored knowledge (~500-1000 tokens).
          </p>
          <p className="text-muted-foreground mt-2 leading-relaxed">
            Control the maximum token budget with <code>--auto-inject-max-tokens</code>:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            {`"args": [
  "--data-dir", "~/.prime/memory",
  "--auto-inject",
  "--auto-inject-max-tokens", "2000"
]`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Per-Project Memory</h2>
          <p className="text-muted-foreground mb-3">
            Use separate <code>--data-dir</code> paths for isolated project contexts:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
            {`{
  "mcpServers": {
    "prime-work": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/projects/acme/.prime", "--auto-inject"]
    },
    "prime-personal": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/personal", "--auto-inject"]
    }
  }
}`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Each instance maintains its own graph, index, and vector store. No data leaks between
            projects.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Tool Reference</h2>
          <p className="text-muted-foreground mb-3">
            Prime exposes 19 MCP tools. Claude selects and calls them automatically based on
            conversation context.
          </p>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b">
                  <th className="p-2 text-left">Tool</th>
                  <th className="p-2 text-left">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_add_node</code>
                  </td>
                  <td className="p-2">
                    Create a knowledge node (person, concept, project, decision, …)
                  </td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_add_edge</code>
                  </td>
                  <td className="p-2">Create a directed relationship between two nodes</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_neighbors</code>
                  </td>
                  <td className="p-2">BFS the graph around a node (depth + relation filters)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_search</code>
                  </td>
                  <td className="p-2">List all nodes of a given type</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_shortest_path</code>
                  </td>
                  <td className="p-2">Find the chain of relationships between two entities</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_forget</code>
                  </td>
                  <td className="p-2">Soft-delete a node (preserved in event history)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_history</code>
                  </td>
                  <td className="p-2">Full event audit trail for any entity</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_stats</code>
                  </td>
                  <td className="p-2">Graph statistics (node/edge counts, types, relations)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_index</code>
                  </td>
                  <td className="p-2">
                    Compressed knowledge index — token-efficient markdown summary by domain
                  </td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_context</code>
                  </td>
                  <td className="p-2">
                    Tiered retrieval (L0 stats / L1 conversation / L2 full hybrid)
                  </td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_embed</code>
                  </td>
                  <td className="p-2">
                    Store a vector. Pass <code>text</code> alone (server embeds) or pass{" "}
                    <code>vector</code> directly
                  </td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_similar</code>
                  </td>
                  <td className="p-2">Find semantically similar embeddings to a stored vector</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">
                    <code>prime_recall</code>
                  </td>
                  <td className="p-2">
                    Hybrid recall: vectors + graph + temporal recency. Text or vector input.
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Agent Prompt Template</h2>
          <p className="text-muted-foreground mb-3">
            For best results, include a system prompt that teaches Claude how to use Prime. Add to
            your <code>CLAUDE.md</code> or system prompt:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
            {`You have access to Prime, a persistent knowledge graph for long-term memory.

## When to store
- Facts about people, projects, decisions, or relationships
- User preferences and corrections
- Important context that should persist across conversations

## When to recall
- Before answering questions about people, projects, or history
- When the user asks "what do you know about X"
- When you need context from previous conversations

## Workflow
1. Check prime_index at conversation start (or use auto-inject)
2. Use prime_recall for specific queries
3. Use prime_context for cross-domain questions
4. Store new facts with prime_add_node + prime_add_edge
5. Always set a domain (engineering, product, revenue, etc.)`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Cookbook Resource</h2>
          <p className="text-muted-foreground leading-relaxed">
            Prime exposes a <code>prime://cookbook</code> MCP resource containing worked examples
            and best practices for common memory patterns (team directories, decision logs, project
            tracking). Claude can read this resource on demand to learn advanced usage patterns.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Troubleshooting</h2>
          <ul className="space-y-3 text-sm text-muted-foreground">
            <li>
              <strong className="text-foreground">Binary not found:</strong> Ensure{" "}
              <code>allsource-prime</code> is in your <code>$PATH</code>. Run{" "}
              <code>which allsource-prime</code> to verify. If installed via
              <code> cargo install</code>, check that <code>~/.cargo/bin</code> is in your PATH.
            </li>
            <li>
              <strong className="text-foreground">Data directory permissions:</strong> Prime needs
              read/write access to the <code>--data-dir</code> path. Run{" "}
              <code>mkdir -p ~/.prime/memory && ls -la ~/.prime/memory</code> to verify. On macOS,
              Claude Desktop may need Full Disk Access for paths outside <code>~</code>.
            </li>
            <li>
              <strong className="text-foreground">Tools not appearing:</strong> After editing the
              config, fully quit and relaunch Claude Desktop (not just close the window). Check
              Claude Desktop&apos;s MCP logs for connection errors.
            </li>
            <li>
              <strong className="text-foreground">Auto-inject not working:</strong> Verify the{" "}
              <code>--auto-inject</code> flag is present in your args array. The{" "}
              <code>prime://auto-context</code> resource only appears when the graph has at least
              one node.
            </li>
          </ul>
        </section>
      </div>
    </div>
  );
}
