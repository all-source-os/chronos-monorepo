# Ledger — Replay Studio

Date: 2026-08-13. Starting commit: `094edaa`.

## Objective and scorecard

Goal: make replay a credible AllSource highlight: safe reconstruction of tenant read-models from immutable history.

Scalar performance metric: raw JavaScript bytes referenced by prerendered `/dashboard/tools/replay`, measured with `tooling/route-weight`. Lower is better. Correctness gates override small byte regressions.

Product gates:

- page renders every backend status;
- operations are tenant-scoped;
- failed/cancelled runs preserve current state;
- successful replay replaces, never appends;
- target and effect are visible before start.

## Baseline

| Metric | Baseline |
|---|---:|
| Route JavaScript | 729,842 B |
| Chunks | 13 |
| Live render | Crash: `Cannot read properties of undefined (reading 'icon')` |
| Tenant isolation | Failed: Core query used `tenant_id: None` |
| Rebuild semantics | Failed: events appended into current projection |
| Target validation | Failed: free text or every global projection |

## Iterations

| # | Proposal | Verdict |
|---|---|---|
| 1 | Normalize Core's lowercase statuses and add unknown fallback | Keep as crash barrier, but insufficient alone. |
| 2 | Add tenant ID to existing Core proxy | Reject. Core projections are global; proxy still exposes unrelated run history and cannot safely own tenant read-model state. |
| 3 | Move dashboard replay onto Query Service's tenant-owned projection engine | Keep. Existing authentication, curated reducers, and tenant event query become authority. |
| 4 | Fold into shadow generation, buffer post-cutoff live events, publish by pointer swap | Keep. Success replaces state; failure/cancellation leaves current generation intact. |
| 5 | Replace filter-heavy form with event-history → target → publish plan | Keep. One enabled target, explicit effect, visible safety contract, usable empty/error states. |

## Result

| Metric | Result | Delta |
|---|---:|---:|
| Route JavaScript | 733,670 B | +3,828 B (+0.52%) |
| Chunks | 13 | 0 |
| Status crash cases | 0 | fixed |
| Tenant isolation tests | passing | added |
| Replacement/failure invariants | passing | added |

Small byte increase accepted: page gains enabled-target discovery and concrete safety/progress context without adding a chunk.

## Verification

- Query Service suite: 1,005 tests passed, 2 skipped.
- Web suite: 74 tests passed; replay unit cases cover lowercase, title-case, unknown status, and plain-string API errors.
- TypeScript check passed.
- Next.js production build passed; route prerendered.
- Rust replay service tests: 2 passed.
- Browser proof required after deployment because production authentication and service routing determine final end-to-end replay behavior.
