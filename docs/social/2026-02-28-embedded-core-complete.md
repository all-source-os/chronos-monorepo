# Twitter Thread: Embedded Core API Complete

**Date**: 2026-02-28
**Topic**: Embedded Core Phase 8 TOON format, full API completion

---

**1/**
We just completed the Embedded Core API for AllSource — all 8 phases, 83 tests passing.

You can now use our event store as an in-process Rust library. No server, no network calls, no external dependencies.

`EmbeddedCore::open(config).await` — that's it.

#RustLang #EventSourcing #OpenSource #CQRS

---

**2/**
Phase 8 adds TOON format output — Token-Oriented Object Notation.

`core.query_toon(query)` returns events encoded with ~50% fewer tokens than JSON. No quoted keys, no redundant braces.

Built for LLMs where every token costs money.

#AI #LLM #DevTools #RustLang

---

**3/**
The embedded API ships as Cargo feature flags, so you only pay for what you use:

- `embedded` — core library API
- `embedded-toon` — TOON-encoded queries
- `embedded-streaming` — real-time event streams
- `embedded-projections` — materialized views

Zero bloat in your binary.

#RustLang #CargoFeatures #ZeroCost

---

**4/**
Also shipped in this batch:

- State machine guards on workflow projections — no more invalid status transitions
- Token compaction now inherits tenant context from the events themselves
- Deterministic task queue ordering for reproducible test runs

Small fixes that matter at scale.

#SoftwareEngineering #EventDriven #Reliability

---

**5/**
On the frontend side:

WebSocket URLs now auto-derive from your API URL. No more hardcoded `ws://localhost:3902` that silently fails in production.

`https://api.example.com` -> `wss://api.example.com`

One fewer env var to forget.

#WebDev #DX #WebSockets #NextJS

---

**6/**
Demo accounts now ship with seeded audit logs — 6 realistic entries covering API key creation, team invites, plan changes.

New users see a working Audit Log page on first login instead of empty state.

#UX #DeveloperExperience #SaaS #Onboarding

---

**7/**
The full embedded Core API covers:

- Event ingestion + batch + queries
- Schema validation
- Projections + snapshots
- Replicant worker protocol
- Streaming tokens + compaction
- AI workflow templates
- TOON format output

All in-process, all tested, all behind feature flags.

#RustLang #EventSourcing #EmbeddedDatabase

---

**8/**
AllSource Core is a purpose-built Rust event store.

469K events/sec. 11.9us queries. WAL + Parquet durability. Now embeddable as a library.

github.com/all-source-os/all-source

#RustLang #OpenSource #EventSourcing #Database #CQRS #DevTools #BuildInPublic
