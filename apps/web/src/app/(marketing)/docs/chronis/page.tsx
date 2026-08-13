import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Chronis — Agent-Native Task CLI",
  description: `Chronis is an agent-native, event-sourced task CLI with TOON output for ~50% fewer tokens. Powered by ${siteConfig.name}.`,
  canonical: "/docs/chronis",
});

export default function ChronisPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">Chronis task CLI</h1>
      <p className="text-lg text-muted-foreground mb-4">
        The agent-native task CLI. Event-sourced, TOON-optimized, built for AI agents and humans
        alike.
      </p>
      <p className="text-sm text-muted-foreground mb-10">
        Inspired by{" "}
        <a
          href="https://github.com/Dicklesworthstone/beads_rust"
          target="_blank"
          rel="noopener noreferrer"
          className="text-foreground underline underline-offset-4 hover:opacity-80"
        >
          beads_rust
        </a>{" "}
        — the pioneering local-first issue tracker that proved agents can self-manage tasks via CLI.
        Chronis takes that idea further with event sourcing, temporal history, and token-optimized
        output.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-8">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Install</h2>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>cargo install chronis</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Quick Start</h2>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn init                                    # Create workspace
cn task create "Design auth module" -p p0  # Create a task
cn list --toon                             # Agent-optimized listing
cn ready --toon                            # Show unblocked tasks
cn claim <id> --toon                       # Claim a task
cn done <id> --toon                        # Complete it
cn archive --all-done                      # Clean up
cn sync                                    # Sync via git`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">TOON: Agent-Native Output</h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Pass <code className="text-primary">--toon</code> to any command for{" "}
            <a
              href="https://github.com/toon-format/toon"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              TOON (Token-Oriented Object Notation)
            </a>{" "}
            output — the same format used by AllSource&apos;s MCP server.{" "}
            <strong>~50% fewer tokens than JSON.</strong> No braces, no quotes, no wasted context
            window.
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`# Human output: ~180 tokens of box-drawing and padding
cn list
╭──────────┬──────┬──────────────────┬─────┬─────────────╮
│ ID       │ Type │ Title            │ Pri │ Status      │
├──────────┼──────┼──────────────────┼─────┼─────────────┤
│ t-abc1   │ task │ Design auth      │ p0  │ open        │
╰──────────┴──────┴──────────────────┴─────┴─────────────╯

# TOON output: ~60 tokens, same information
cn list --toon
[id|type|title|pri|status|claimed|blocked_by|parent|archived]
t-abc1|task|Design auth|p0|open||||false`}</code>
          </pre>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Mutations return single-line acks:{" "}
            <code className="text-primary">ok:claimed:t-abc1</code>,{" "}
            <code className="text-primary">ok:done:t-abc1</code>. An entire agent loop — ready,
            claim, work, done — burns ~20 tokens of CLI overhead instead of 200+.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Commands</h2>
          <div className="space-y-3">
            {[
              {
                name: "cn task create <title>",
                desc: "Create a task with -p, --type, --parent, --blocked-by, -d flags",
              },
              {
                name: "cn list [--status]",
                desc: "List tasks (--archived, --all, --toon)",
              },
              { name: "cn ready", desc: "Show open, unblocked tasks" },
              {
                name: "cn show <id>",
                desc: "Task details, children, and event timeline",
              },
              {
                name: "cn claim <id>",
                desc: "Claim a task (--cascade for epics, uses CN_AGENT_ID)",
              },
              {
                name: "cn done <id>",
                desc: "Complete a task (--reason, --cascade)",
              },
              { name: "cn approve <id>", desc: "Approve a task" },
              {
                name: "cn archive",
                desc: "Archive tasks (--all-done, --done-before, or specific IDs)",
              },
              { name: "cn unarchive <ids>", desc: "Restore archived tasks" },
              { name: "cn dep add/remove", desc: "Manage task dependencies" },
              {
                name: "cn sync",
                desc: "Git-based sync (pull, import, export, commit, push)",
              },
              {
                name: "cn tui",
                desc: "Interactive terminal UI dashboard",
              },
              {
                name: "cn serve",
                desc: "Embedded web viewer (Axum + HTMX)",
              },
            ].map((cmd) => (
              <div
                key={cmd.name}
                className="flex items-start gap-3 rounded-lg border border-border p-3"
              >
                <code className="text-sm font-mono text-primary shrink-0">{cmd.name}</code>
                <span className="text-sm text-muted-foreground">{cmd.desc}</span>
              </div>
            ))}
          </div>
          <p className="text-sm text-muted-foreground mt-3">
            All commands accept <code className="text-primary">--toon</code> for agent-optimized
            output.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Bulk Actions</h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Cascade operations apply to a task and all its children — close out an entire epic in
            one command:
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn claim <epic-id> --cascade              # Claim epic + all children
cn done <epic-id> --cascade               # Complete epic + all children
cn done <epic-id> --cascade --reason="Done"`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Archiving</h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Hide completed tasks from default listings without deleting them. Event-sourced —
            nothing is lost, everything is restorable.
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn archive --all-done            # Archive all completed tasks
cn archive --done-before 30      # Archive tasks done 30+ days ago
cn unarchive t-abc1              # Restore an archived task
cn list --archived               # Show only archived tasks`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Git Sync</h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Sync task state across machines via git. Events are exported to an append-only JSONL
            file — no merge conflicts, UUID-based dedup prevents duplicates.
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn sync   # pull → import → export → commit → push`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Chronis vs beads_rust</h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 pr-4 font-medium text-foreground">Feature</th>
                  <th className="text-left py-2 pr-4 font-medium text-muted-foreground">
                    beads_rust
                  </th>
                  <th className="text-left py-2 font-medium text-foreground">Chronis</th>
                </tr>
              </thead>
              <tbody className="text-muted-foreground">
                {[
                  ["Agent output", "JSON / human text", "TOON (~50% fewer tokens)"],
                  ["Storage", "SQLite + JSONL", "Event-sourced (WAL + Parquet)"],
                  ["History", "Current state only", "Full temporal event stream"],
                  ["Cascade ops", "Manual scripting", "--cascade flag"],
                  ["Archiving", "Not available", "cn archive --all-done"],
                  ["Approval gates", "Not available", "cn approve"],
                  ["Git sync", "commit && push", "Append-only JSONL + UUID dedup"],
                  ["TUI / Web UI", "Separate binaries", "Built-in (cn tui, cn serve)"],
                ].map(([feature, beads, chronis]) => (
                  <tr key={feature} className="border-b border-border/50">
                    <td className="py-2 pr-4 font-medium text-foreground">{feature}</td>
                    <td className="py-2 pr-4">{beads}</td>
                    <td className="py-2 font-medium text-foreground">{chronis}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="text-sm text-muted-foreground mt-3">
            Both tools share the same core workflow:{" "}
            <code className="text-primary">ready → claim → done → sync</code>. Migrate with{" "}
            <code className="text-primary">cn migrate-beads</code>.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Architecture</h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Chronis embeds AllSource Core as a library. Every mutation emits an event into the WAL.
            A TaskProjection folds events into queryable state in a DashMap (~12&micro;s reads, 469K
            events/sec throughput).
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`.chronis/
  wal/            # Write-ahead log (CRC32, fsync)
  storage/        # Parquet columnar storage
  sync/           # Git sync exchange (events.jsonl)
  config.toml     # Workspace config`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Source &amp; Docs</h2>
          <p className="text-muted-foreground leading-relaxed">
            Full source, README, and contribution guide on{" "}
            <a
              href="https://github.com/all-source-os/all-source/tree/main/apps/chronis"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              GitHub
            </a>
            . Published on{" "}
            <a
              href="https://crates.io/crates/chronis"
              target="_blank"
              rel="noopener noreferrer"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              crates.io
            </a>
            .
          </p>
        </section>
      </div>
    </div>
  );
}
