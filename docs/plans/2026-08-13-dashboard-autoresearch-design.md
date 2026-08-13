# Dashboard route-family improvement design

## Subject and audience

AllSource is an event-store operations surface for developers. Each dashboard route must answer one operational question and expose one clear next action. Design stays within the existing midnight, neutral, and cyan system; this is product refinement, not a visual rebrand.

## Chosen approach

Treat the dashboard as one route family. Improve shared behavior first, then route-specific outliers. This beats cosmetic page-by-page edits because common shell, motion, loading, error, and navigation choices affect every route. It also avoids a full redesign that would obscure measurable performance changes and destabilize working flows.

Two rejected approaches:

- Independent visual redesigns per route: more novelty, less coherence, repeated fixes.
- All 89 web routes in one corpus: mixes marketing, docs, auth, and product jobs into a meaningless score.

## Visual system

- Palette: existing background, card, border, muted, primary cyan, destructive tokens.
- Type: system UI for interface text; monospace only for IDs, payloads, commands, and timestamps.
- Layout: route title and primary action first; operational state second; exploration controls third; data last.
- Motion: no page entrance choreography or counter animation. Retain motion only where it explains state change.
- Signature: write → inspect → query path, expressed through real event data and direct navigation.

## Autoresearch contract

Frozen corpus: 13 prerendered signed-in routes under `/dashboard`, built from one lockfile and measured by `tooling/route-weight`.

Primary scalar, lower is better: total raw bytes of unique JavaScript referenced by each route, summed across the route family. Per-route bytes remain visible so a shared win cannot hide one regressing page. Missing chunks invalidate a result.

Behavior gates:

1. Web and shared UI TypeScript checks pass.
2. Web and shared UI tests pass.
3. Production build passes.
4. Changed-file Biome checks pass.
5. Key routes remain keyboard-navigable and visually usable on desktop and mobile.
6. Empty, loading, error, and success states remain honest and actionable.

Stop after six scored experiments or three consecutive discards. Keep only measured wins that pass every gate. Correctness fixes may be kept without a byte win when recorded explicitly.
