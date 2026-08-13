# Ledger — dashboard route family

Date: 2026-08-13. Starting commit: `89c92ce`.

## Corpus and scalar

Corpus: every prerendered signed-in page under `apps/web/src/app/dashboard`.

Scalar: total raw bytes of unique Next.js JavaScript chunks referenced per page, summed across all 13 pages. Lower is better. `tooling/route-weight` rejects missing chunks.

## Baseline

| Route | Raw JS | Chunks |
|---|---:|---:|
| `/dashboard` | 903,101 | 16 |
| `/dashboard/analytics` | 1,438,758 | 19 |
| `/dashboard/api-keys` | 1,019,312 | 16 |
| `/dashboard/billing` | 1,022,872 | 16 |
| `/dashboard/demo` | 1,479,189 | 19 |
| `/dashboard/demo/onboarding` | 861,833 | 15 |
| `/dashboard/events` | 1,089,445 | 19 |
| `/dashboard/memory` | 1,033,164 | 17 |
| `/dashboard/pipelines` | 1,411,087 | 18 |
| `/dashboard/settings` | 1,004,700 | 16 |
| `/dashboard/settings/audit-log` | 997,699 | 16 |
| `/dashboard/team` | 1,014,771 | 16 |
| `/dashboard/tools/replay` | 1,003,383 | 16 |
| **Total** | **14,279,314** | — |

Largest identifiable costs:

- Recharts route chunk: 366,704 B.
- Motion runtime: 134,072 B.
- React DOM: 228,970 B, shared and framework-bound.

## Iterations

| # | Proposal | Total | Delta | Verdict |
|---|---|---:|---:|---|
| 0 | Baseline | 14,279,314 B | — | — |
| 1 | Replace dashboard `BlurFade` wrappers with static `FadeIn`; fix accessibility defects exposed by changed-file gate | **12,747,662 B** | **−1,531,652 B (−10.73%)** | **Keep** — eleven routes drop the 134 KB motion runtime and stop decorative page-load movement. TypeScript, build, and changed-file lint pass. |
| 2 | Lazy-load Recharts analytics/projection views and demo engines behind stable skeletons | **11,406,426 B** | **−1,341,236 B (−10.52%)** | **Keep** — Analytics drops 439,326 B, Demo 485,623 B, and Projections 403,332 B. Every route improves. TypeScript and production build pass. |
| 3 | Replace inert Events calendar with a working native date filter; lazy-load the closed event drawer | **11,345,608 B** | **−60,818 B (−0.53%)** | **Keep** — Events drops 39,218 B, removes a deceptive control, and gains tested search/date behavior. Every route improves. TypeScript, 65 web tests, and production build pass. |
| 4 | Replace header time-travel calendar/popover stack with labelled native date/time inputs and a keyboard-safe dialog | **9,773,503 B** | **−1,572,105 B (−13.86%)** | **Keep** — twelve routes drop about 132 KB; Replay already pays most calendar cost elsewhere. Trigger state, dismissal, and form labels are now explicit. TypeScript and production build pass. |
| 5 | Add visible, retryable load failures to API Keys, Team, Projections, Audit Log, and Replay | **9,778,198 B** | **+4,695 B (+0.05%)** | **Keep for correctness** — failed requests no longer masquerade as empty data or exist only in console output. TypeScript, changed-file lint, and production build pass. |
| 6 | Replace Replay calendar popovers with labelled native date/time controls | **9,646,660 B** | **−131,538 B (−1.35%)** | **Keep** — Replay drops 131,733 B and two initial chunks while preserving local date-to-UTC conversion at submission. TypeScript, changed-file lint, and production build pass. |

Iteration 1 route totals:

| Route | Raw JS | Delta |
|---|---:|---:|
| `/dashboard` | 903,389 | +288 |
| `/dashboard/analytics` | 1,299,355 | −139,403 |
| `/dashboard/api-keys` | 880,079 | −139,233 |
| `/dashboard/billing` | 883,577 | −139,295 |
| `/dashboard/demo` | 1,339,937 | −139,252 |
| `/dashboard/demo/onboarding` | 861,977 | +144 |
| `/dashboard/events` | 950,155 | −139,290 |
| `/dashboard/memory` | 893,863 | −139,301 |
| `/dashboard/pipelines` | 1,271,929 | −139,158 |
| `/dashboard/settings` | 865,413 | −139,287 |
| `/dashboard/settings/audit-log` | 858,410 | −139,289 |
| `/dashboard/team` | 875,484 | −139,287 |
| `/dashboard/tools/replay` | 864,094 | −139,289 |

Iteration 2 route totals:

| Route | Raw JS | Delta vs iteration 1 |
|---|---:|---:|
| `/dashboard` | 902,017 | −1,372 |
| `/dashboard/analytics` | 860,029 | −439,326 |
| `/dashboard/api-keys` | 878,786 | −1,293 |
| `/dashboard/billing` | 882,296 | −1,281 |
| `/dashboard/demo` | 854,314 | −485,623 |
| `/dashboard/demo/onboarding` | 860,718 | −1,259 |
| `/dashboard/events` | 948,805 | −1,350 |
| `/dashboard/memory` | 892,544 | −1,319 |
| `/dashboard/pipelines` | 868,597 | −403,332 |
| `/dashboard/settings` | 864,144 | −1,269 |
| `/dashboard/settings/audit-log` | 857,158 | −1,252 |
| `/dashboard/team` | 874,197 | −1,287 |
| `/dashboard/tools/replay` | 862,821 | −1,273 |

Iteration 4 route totals:

| Route | Raw JS | Delta vs iteration 3 |
|---|---:|---:|
| `/dashboard` | 768,522 | −131,695 |
| `/dashboard/analytics` | 726,534 | −131,695 |
| `/dashboard/api-keys` | 745,291 | −131,695 |
| `/dashboard/billing` | 749,062 | −131,434 |
| `/dashboard/demo` | 720,819 | −131,695 |
| `/dashboard/demo/onboarding` | 727,223 | −131,695 |
| `/dashboard/events` | 777,925 | −131,662 |
| `/dashboard/memory` | 767,265 | −123,479 |
| `/dashboard/pipelines` | 735,102 | −131,695 |
| `/dashboard/settings` | 730,649 | −131,695 |
| `/dashboard/settings/audit-log` | 723,663 | −131,695 |
| `/dashboard/team` | 740,702 | −131,695 |
| `/dashboard/tools/replay` | 860,746 | −275 |

Iteration 4 result: **9,773,503 B**, down **4,505,811 B (31.55%)** from baseline across the full corpus.

Final result after six experiments and browser-found accessibility fixes: **9,646,660 B**, down **4,632,654 B (32.44%)** from baseline. Every route passes its baseline; Replay falls from 1,003,383 B and 16 chunks to 729,863 B and 13 chunks.

## Behavior review

Tracked alongside scalar:

- Every visible action has a destination or handler.
- Every page names its job and primary next action.
- Empty/error states explain recovery.
- Mobile navigation closes after selection.
- Dialogs and drawers remain reachable, centered, and dismissible.
- Data pages avoid decorative motion and zero-value flicker.

## Browser proof

Proofshot exercised all 13 routes at 1440×900 and the Events route at 390×844. It found and drove two shared-shell fixes: mobile header offset no longer pushes controls off-screen, and the Time Travel trigger now has an accessible name with a viewport-contained dialog. Mobile sidebar navigation reaches the selected route and closes. Final capture reports zero console errors and zero server errors; expected local Vercel Analytics script-block logs remain informational.
