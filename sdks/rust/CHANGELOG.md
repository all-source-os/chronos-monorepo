# Changelog

All notable changes to the `allsource` Rust SDK.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows [SemVer](https://semver.org).

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
