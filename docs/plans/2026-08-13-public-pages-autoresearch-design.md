# Public-page clarity autoresearch

## Audience and page jobs

AllSource serves developers and technical buyers deciding whether they need a durable event store, then trying to make one write and one read. Homepage must identify product in first sentence. Platform, solution, and comparison pages must answer one evaluation question. Docs and install pages must lead to one runnable next step. Blog, status, and legal pages must remain easy to identify and leave.

## Chosen approach

Use one shared public shell, one plain vocabulary, and route-family-specific intros. Preserve existing AllSource midnight/cyan identity. Replace decorative client navigation and entrance motion before rewriting isolated cards. Every page gets a visible route back to product, docs, pricing, and signup.

Rejected alternatives:

- Rewrite 51 pages independently: high drift, duplicated shell defects, weak measurement.
- Homepage-only redesign: leaves docs and long-tail landing pages stranded.
- Visual rebrand: clarity problem is hierarchy and language, not palette.

## Visual direction

- Palette: Night `#0b1522`, Ink `#f3f5f7`, Cyan `#08b6df`, Slate `#9aa5b1`, Rule `#263547`.
- Type: existing system sans for prose; monospace only for events, commands, identifiers, and measured values.
- Layout: direct navigation; page thesis; one explanatory lead; primary and secondary actions; evidence; detail.
- Motion: no generic entrance choreography. Motion stays only where it explains live product state.
- Signature: concrete event path — **write → inspect → query** — supported by real code, event names, and measured system facts.

## Autoresearch contract

Frozen corpus: 51 prerendered public HTML routes from commit `51a5cd2`; dashboard, API, account, onboarding, and UI-test routes excluded. Dynamic blog articles inherit shared shell and are browser-checked separately.

Primary performance scalar, lower is better: summed raw unique JavaScript referenced per route, measured by `tooling/route-weight` after a production build. Baseline: **41,575,318 B**.

Clarity gates:

1. Exactly one `h1` and one `main` landmark per route.
2. Shared navigation and footer on every route.
3. H1 names product, task, audience, or comparison; generic one-word headings gain context.
4. First lead explains page outcome without relying on surrounding navigation.
5. Marketing pages offer one primary action and one lower-commitment path.
6. Link and button labels name their destination or result.
7. Desktop and mobile navigation, focus, reduced motion, and page hierarchy pass browser proof.

Keep correctness and clarity wins even when they add a small byte cost; record that trade-off. Stop after six scored iterations or three consecutive discards.
