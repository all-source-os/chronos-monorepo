---
description: Upgrade mammoth from local-only to hosted cross-machine memory — onboard, get an API key, and restart the prime server with sync on.
---

Walk the user through turning on cross-machine / team memory. This is the
**upgrade**, not a gate — local-only already works without it. Only run this when
the user actually wants memory shared across machines or a team.

Steps:
1. **Confirm intent.** Cross-machine sync ships this machine's `prime.*` events to
   a hosted AllSource Core so other machines (and teammates on the same tenant)
   can recall them. It needs a free AllSource account + an API key. If the user
   only works on one machine, tell them local-only is enough and stop.
2. **Get an account + key.** If the `allsource-onboard` skill is available, use it
   — it registers on the hosted Core, creates an API key, and writes
   `.chronis/config.toml`. Otherwise point the user to sign up and mint a key,
   then have them paste it (treat the key as a secret — never echo it back or
   write it into Prime memory).
3. **Turn sync on.** The prime server must be restarted with sync flags. Show the
   user how to update their MCP config's `prime` args to add:
   `"--sync-to", "<core-url>", "--api-key", "<key>"`
   (or set `PRIME_SYNC_TO` / `PRIME_API_KEY` env vars). Default Core URL is
   `https://api.all-source.xyz` unless they self-host.
4. **Verify.** After they reload the agent, run `prime_stats` (or `/memory-status`)
   and confirm the server reports sync is active. New memories now replicate; the
   tenant's web Memory tab should show them.

Security: the API key is a credential. Never log it, never store it in a Prime
node or embedding, never commit it. It lives in the agent's MCP config / env only.

Note: hosted is metered/paid, not unbounded-free — there may be a plan limit.
Local-only remains the zero-cost default if the user doesn't upgrade.
