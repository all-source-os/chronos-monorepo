# better-auth-allsource

[![Crates.io](https://img.shields.io/crates/v/better-auth-allsource.svg)](https://crates.io/crates/better-auth-allsource)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Event-sourced authentication persistence for [better-auth-rs](https://crates.io/crates/better-auth-core), powered by [Allsource](https://github.com/all-source-os/all-source).

Instead of storing auth state in a traditional database, every create, update, and delete becomes an immutable event in Allsource's event store. You get full audit history, time-travel debugging, and crash-safe durability — all with sub-millisecond read performance.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
better-auth-allsource = "0.14"
better-auth-core = "0.8"
```

Wire it up:

```rust
use better_auth_allsource::AllsourceAuthAdapter;
use better_auth_core::BetterAuth;

let adapter = AllsourceAuthAdapter::new(
    "http://localhost:3900",  // Allsource Core (event store)
    "http://localhost:3902",  // Allsource Query Service
    "ask_your_api_key",      // API key
);

let auth = BetterAuth::builder()
    .database(adapter)
    .secret("your-secret-key")
    .build()
    .await?;
```

That's it. All 10 `DatabaseAdapter` sub-traits are implemented — users, sessions, accounts, organizations, members, invitations, verification tokens, two-factor auth, API keys, and passkeys all work out of the box.

## How It Works

Each auth entity maps to an Allsource event stream:

| Entity | Entity ID Pattern | Event Types |
|---|---|---|
| User | `auth-user:{id}` | `UserCreated`, `UserUpdated`, `UserDeleted` |
| Session | `auth-session:{token}` | `SessionCreated`, `SessionDeleted` |
| Account | `auth-account:{id}` | `AccountCreated`, `AccountUpdated`, `AccountDeleted` |
| Verification | `auth-verification:{id}` | `VerificationCreated`, `VerificationDeleted` |
| Organization | `auth-org:{id}` | `OrgCreated`, `OrgUpdated`, `OrgDeleted` |
| Member | `auth-member:{id}` | `MemberCreated`, `MemberUpdated`, `MemberDeleted` |
| Invitation | `auth-invitation:{id}` | `InvitationCreated`, `InvitationUpdated`, `InvitationDeleted` |
| Two-Factor | `auth-2fa:{id}` | `TwoFactorCreated`, `TwoFactorUpdated`, `TwoFactorDeleted` |
| API Key | `auth-apikey:{id}` | `ApiKeyCreated`, `ApiKeyUpdated`, `ApiKeyDeleted` |
| Passkey | `auth-passkey:{id}` | `PasskeyCreated`, `PasskeyUpdated`, `PasskeyDeleted` |

**Event pattern:**
- **Create/Update** — appends a full-state snapshot as the event payload
- **Delete** — appends a `{ "_deleted": true }` marker event
- **Read** — fetches the latest event for the entity and deserializes the payload

This means every auth state change is preserved as an immutable event. You can reconstruct any entity's full history by querying its event stream.

## Features

- **Full `DatabaseAdapter` coverage** — all 10 sub-traits (`UserOps`, `SessionOps`, `AccountOps`, `VerificationOps`, `OrganizationOps`, `MemberOps`, `InvitationOps`, `TwoFactorOps`, `ApiKeyOps`, `PasskeyOps`)
- **Event-sourced audit trail** — every auth mutation is an immutable event with timestamp
- **Durable storage** — Allsource Core persists events via WAL + Parquet with CRC32 checksums
- **Fast reads** — latest-state queries hit Allsource's in-memory DashMap (11.9us p99)
- **Soft deletes** — deletion markers preserve history; entities can be restored
- **Field-based search** — find users by email, accounts by provider, sessions by user ID, etc.
- **TLS support** — `rustls` (default) or `native-tls` feature flags

## TLS Configuration

```toml
# Default: rustls
better-auth-allsource = "0.14"

# Or use native-tls
better-auth-allsource = { version = "0.14", default-features = false, features = ["native-tls"] }
```

## Architecture

```
┌─────────────────────────────────────────────┐
│          your application                   │
│                                             │
│  BetterAuth ──► AllsourceAuthAdapter        │
│                    │                        │
│                    ├── writes ──► Core :3900 │
│                    │             (WAL+Parquet)
│                    └── reads ──► QS   :3902 │
│                                  (queries)  │
└─────────────────────────────────────────────┘
```

Writes go directly to Allsource Core (the event store). Reads go through the Query Service, which handles auth validation and query routing. Both are part of the [Allsource](https://github.com/all-source-os/all-source) platform.

## Error Handling

All methods return `AuthResult<T>` from `better-auth-core`. The adapter maps HTTP, JSON, and API errors into `AuthError::Database(DatabaseError::Query(...))`:

```rust
use better_auth_allsource::AllsourceAuthError;

// The error type exposes three variants:
// AllsourceAuthError::Http(_)  — network/connection errors
// AllsourceAuthError::Json(_)  — serialization errors
// AllsourceAuthError::Api { status, message } — non-2xx responses from Allsource
```

## Requirements

- [Allsource Core](https://github.com/all-source-os/all-source) running (event store)
- [Allsource Query Service](https://github.com/all-source-os/all-source) running (API gateway)
- A valid API key

## License

MIT
