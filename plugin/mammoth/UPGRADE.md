# Upgrading mammoth: local-only → hosted (cross-machine) memory

mammoth is **local-only by default** — durable memory on your disk, no account,
zero cost. That's the right tier for one machine. Upgrade only when you want
memory **across machines** (laptop + desktop) or **shared with a team**.

> Free tier remembers across sessions on *this machine*, no account. Cross-machine
> and team memory are the upgrade — a pull, not a gate.

## What changes

Local-only stores `prime.*` events on disk under `--data-dir`. The hosted upgrade
adds a background sync loop that ships those events to a hosted AllSource Core on
your tenant, so any machine signed into the same tenant recalls the same memory,
and the web Memory tab shows it. Nothing about the local store changes — sync is
purely additive.

## Steps

### 1. Get a free AllSource account + API key

Use the `allsource-onboard` skill (registers on the hosted Core, mints an API key,
writes `.chronis/config.toml`), or sign up at https://www.all-source.xyz and mint
a key in the dashboard.

### 2. Add sync flags to the prime server

Update your agent's MCP config so the `prime` server starts with sync on:

```jsonc
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [
        "--data-dir", "~/.prime/memory",
        "--auto-inject",
        "--sync-to", "https://api.all-source.xyz",
        "--api-key", "<your-key>"
      ]
    }
  }
}
```

Or via environment instead of inline args: `PRIME_SYNC_TO` + `PRIME_API_KEY`
(keeps the key out of a committed config). Self-hosters point `--sync-to` at their
own Core.

### 3. Reload + verify

Restart the agent, then run `/memory-status` (or `prime_stats`). Confirm sync is
active. New memories replicate on the configured interval; existing local memories
sync up too.

## Security

The API key is a **credential**:
- Never commit it. Prefer `PRIME_API_KEY` (env) over an inline `--api-key` in a
  tracked config. `.mcp.json` is gitignored by default in this repo for this reason.
- Never write it into a Prime node or embedding.
- Rotate it from the dashboard if exposed.

## Cost

Hosted is metered/paid, not unbounded-free — check your plan's limits. Local-only
stays free forever; you can run both (local store + sync) or drop sync anytime by
removing the flags.
