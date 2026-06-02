# prime voice file — launch runbook

> All launch *assets* are written and committed. This runbook sequences the
> **owner-only execution** (capturing the demo clip, pushing, publishing to the
> blog / HN / social with your accounts). Nothing here can be automated by an agent
> — it's the human checklist. Drafted 2026-06-02. Mirrors
> `docs/launch/MAMMOTH_LAUNCH_RUNBOOK.md`.

## What this campaign is

Ride the viral "your voice is your last competitive moat" thesis and land it on
prime: the static 4k-token markdown voice file is the v0; prime is the durable,
queryable, portable, team-shareable version — proven by a real voice-ON vs
voice-OFF demo and `prime_recall` hitting the right facet by meaning.

**The hero is `prime_recall`.** The auto-compressed index (`prime_index`), one-file
export, and `--auto-inject` now work too (fixed in `allsource-prime 0.21.6`, commit
4b61441) and may be presented as shipped. The ONE thing not to over-claim:
`prime_context`'s L2 *vector* arm still returns empty (a documented TODO) — never
claim `prime_context` returns vector hits; point people at `prime_recall` for the
vector path. This is the cardinal rule — over-claiming on an identity product
destroys the trust that is the entire point.

## Assets (all committed, ready)

| asset | path | used for |
|---|---|---|
| Launch blog | `apps/web/content/your-voice-file-shouldnt-be-dead-markdown.mdx` | `/blog/your-voice-file-shouldnt-be-dead-markdown` on the site |
| Social pack | `docs/social/prime-voice-file-campaign.md` | LinkedIn + X thread + Show HN + visual notes |
| Comparison snippet | `docs/sales/voice-file-comparison.md` | reusable on the `/compare` surface + sales |
| Design + question bank | `docs/proposals/PRIME_VOICE_FILE.md` | the spec the campaign advertises |
| Skill + command | `plugin/mammoth/skills/voice-identity/SKILL.md`, `plugin/mammoth/commands/voice.md` | the `/voice` flow the campaign tells people to run |
| Captured demo | `tooling/voice-demo/RESULTS.md` | every number in every asset traces here |

Headline numbers (consistent across all assets, all trace to RESULTS.md):
**12 facets recorded; `prime_recall` top hit 0.757 on a reworded query; the
populated `prime_index` (12 nodes / 5 domains / 77 tokens); voice-ON vs voice-OFF
on the same prompt.** One-liner: *your voice file shouldn't be a dead markdown file.*

## Pre-flight gates (must ALL be green before posting)

These are owner actions — an agent can't capture a clip, push your repo, or post:

1. [ ] **Capture the demo clip** — the magic moment: `/voice run` (or a pre-seeded
       store), then ask the agent to write something; it recalls the relevant voice
       facets and the draft sounds like the user. *Single most important asset; the
       whole launch leads with it.* If the clip doesn't land the magic moment, fix
       before launching.
2. [ ] **Render the supporting images** — the `prime_recall` results table (tweet 3)
       and the voice-ON / voice-OFF side-by-side (tweet 4), both from RESULTS.md.
3. [ ] **Blog hero image** — `apps/web/public/assets/blog/your-voice-file-shouldnt-be-dead-markdown.webp`
       (owner to render; referenced by the MDX frontmatter `image`).
4. [ ] **Blog renders on Vercel** — confirm `/blog/your-voice-file-shouldnt-be-dead-markdown`
       builds. Frontmatter is schema-correct (title / publishedAt / summary /
       author "all.source team" / category "product" / image). **Zero `import`
       lines, zero `<Component>` JSX** — verified; do not add any (a prior post
       broke the build importing nonexistent `@/components/mdx/*`).
5. [ ] **Every number traces to RESULTS.md** — the only metrics in any asset are:
       12 facets, top hit 0.757, the named recalled facets, the voice-ON/OFF
       contrast. No invented numbers.
6. [ ] **No over-claim audit** — `prime_index`, `/voice export`, `--auto-inject`,
       and "compressed index/export" are shipped (0.21.6) and may be presented as
       working. The single thing to guard against: any claim that `prime_context`'s
       *vector* arm returns hits — it returns empty (documented TODO); the vector
       path is `prime_recall`. Grep each asset and confirm none over-claim that.
7. [ ] **Push `main`.** Blog + plugin resolve from the repo. `git status` first,
       include related dirty files, `git push origin main`.
8. [ ] **Clean-machine smoke test** — `cargo install allsource-prime` →
       `/plugin install mammoth` → approve prime → `/voice run` records a facet →
       `prime_recall` returns it. Do NOT launch if this fails.
9. [ ] **Verify live URLs** — `all-source.xyz/prime` and
       `github.com/all-source-os/chronos` resolve publicly; the
       `tooling/voice-demo/RESULTS.md` link resolves.

## Launch sequence (owner, once gates green)

Order matters — each step's link feeds the next.

1. **Push `main`** — the blog ships on the Vercel deploy from a `main` push.
2. **Ship the blog** — confirm `/blog/your-voice-file-shouldnt-be-dead-markdown`
   renders and the OG image resolves (`/og` auto-gen).
3. **Post Show HN** — title + body from `docs/social/prime-voice-file-campaign.md`.
   Lead with the demo + the real captured recall numbers; HN rewards runnable
   proof, and the honest residual-limit note plays well there. *Morning ET weekday.*
4. **Post the X thread** — 7 tweets from the same file. Tweet 1 = moat line + demo
   clip. Tweet 3 = recall table. Tweet 4 = voice-ON/OFF. **Cross-link @caveman +
   mammoth** — same family, complementary.
5. **LinkedIn post** — the single-post version from the same file, in the source
   post's register (short lines, the closing question).
6. **Cross-link everything** — blog links the repo + RESULTS.md; threads link the
   blog; the repo README links the blog. caveman ↔ mammoth ↔ voice all three ways
   if possible.

## Honesty (the trust move — keep it in every post)

Lead with `prime_recall` (proven: top hit 0.757 on a reworded query) and the
voice-ON/OFF demo. The auto-compressed index (`prime_index`), one-file export, and
`--auto-inject` now work as of 0.21.6 (the 0-node projection bug is fixed, commit
4b61441) and may be shown as shipped — `prime_index` returns the live, populated
index (12 nodes / 5 domains / 77 tokens). Keep ONE honest residual line: the
*vector* sub-arm of `prime_context` L2 still returns empty (a documented `// TODO:
vector search integration`), so the vector path is `prime_recall`. Publishing that
one limit is the same trust move mammoth made with the narrowing-edge benchmark
note. Never claim `prime_context` returns vector hits.

## Go-metric

- Stars + `/voice run` install velocity in the first **72h**.
- Blog → repo click-through and Show HN ranking.
- Hosted conversion from the team-voice (`--sync-to`) upgrade prompt above a floor.
  If local installs spike but hosted converts ~0%, that's signal to rethink
  monetization — **not** to gate the free local tier.
- **Do not launch on a weak demo clip** — if it doesn't land the magic moment, fix
  first.

## What's NOT done by this runbook

Capturing the demo clip, rendering the images/hero, repo push, blog publish, HN +
X + LinkedIn posting. All owner actions — nothing here is posted or pushed by an
agent.
