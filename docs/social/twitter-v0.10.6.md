---
status: published
---

# Twitter Thread — AllSource Chronos v0.10.6

## 1/9

Most startups depend on npm, PyPI, and crates.io to ship their SDKs.

We built our own package registry. One Rust binary. Four protocols. Zero vendor lock-in.

AllSource Chronos v0.10.6 is out 🧵

`#opensource #rust #eventsourcing`

---

## 2/9

We ship SDKs in 4 languages. That means 4 package managers, 4 auth systems, 4 points of failure.

So we built @allsource/registry — a single Rust binary that speaks:

→ Cargo (Rust)
→ npm (TypeScript)
→ PyPI (Python)
→ Go module proxy

Runs on Fly.io. 31-line Dockerfile. ~15 MB image.

`#rustlang #flyio #devtools`

---

## 3/9

Hot take: if your event store doesn't ship an SDK, it's not a database — it's a protocol spec with a README.

Event sourcing is hard. Not the storage — the client code. Append is easy. But folding events into state, handling projections, retrying on transient failures, circuit-breaking when the server is down?

Every team rebuilds this. In every language. Badly.

Postgres doesn't need an SDK because SELECT is universal. Event stores don't have that luxury. Your wire format is your API. Your fold semantics are your query language.

If you're not shipping a client that handles this, you're shipping homework.

`#eventsourcing #databases #dx`

---

## 4/9

Our Rust SDK had features the others didn't. That's fixed now.

Go SDK gains:
→ Circuit breaker with configurable thresholds
→ Client-side fold/projection support
→ Full integration test suite
→ Version tracking

TypeScript SDK gains:
→ Circuit breaker (same pattern)
→ Fold support
→ Integration tests against live Core

Same capabilities. Every language.

`#golang #typescript #sdk`

---

## 5/9

OAuth lived in the Query Service — an Elixir middleman that just forwarded everything to the Control Plane anyway.

We deleted 397 lines of Elixir and replaced them with Go handlers that:

→ Own the full GitHub + Google OAuth flow
→ Sign JWTs directly (no proxy hop)
→ Validate CSRF state via httpOnly cookies
→ Fail cleanly when Core is unreachable

Auth belongs in the auth service. Not the API gateway.

`#golang #oauth #security`

---

## 6/9

While moving OAuth, we did a principal-engineer-level security audit:

→ CSRF state parameter with cookie-based verification
→ SameSite=Lax explicitly set (not browser-default)
→ Secure flag derived from FRONTEND_URL scheme (works behind TLS proxies)
→ No more silent degradation — Core down = clean 503, not a broken session
→ Client secrets never logged, even on error

The boring stuff that prevents the headlines.

`#appsec #infosec #webdev`

---

## 7/9

Deleted across this release:

→ OAuth controller (397 lines of Elixir)
→ Dead QS config (OAuth env vars, control_plane_url)
→ Unused cluster health functions
→ 44 lines of dead test code
→ Legacy middleware convention (Next.js 16 proxy migration)

Every line of dead code is a line someone will try to understand later. Don't make them.

`#cleancode #refactoring #elixir`

---

## 8/9

AllSource Chronos v0.10.6:

→ 5,123 lines added, 1,101 removed
→ 77 files changed
→ 4 SDKs at feature parity
→ 1 self-hosted registry serving 4 ecosystems
→ 79 MB total Docker footprint (Core + CP + QS)
→ 469K events/sec write throughput (unchanged)

Small images. Big capabilities.

`#database #performance #docker`

---

## 9/9

We're building an AI-native event store in the open. Rust core, Elixir API gateway, Go control plane.

Every event is durable. Every query is fast. Every SDK is first-class.

Star us if this is the kind of infra you want to exist:

github.com/all-source-os/chronos-monorepo

`#opensource #eventdriven #eventsourcing #rust #elixir #golang #database`
