# Ledger — customer dashboard JS weight

Autoresearch loop over the client JavaScript the customer dashboard pulls in.

## Setup

**Frozen corpus.** The `/dashboard` route of `apps/web`, prerendered by
`bun run build` (Next.js 16.2.7, Turbopack) with dependencies resolved from the
committed `bun.lock`. Starting commit `fee857b`. The route, the build command,
and the lockfile did not change during the loop.

**Scalar (lower is better).** Total raw bytes of the *unique*
`/_next/static/**.js` chunks referenced by `.next/server/app/dashboard.html`,
measured by `tooling/route-weight`.

Raw, not gzipped: raw bytes are what the bundler controls and what actually
moves when a dependency leaves the graph. Unique, because the browser fetches a
repeated chunk once. Deterministic for a given build, so there is no noise band
and no need for repeated sampling.

Why not "First Load JS": Next 16 on Turbopack no longer prints that column, so
there is no build-output number to optimise against. Hence the harness.

Note on what this measures: `/dashboard` is auth-gated and its prerendered HTML
is a shell whose only visible text is "AllSource Loading…". The scalar is
therefore the **boot bundle** — the JS a signed-in user must download before the
dashboard can render anything at all. That is the right target for perceived
load, but it is not a measure of runtime responsiveness once booted.

**Gate.** A proposal had to satisfy ALL of these to be eligible:

1. `bun run build` exits 0.
2. `bunx tsc --noEmit` is clean.
3. `bunx biome check` is clean on every file the proposal touched.
4. `route-weight` reports **0 missing chunks** — a referenced-but-absent chunk
   would silently lower the scalar and read as a win.
5. `dashboard.html` still renders the shell (contains "Loading"). This is the
   anti-cheat: without it, deleting the dashboard is the global optimum.

The gate did reject work: iteration 1 failed check 3 on first run (formatting
and import ordering) and had to be fixed before it could be scored.

**Baseline.** `1,029,112 B` across 17 chunks.

Composition at baseline, by fingerprinting the minified chunks:

| Bytes | Contents |
|---|---|
| 228,970 | react-dom — framework, irreducible |
| 154,330 | unidentified |
| 134,072 | **`motion` (motion/react)** |
| 112,594 | unidentified |
| 97,782 | radix + date-fns + inline SVG icons |

**Stop conditions declared up front.** 8 iterations, or 3 consecutive discards.

## Ledger

| # | Proposal | Scalar | Δ vs baseline | Verdict |
|---|---|---|---|---|
| 0 | Baseline | 1,029,112 B (17 chunks) | — | — |
| 1 | Replace `BlurFade` (motion/react) with a CSS `FadeIn` on the dashboard boot path | **889,434 B** (15 chunks) | **−139,678 (−13.6%)** | **Keep** — `motion` was in the boot bundle solely to fade cards in; `tailwindcss-animate` does the same thing with no runtime. Two whole chunks left the graph. |
| 2 | Add `experimental.optimizePackageImports` for `@allsource/ui` + `lucide-react` | 889,434 B | 0 (±0) | **Discard** — exactly zero bytes. Turbopack already does this. Reverted rather than keep config surface that buys nothing. |
| 3 | Replace date-fns `format(d,"yyyy-MM-dd")` with a local `toLocalYmd` helper | 889,459 B | +25 vs #1 | **Keep, but NOT as a performance win** — the scalar got *worse* by 25 B; date-fns was already tree-shaken to almost nothing. Kept because the helper fixes a real bug (below). Recorded as a correctness change that costs 25 bytes. |

**Net: 1,029,112 → 889,459 B. −139,653 (−13.57%), 17 → 15 chunks.**

Every byte of that is iteration 1. Iterations 2 and 3 moved nothing and are
recorded so nobody retries them.

## Bug found mid-loop (not a performance change)

`time-travel-picker.tsx` initialised its two inputs from **different clocks** —
the date via `toISOString()` (UTC), the time via `toTimeString()` (local) —
and `handleCustomDateTimeApply` then recombines them into one *local* `Date`.
For any user not on UTC, the two inputs disagreed for part of every day, so the
time-travel control queried a point up to a day off. Fixed in iteration 3 by
routing both through `toLocalYmd`.

This is why iteration 3 was kept despite costing 25 bytes.

## Still open

- **266,924 B across two unidentified chunks** (154,330 + 112,594). Fingerprinting
  by library token failed on both. Identifying them is the highest-value next
  step — together they are ~30% of the remaining bundle.
- **react-dom is 228,970 B** and irreducible without changing framework.
- **`motion` still ships elsewhere.** Only the dashboard boot path was cleared.
  `BlurFade` is still used on `/dashboard/demo` and `/dashboard/settings/audit-log`,
  which pay the 134 KB on their own routes.
- **The scalar cannot see runtime cost.** Data-fetch waterfalls, re-render
  storms, and N+1 API calls from the dashboard hooks are invisible to it. If
  "dashboard performance" means time-to-interactive rather than bytes, that
  needs a different frozen corpus and a different scalar.
- The loop **stopped early by operator decision after 3 iterations**, not because
  a declared stop condition tripped.
