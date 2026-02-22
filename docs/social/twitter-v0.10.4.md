---
status: published
---

# Twitter Thread — AllSource Chronos v0.10.4

## Tweet 1/7 — Hook

AllSource Chronos v0.10.4 is out.

We found a critical persistence bug that affected every published image since launch — and shipped a massive update to fix it.

Here's what happened and what's new 🧵

`#opensource #eventstore #rust`

📷 *Image: Chronos logo or repo README header with badges*

---

## Tweet 2/7 — The Bug

The bug: our Rust Core was initializing EventStore with default config — all-None fields.

Env vars like ALLSOURCE_WAL_DIR and ALLSOURCE_STORAGE_DIR were set in Docker Compose... but main.rs never read them.

Every published image was running in-memory only. Events lost on restart.

`#rustlang #debugging`

📷 *Image: code diff of `main.rs` showing the old `EventStore::new()` vs new `EventStoreConfig::from_env()` call*

---

## Tweet 3/7 — The Fix

The fix: `EventStoreConfig::from_env()` now reads ALLSOURCE_DATA_DIR / WAL_DIR / STORAGE_DIR and constructs the full WAL + Parquet pipeline.

7 unit tests. A new durability test script that writes events, kills the container, restarts, and verifies everything survived.

We'll never ship this class of bug again.

📷 *Image: terminal output of durability test passing — green checkmarks*

---

## Tweet 4/7 — Dashboard

New dashboard pages:
→ Analytics with usage charts
→ Team management + invite flow
→ Audit log viewer
→ Event replay tool
→ Changelog + status pages

Full operational visibility in the browser.

`#nextjs #react #dashboard`

📷 *Image: screenshot of the analytics or team management dashboard page*

---

## Tweet 5/7 — Query Service

Query Service got a major expansion:

• Audit log controller
• Team management controller
• Event replay controller
• Usage analytics controller
• Expanded Core client proxying

All wired through the Elixir API gateway with proper auth.

`#elixir #phoenix #api`

📷 *Image: screenshot of the new routes in router.ex or the controller list*

---

## Tweet 6/7 — SDKs

Two new SDKs shipped:

🔹 Go SDK — full client with test suite
🔹 Python SDK — sync + async clients with tests

Plus the existing Rust and TypeScript SDKs. Four languages, one event store.

`#golang #python #sdk #devtools`

📷 *Image: side-by-side code snippets showing Go and Python SDK usage*

---

## Tweet 7/7 — CTA

v0.10.4 is the first release where persistence actually works in Docker. If you tried Chronos before and lost data — that's fixed.

⭐ github.com/all-source-os/chronos-monorepo

`#opensource #eventdriven #eventsourcing #rust #database`

📷 *Image: repo star count / GitHub social preview card*
