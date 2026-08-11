# Changelog

All notable changes to the `allsource` Rust SDK.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows [SemVer](https://semver.org).

## [Unreleased]

### Added

- `CoreClient::get_projection_state_summary` (`GET /api/v1/projections/:name/state`),
  `CoreClient::bulk_get_projection_states` (`POST /api/v1/projections/:name/bulk`)
  and `CoreClient::poll_consumer_events` (`GET /api/v1/consumers/:id/events`).
  These three Core routes had no SDK method, and `CoreClient::transport()` is
  `pub(crate)`, so reaching them meant standing up a second HTTP client without
  the SDK's retry loop and circuit breaker. Polling returns the new
  `ConsumerEvent` type (WAL `position` + `event`) to ack with `ack_consumer`.
  Resolves issue #246.
- `Error::Protocol` — the server answered 2xx with a body that breaks the API
  contract (currently: it ignored a pagination parameter). Distinct from
  `Error::Json` (the body did not parse) and `Error::Api` (the server said no).

### Fixed

- Docs: `CoreClient::get_projection_state` and `ProjectionHandle::get_state` no
  longer claim Core's `GET /api/v1/projections/:name/:entity_id/state` requires
  a registered projection. Core ≥ 0.19.1 resolves the registered projection
  first and falls back to the projection state cache, so state pushed with
  `put_projection_state` / `bulk_put_projection_state` reads back through the
  same endpoint — the "one worker computes, many stateless readers read" shape.
  The stale caveat predates the Core v0.19.1 fallback. Resolves issue #247.
- `EventPaginator` and `EntityPaginator` no longer loop forever against a server
  that ignores `offset`. Core's `/api/v1/events/query` dropped the parameter
  until the fix for issue #250 — every page came back as page one with
  `has_more: true`, so `collect_all()` spun and grew until it ran out of memory.
  Both paginators now detect a page that repeats the previous page's first item
  and return the new `Error::Protocol` naming the dropped parameter, instead of
  spinning. A released SDK still meets older Core deployments, so the client
  needs its own guard.

## [0.21.0] — 2026-05-16

### Added

- `QueryEventsParams::order` plus the `SortOrder` enum (`Asc`/`Desc`) and the
  `order_desc()` / `order_asc()` builder shorthands. This sends `order=desc` to
  Core's `/api/v1/events/query`, so clients can fetch newest-first (e.g. the
  latest event for an entity with `.order_desc().limit(1)`) without folding the
  whole stream in memory. Resolves issue #178.
- `EventPaginator` / `EntityPaginator` and the `QueryClient::query_events_paged`
  / `list_entities_paged` constructors. They drive `limit`/`offset` paging:
  `next_page()` fetches the next batch until the server reports no more,
  `collect_all()` drains everything. `has_more` is used when present, with
  short-page fallback for older Core. `DEFAULT_PAGE_SIZE` (100) is the per-page
  size when params set no `limit`.
- `list_entities` now accepts an `order` parameter (`asc`/`desc`) via the new
  `ListEntitiesParams::order`. Core's entity-listing endpoint sorts by
  last-event time with an `entity_id` tie-break, making the order total and
  offset pagination stable.

### Changed

- **Breaking:** `QueryClient::list_entities` now takes a single
  `ListEntitiesParams` (builder: `event_type_prefix` / `limit` / `offset` /
  `order`) instead of three positional `Option` arguments. Migration:
  `list_entities(Some("p."), Some(50), None)` →
  `list_entities(ListEntitiesParams::new().event_type_prefix("p.").limit(50))`.
- `QueryEventsParams` / `QueryEventsResponse` / `query_events` rustdoc now state
  the result ordering guarantee: events are ordered by `(timestamp, version)`,
  ascending by default, so `limit`/`offset` pagination over a sorted view is
  well-defined.

## [0.19.2] — 2026-04-17

### Changed

- Documentation-focused release. No public API changes in the SDK.
- README rewritten with a "Which client do I want?" decision table, "Performance cheatsheet", and "API keys" section so consumers don't need a separate perf guide.
- Crate-level rustdoc updated to mirror the same guidance (shows in rust-analyzer hover and docs.rs).
- Per-type rustdoc expanded on `CoreClient`, `QueryClient`, and `ProjectionWorker` explaining when each is the right default.

### Fixed (monorepo, no SDK-code impact)

- Core `Dockerfile` now stubs `sdks/rust/examples/asset_projection.rs` so cargo-chef parses the workspace correctly during image build. v0.19.1's Docker tag build failed on this; v0.19.2's ships.
- Monorepo `Makefile` `set-version` regex handles 3-segment versions in `apps/prime-mcp/Cargo.toml`'s `allsource-core` dep constraint (was silently skipping on 3-segment patterns).

[0.19.2]: https://github.com/all-source-os/all-source/releases/tag/v0.19.2

## [0.19.1] — 2026-04-17

### Changed

- Version bump as part of monorepo-wide v0.19.1 release. No SDK source changes. Pairs with Core v0.19.1, which adds cache-fallback to `GET /api/v1/projections/:name/:entity_id/state` so `ProjectionHandle::get_state` now works for SDK-managed projections (no registered projection required). The caveat previously documented in the README / use-case guide is resolved against any Core ≥ 0.19.1.

[0.19.1]: https://github.com/all-source-os/all-source/releases/tag/v0.19.1

## [0.19.0] — 2026-04-17

### Added

- **`ProjectionWorker`** — first-party worker for building custom projections from Core's event stream ([#155]). Handles WebSocket subscription, durable-consumer registration, replay → live transition, per-entity version dedup, checkpointing, and exponential-backoff reconnection. Users provide only the reducer closure.
  - Builder API: `ProjectionWorker::<S>::builder(core).name(...).event_types(...).reducer(...).checkpoint_interval(...).build()`
  - Optional state push-back: `.state_flush_entities(...).state_flush_every(...).state_flush_interval(...)`
  - Lifecycle: `worker.start().await?` → `ProjectionHandle<S>` with `state()`, `get_state(id)`, `is_caught_up()`, `current_position()`, `stop().await`
- **`ws` feature** — WebSocket client (`EventStreamClient`, `EventStream`, `StreamItem`) for Core's `/api/v1/events/stream`. Parses replay/replay_complete/live/lagged frames into a typed stream.
- **`projection-worker` feature (default-on)** — pulls in `ws` + async-trait for the worker. Disable with `default-features = false` for HTTP-only builds.
- **`CoreClient` projection + consumer helpers**: `get_projection_state`, `put_projection_state`, `bulk_put_projection_state`, `register_consumer`, `get_consumer`, `ack_consumer`, `save_checkpoint`, `load_checkpoint`.
- **`ConsumerState` type** for durable-consumer responses.
- Integration tests against a live Core (skip gracefully without `ALLSOURCE_TEST_CORE_URL`): cold start, restart resume, version dedup, reducer-error propagation.
- Runnable example: `examples/asset_projection.rs` (mirrors the API shape requested in #155).
- Documentation: README "Building custom projections" section + full guide in `docs/use-cases/custom-projections.md`.

### Changed

- README rewritten: the old `Client::new` snippet referenced a type that hasn't existed for several releases; it's now `QueryClient` + `CoreClient`.
- `Error::WebSocket(String)` variant added (behind `ws` feature).

### Internal

- `HttpTransport` exposes `pub(crate)` methods for `put` and `get_optional` (404 → None). No user-facing change.

[#155]: https://github.com/all-source-os/all-source/issues/155
[0.19.0]: https://github.com/all-source-os/all-source/releases/tag/sdk-rust-v0.19.0
