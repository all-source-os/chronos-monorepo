# Ledger — public page clarity

Date: 2026-08-13. Starting commit: `51a5cd2`.

## Corpus baseline

- 51 prerendered public routes.
- Raw route JavaScript total: **41,575,318 B**.
- 38/51 routes include primary navigation.
- 38/51 routes include footer navigation.
- 38/51 routes include a `main` landmark.
- 49/51 routes contain exactly one `h1`; Pricing and Connect contain none.

Dynamic blog articles are not part of the static byte scalar. Shared-shell behavior and a real article route remain browser gates.

## Iterations

| # | Proposal | JS total | Clarity result | Verdict |
|---|---|---:|---|---|
| 0 | Baseline | 41,575,318 B | Nav 38/51 · footer 38/51 · main 38/51 · one H1 49/51 | — |
| 1 | Add one shared public shell; replace client dropdown navigation with direct server-rendered links | **37,453,096 B** | Nav, footer, and main 51/51 | **Keep** — all stranded routes gain a way home while corpus drops 4,122,222 B (−9.91%). |
| 2 | Server-render Connect thesis; give Pricing a real H1; replace generic docs headings and ambiguous “free” trial labels | **37,314,146 B** | Exactly one H1 51/51; trial length explicit | **Keep** — semantics reach full corpus and JavaScript drops another 138,950 B (−0.37%). |
| 3 | Replace decorative `BlurFade` across public content with stable server-rendered sections | **36,195,372 B** | Titles and content paint immediately; reduced-motion no longer depends on runtime observers | **Keep** — eight route families drop the 134 KB motion runtime; corpus falls 1,118,774 B (−3.00%). |
| 4 | Replace homepage slogans and duplicate signup form with one concrete event-store promise and two direct paths; remove decorative background and redundant product sections | **36,162,626 B** | First viewport now identifies product, use, hosted price, and self-host option without animation | **Keep** — clearer buying path and another 32,746 B removed (−0.09%). |
| 5 | Render content-page reveal and hover wrappers as plain HTML; retain motion only where it communicates live event flow | **33,628,992 B** | Page copy is present at first paint and no longer waits for viewport observers | **Keep** — 19 route families shed the 133 KB Motion runtime; corpus falls 2,533,634 B (−7.01%). |
| 6 | Replace homepage auto-rotating explainers and unsupported competitor matrix with static product steps and reproducible proof; rewrite vague solution heroes and reconcile MCP counts | **33,601,535 B** | Route openings now name the object, operation, and buyer outcome; tenant (55+) and fleet (73) tool scopes are explicit; Prime count matches its 19 dispatch arms | **Keep** — clearer corpus, fewer unsupported absolutes, and another 27,457 B removed (−0.08%). |
| 7 | Limit homepage publishing feed to three recent articles and link to the complete blog | **33,601,535 B** | Final conversion action and footer now follow three cards instead of 35 | **Keep** — route JavaScript stays flat while page length and competing links fall sharply. |
| 8 | Remove residual template slogans and setup-duration promises found during browser verification | **33,601,576 B** | Event Store, Stream Processing, Prime, and Prime docs use task-based headings instead of hype or countdowns | **Keep** — clarity improves for a negligible 41 B corpus change. |
| 9 | Fix production blog path resolution, preserve supplied document titles, remove stale/absolute proof claims, explain live-price failure, and repair responsive public CTAs | **33,602,218 B** | Dynamic articles render from the standalone working directory; route titles are unique; benchmark and MCP labels state scope; all public routes fit 390 px | **Keep** — release proof closes production, SEO, accuracy, and mobile defects with flat route weight. |

## Final result

- Public-route JavaScript: **33,602,218 B**, down **7,973,100 B (19.18%)** from baseline.
- Shared navigation, footer, `main`, and exactly one `h1`: **51/51** prerendered public routes.
- Motion remains only in the homepage event-flow demo, where changing state explains product behavior.
- Dynamic blog pages inherit the same public shell and remain a browser verification gate.
