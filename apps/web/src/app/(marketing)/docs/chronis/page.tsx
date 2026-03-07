import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Chronis — Task CLI",
  description: `Chronis is an event-sourced task CLI powered by ${siteConfig.name}. Install with cargo install chronis.`,
});

export default function ChronisPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        Chronis
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        Event-sourced task CLI powered by AllSource. Every action is an immutable
        event — state is derived from projections over the event stream.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-8">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Install
          </h2>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>cargo install chronis</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Quick Start
          </h2>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn init                                    # Create workspace
cn task create "Design auth module" -p p0  # Create a task
cn task create "Write tests" --type=bug    # Create a bug
cn list                                    # List all tasks
cn ready                                   # Show unblocked tasks
cn claim <id>                              # Claim a task
cn done <id> --reason="Shipped"            # Complete it
cn sync --git                              # Sync via git`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Commands
          </h2>
          <div className="space-y-3">
            {[
              { name: "cn init", desc: "Initialize a .chronis/ workspace" },
              { name: "cn task create <title>", desc: "Create a task with -p, --type, --parent, --blocked-by, -d flags" },
              { name: "cn list [--status]", desc: "List tasks (--archived, --all flags available)" },
              { name: "cn ready", desc: "Show open, unblocked tasks" },
              { name: "cn show <id>", desc: "Task details, children, and event timeline" },
              { name: "cn claim <id>", desc: "Claim a task (uses CN_AGENT_ID env var)" },
              { name: "cn done <id>", desc: "Mark a task as done (optional --reason)" },
              { name: "cn approve <id>", desc: "Approve a task" },
              { name: "cn archive", desc: "Archive tasks (--all-done, --done-before, or specific IDs)" },
              { name: "cn unarchive <ids>", desc: "Restore archived tasks" },
              { name: "cn dep add/remove", desc: "Manage task dependencies" },
              { name: "cn sync --git", desc: "Pull, import, export, commit, push" },
              { name: "cn tui", desc: "Interactive terminal UI dashboard" },
              { name: "cn serve", desc: "Embedded web viewer (Axum + HTMX)" },
            ].map((cmd) => (
              <div
                key={cmd.name}
                className="flex items-start gap-3 rounded-lg border border-border p-3"
              >
                <code className="text-sm font-mono text-primary shrink-0">
                  {cmd.name}
                </code>
                <span className="text-sm text-muted-foreground">{cmd.desc}</span>
              </div>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Bulk Actions
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Cascade operations apply to a task and all its children — close out an
            entire epic in one command:
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn claim <epic-id> --cascade              # Claim epic + all children
cn done <epic-id> --cascade               # Complete epic + all children
cn done <epic-id> --cascade --reason="Done"`}</code>
          </pre>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Children are processed before the parent (bottom-up). Tasks already
            in the target state are skipped.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Archiving
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Hide completed tasks from default listings without deleting them.
            Archived tasks are preserved in the event stream and can be restored
            at any time.
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn archive t-abc1 t-abc2        # Archive specific tasks
cn archive --all-done            # Archive all completed tasks
cn archive --done-before 30      # Archive tasks done 30+ days ago
cn unarchive t-abc1              # Restore an archived task

cn list                          # Excludes archived (default)
cn list --archived               # Show only archived tasks
cn list --all                    # Show everything`}</code>
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Git Sync
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Sync task state across machines via git. Events are exported to an
            append-only JSONL file that git can merge naturally.
          </p>
          <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
            <code>{`cn sync --git   # pull → import → export → commit → push`}</code>
          </pre>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Each event is written once by its creating machine. UUID-based
            deduplication prevents duplicates across machines. Works with any git
            remote — GitHub, GitLab, or bare repos.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Architecture
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Chronis embeds AllSource Core as a library. Every mutation emits an
            event into the WAL. A TaskProjection folds events into queryable state
            in a DashMap (~12&micro;s reads, 469K events/sec throughput).
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
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Source &amp; Docs
          </h2>
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
