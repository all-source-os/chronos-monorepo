# AllSource brand guide

## Brand idea

AllSource turns system history into evidence. Every visual and written choice
should reinforce ordered events, durable state, and answers traceable to source.

Use **AllSource Event Store** as the full product name and **AllSource** in
compact UI. Product layers remain Core, Prime, Hosted AllSource, and MCP
connectors. Solutions describe workloads, not additional products.

## Color

Palette comes from product logo plus approved blue field. Use semantic tokens in
application code; hex values belong here, logo assets, generated images, and
token definitions only.

| Token | Value | Use |
| --- | --- | --- |
| Brand field | `#07549A` | Default marketing field and campaign backgrounds |
| Core blue | `#0277BD` | Light-theme actions and links |
| Signal blue | `#29B6F6` | Diagrams, traces, and large decorative signals |
| Ice blue | `#81D4FA` | Dark-theme actions, links, focus, and small signal text |
| Source ink | `#122033` | Code surfaces, dark text, and primary-button text |
| Paper | `#F7FBFF` | Light-theme field, forms, and readable technical documents |

Rules:

- Blue owns branded surfaces. Do not use near-black as a full-page marketing
  background.
- Source ink may appear behind code, terminals, and technical diagrams.
- Green, amber, and red communicate success, warning, and failure only.
- Do not use purple, orange, or rainbow gradients as decoration.
- Body text and controls must meet WCAG 2.2 AA. Ice blue on brand field is
  4.64:1; paper on brand field is 7.36:1.

## Typography

- Use existing system sans stack for headings, body copy, navigation, and
  controls. It stays fast and matches product UI.
- Use existing monospace stack only for event names, commands, identifiers,
  timestamps, system roles, and verification state.
- Headlines use sentence case, semibold weight, tight tracking, and readable
  line breaks. Avoid oversized one-word lines.
- Utility labels may use uppercase monospace only when value describes real
  system state.

## Logo

- Use supplied AllSource logo without recoloring, cropping, rotation, or new
  effects.
- Keep clear space equal to half logo width on all sides.
- Minimum digital size: 32px. Pair with “AllSource” and optional “Event Store”
  modifier in navigation or product identity contexts.

## Layout and components

- Use centered content, 16px mobile gutters, 24px tablet gutters, and 32px
  desktop gutters. Keep reading copy below 75 characters per line.
- Prefer rules, ordered traces, and explicit state changes over ornamental
  cards. Border radius follows existing component tokens.
- One page, one primary action. Secondary actions use outline or text treatment.
- Use branded blue for page fields and primary actions. Use paper or blue cards
  to create hierarchy; avoid detached floating glass panels.
- Black terminal panels remain valid when content is executable or diagnostic.

## Signature

Ordered provenance rail is the reusable brand device:

```text
event.recorded → state.derived → answer.recalled
```

Use it only when explaining real data flow, history, replay, or recall. Never
add it as empty decoration.

## Voice

- Lead with concrete user job and shipped capability.
- Prefer exact nouns, active verbs, limits, and evidence.
- Avoid “revolutionary,” “seamless,” “unlock,” “supercharge,” and generic AI
  claims.
- Keep Core, Query Service, Prime, hosted service, and MCP boundaries accurate.

## Social identity

Canonical public profiles are GitHub, X, and GitHub Discussions. Do not publish
or include an Instagram identity.
