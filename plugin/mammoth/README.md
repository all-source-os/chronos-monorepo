# mammoth 🦣

**Durable memory for your coding agent.** Decisions, task history, and code
context from past sessions get remembered and recalled automatically — so you
stop re-explaining your project every single time. Works locally with zero
signup (your memory, your disk, survives reboots), and syncs across machines when
you want it to. Any MCP agent — Claude Code, Cursor, Cline, Windsurf, Codex,
Gemini — speaks it natively.

> *caveman make few token. mammoth never forget token.*

mammoth is the distribution front-door for two AllSource engines:
[`allsource-prime`](https://crates.io/crates/allsource-prime) (semantic recall —
graph + vector + temporal over a durable event store) and
[`chronis`](https://crates.io/crates/chronis) (`cn`, event-sourced task/work
memory). caveman compresses what you *say*; mammoth remembers what you *said*.
They compose.

## What you get

A `mammoth-memory` skill that runs a recall/record loop, plus the Prime MCP
server wired in via `.mcp.json` exposing 13 `prime_*` tools (`prime_recall`,
`prime_context`, `prime_add_node`, `prime_embed`, `prime_stats`, …) and the
`prime://auto-context` resource — a compressed knowledge index auto-injected into
every conversation.

## Prerequisite

The `allsource-prime` binary must be on your `PATH`:

```bash
cargo install allsource-prime
```

(Requires `>= 0.21.3` for in-process text embedding via fastembed — no external
embedding service.)

## Install (Claude Code)

Add the marketplace and install the plugin:

```
/plugin marketplace add all-source-os/chronos
/plugin install mammoth
```

Restart Claude Code and approve the project `prime` MCP server when prompted. The
`mammoth-memory` skill activates automatically; recall fires before
prior-knowledge questions and salient facts get recorded as you work.

## How it stores

Local-only by default: `allsource-prime --data-dir <project>/.prime` runs a fully
durable store — WAL (CRC32 + fsync) + Snappy Parquet snapshots, the same
AllSource Core engine. Your memory survives restarts and never leaves your
machine. No account required.

Cross-machine / team memory is the upgrade: add `--sync-to <url> --api-key <key>`
to ship `prime.*` events to a hosted AllSource Core. Free tier remembers across
sessions on *this machine*; cross-machine and team memory need a free AllSource
account.

## Provenance

mammoth records its own analysis — decisions, rationale, gotchas, outcomes,
metadata, file paths. It does **not** write raw source code, secrets, or full
file contents into memory unless you explicitly allow it.

## License

MIT (plugin layer). The engines: `allsource-prime` and `allsource-core` are
Apache-2.0; `chronis` is MIT.
