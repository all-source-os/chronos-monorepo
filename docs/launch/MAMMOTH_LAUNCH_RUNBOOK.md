# mammoth — launch runbook (bead t-dc63)

> All launch *assets* are written and committed. This runbook sequences the
> **owner-only execution** (publishing to GitHub / a marketplace / social, with
> your accounts). Nothing here can be automated by an agent — it's the human
> checklist. Drafted 2026-05-31.

## Assets (all committed, ready)

| asset | path | used for |
|---|---|---|
| Launch blog | `apps/web/content/mammoth-durable-agent-memory.mdx` | `/blog/mammoth-durable-agent-memory` on the site |
| Social threads | `docs/social/mammoth-launch-x-thread.md` | X thread + Show HN + LinkedIn |
| Marketplace listing copy | `plugin/mammoth/LISTING.md` | the plugin card + pre-publish checklist |
| Plugin README / INSTALL / UPGRADE | `plugin/mammoth/{README,INSTALL,UPGRADE}.md` | repo landing + install docs |
| Pricing decision | `docs/proposals/MAMMOTH_HOSTED_PRICING_OPTIONS.md` | hosted tier (Option B) |
| Benchmark harness | `tooling/mammoth-bench/` | the proof skeptics can re-run |

Headline number (consistent across all assets): **recall hit@5 0.90 vs 0.83
search+grep, ~3ms recall, durability PASS.** One-liner: *caveman make few token.
mammoth never forget token.*

## Pre-flight gates (must ALL be green before posting)

These are owner actions — an agent can't capture a GIF, push your repo, or post:

1. [ ] **Push `main`.** The marketplace + blog resolve from the repo. ~14 mammoth
       commits are local-only. Review the non-mammoth dirty files from the
       concurrent session first, then `git push origin main`.
2. [ ] **Capture the demo GIF/video** — the magic moment (fresh session recalls an
       earlier-session decision from a reworded query, nothing pasted). *Single
       most important asset; the whole launch leads with it.*
3. [ ] **Render the benchmark table image** for the card + tweet 3.
4. [ ] **Plugin icon** in `plugin/mammoth/assets/` (mammoth mark, caveman-register).
5. [ ] **Clean-machine install smoke test** — `/plugin marketplace add
       all-source-os/chronos` → `/plugin install mammoth` → approve prime → tools
       appear. Do NOT launch if this fails (the verify-before-listing rule).
6. [ ] **Verify live URLs** — `homepage` (`all-source.xyz/prime`) and `repository`
       (`github.com/all-source-os/chronos`) both resolve publicly.
7. [ ] **Blog renders** — confirm `/blog/mammoth-durable-agent-memory` builds on
       Vercel (frontmatter is schema-correct; verified no broken MDX imports).

## Launch sequence (owner, once gates green)

Order matters — each step's link feeds the next.

1. **Publish the plugin** (`t-8882`): push done → `/plugin marketplace add
   all-source-os/chronos`; tag a release if the marketplace keys off tags (follow
   the repo's immutable-tag policy).
2. **Ship the blog** — it's already in `apps/web/content/`; a Vercel deploy on
   `main` push publishes it. Confirm the OG image renders (`/og` auto-gen).
3. **Post Show HN** — title + body from `docs/social/mammoth-launch-x-thread.md`.
   Lead with the demo + the reproducible benchmark; HN rewards runnable proof.
   *Post in the morning ET on a weekday for best ranking.*
4. **Post the X thread** — 6 tweets from the same file. Tweet 1 = meme line +
   demo GIF. Tweet 3 = bench image. **Tag/cross-link @caveman** — complementary,
   not competitive; ride its audience.
5. **LinkedIn post** — the single-post version from the same file.
6. **Cross-link everything** — blog links the repo + benchmark; threads link the
   blog; the repo README links the blog. caveman ↔ mammoth both ways if possible.

## Go-metric (from the proposal)

- Stars + install velocity in the first **72h** (the caveman-style curve signal).
- Hosted conversion from the `/memory-status` → `/memory-sync` upgrade prompt
  above a pre-set floor. If local installs spike but hosted converts ~0%, that's
  signal to rethink monetization — **not** to gate the free tier.
- **Do not launch on mediocre numbers** — if the clean-machine smoke test or the
  demo GIF doesn't land the magic moment, fix first.

## Honesty (the trust move — keep it in every post)

Lead with the real benchmark *and* the narrowing note: the edge over grep is
+0.07 at 60 memories (was +0.17 at 18). Publishing the softening is the same
trust move caveman made with "affects output tokens only." Don't round it up.

## What's NOT done by this runbook

Posting, GIF capture, repo push, marketplace publish, and the pricing *numbers*
(structure is Option B per `t-a238`; price/cap values need a Core Fly unit-cost
estimate). All owner actions.
