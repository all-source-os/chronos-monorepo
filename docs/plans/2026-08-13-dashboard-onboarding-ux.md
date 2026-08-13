# Dashboard and onboarding UX repair

## Product job

Move signed-in users from an empty tenant to one verified event, then make the dashboard answer:

1. Is data flowing?
2. What happened recently?
3. What should I do next?

## Direction

- Keep existing navy, neutral, and cyan design tokens.
- Use a static, compact application shell. Avoid entrance choreography and animated counters.
- Put actions and setup state before quota, charts, and account administration.
- Make a real event's path — create, see, query — the signature interaction.
- Preserve dashboard routes and Query Service boundaries.

## Repairs

- Render a cached authenticated shell while session verification runs; redirect only for a rejected session.
- Show an honest degraded-session state for network and service failures.
- Replace the inert Create Event controls with a validated dialog backed by the real event API.
- Route both onboarding entry points through the tested SDK wizard and real ingest/query calls.
- Replace hosted onboarding's local URLs, vague claims, emoji, and generic celebration copy.
- Close mobile navigation after route changes; remove controls that claim unsupported live state.
- Replace the render-blocking external font stylesheet with a system font stack.
- Add focused tests for event creation, onboarding route behavior, and session failure handling.

## Acceptance checks

- `/dashboard/events?action=create` opens a usable dialog.
- Valid JSON creates a real event, refreshes event data, and gives a direct link to the created stream.
- Invalid JSON and API failures produce useful inline errors without losing form data.
- `/onboarding` stays on its canonical route and executes real `/api/v1/events` ingest/query calls.
- Dashboard shell does not blank when cached auth exists or a session refresh returns 5xx.
- Mobile menu closes after navigation; status UI links to real status instead of asserting connectivity.
- Web type-check, tests, production build, changed-file lint, and relevant Rust checks pass.
- Desktop and mobile proof screenshots show no clipping, overlap, or illegible controls.
