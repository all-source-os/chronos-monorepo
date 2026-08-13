# Projection workspace redesign

## Problem

The empty projections page presents three zero-value metric cards, two competing
primary actions, and a large empty panel. It explains neither what a projection
does nor which real read models AllSource can build.

## Decision

Use a state-aware workspace:

- Empty state: show the live projection catalog as the primary surface. Every
  template is a named, keyboard-accessible action with its real description and
  output kind.
- Active state: replace oversized metric cards with one compact status strip,
  then show enabled read models and their inspect/disable actions.
- Keep one `Add read model` action after the first projection is enabled. Use an
  accessible modal catalog rather than a detached dropdown.
- Explain the actual path—event history, template fold, queryable state—and link
  existing projections to Replay Studio for rebuilds.
- Remove the dashboard-stats request from this page. It existed only to show a
  zero-event warning and caused unrelated requests before the page became useful.

## Visual direction

Keep the established dark AllSource shell and cyan accent. Use compact borders,
quiet surfaces, and data-shape icons. Avoid gradients and decorative dashboard
widgets. Projection catalog is the page's visual signature because it maps
directly to product capability.

## Acceptance criteria

- Empty tenants can understand and enable a real template above the fold.
- No duplicate primary action or zero-value summary cards in empty state.
- Active tenants see enabled, ready, and building counts without large dead space.
- Template and projection actions have accessible names, focus styles, and dialog
  semantics.
- Load, mutation-error, empty, building, ready, and all-templates-enabled states
  remain explicit.
