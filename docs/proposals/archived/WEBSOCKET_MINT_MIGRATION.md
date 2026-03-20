# Proposal: Migrate WebSocket Client from WebSockex to Mint.WebSocket

**Status**: Proposed
**Date**: 2026-03-01
**Author**: Claude Code
**Effort**: ~2-4 hours

## Problem

The Query Service WebSocket client (`CoreWebSocketClient`) cannot connect to Core on Fly.io because WebSockex doesn't support IPv6 properly. Fly.io `.internal` hostnames only resolve via AAAA (IPv6) records, and WebSockex's `socket_connection_options` calls `Keyword.merge` on socket options — which crashes when passed the `:inet6` atom (not a keyword pair). Neither `-kernel inet6 true` in ERL_AFLAGS nor manual DNS pre-resolution reliably fix this.

Currently WebSocket is disabled on Fly.io via `CORE_WS_ENABLED=false`, meaning real-time event streaming doesn't work in production.

## Solution

Replace WebSockex with `mint` + `mint_web_socket` for the WebSocket connection. Mint handles IPv6 natively via `Mint.HTTP.connect/4` transport options, and `mint_web_socket` is already a transitive dependency via Phoenix/LiveView.

## Current Architecture

```
CoreWebSocketClient (GenServer)
  - Manages lifecycle, retries, PubSub broadcasting
  - Monitors CoreWebSocketWorker

CoreWebSocketWorker (WebSockex process)
  - Holds raw WebSocket connection
  - Forwards parsed events to parent
  - 78 lines, 7 WebSockex callbacks
```

## Proposed Architecture

```
CoreWebSocketClient (GenServer) — minimal changes
  - Same lifecycle, retries, PubSub broadcasting
  - Monitors CoreWebSocketWorker

CoreWebSocketWorker (GenServer + Mint.WebSocket) — rewrite
  - Uses Mint.HTTP for TCP/TLS connection with :inet6
  - Uses Mint.WebSocket for upgrade handshake and framing
  - Message-based recv loop via handle_info
  - ~120 lines
```

## Files Changed

| File | Change | Effort |
|------|--------|--------|
| `mix.exs` | Add `mint_web_socket`, remove `websockex` | Trivial |
| `core_websocket_worker.ex` | Full rewrite from WebSockex to Mint.WebSocket GenServer | Medium (~120 lines) |
| `core_websocket_client.ex` | Update error pattern matching (remove WebSockex error structs) | Small |
| `fly.toml` | Remove `CORE_WS_ENABLED = "false"` | Trivial |
| `config/runtime.exs` | Keep `core_ws_enabled` support (useful for dev) | None |
| `core_websocket_client_test.exs` | Minor updates if any | Small |

## WebSockex APIs to Replace

| WebSockex API | Mint.WebSocket Equivalent |
|---------------|--------------------------|
| `WebSockex.start_link(url, mod, state, opts)` | `Mint.HTTP.connect(:http, host, port, opts)` + `Mint.WebSocket.upgrade(:ws, conn, path, headers)` |
| `handle_connect/2` callback | Detect `:mint_web_socket` upgrade response in `handle_info` |
| `handle_frame({:text, json}, state)` | `Mint.WebSocket.decode(ws, data)` returns `[{:text, json}]` |
| `handle_frame({:ping, _}, state)` | `Mint.WebSocket.encode(ws, :ping)` |
| `handle_disconnect/2` callback | Detect `{:tcp_closed, _}` or `{:ssl_closed, _}` in `handle_info` |
| `WebSockex.ConnError` | `Mint.TransportError` |
| `WebSockex.RequestError` | `Mint.WebSocket.UpgradeFailureError` |

## IPv6 Fix

The core fix is in `Mint.HTTP.connect/4`:

```elixir
Mint.HTTP.connect(:http, "allsource-core.internal", 3900,
  transport_opts: [inet6: true]
)
```

Mint passes transport options directly to `:gen_tcp.connect` as keyword pairs, which `:gen_tcp` handles correctly — no `Keyword.merge` on bare atoms.

## Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Mint.WebSocket** (chosen) | Already in dep tree, IPv6 native, ecosystem direction | Slightly more code than WebSockex |
| `:gun` | Native WS + IPv6, battle-tested | Erlang lib, heavier dep, different paradigm |
| Patch WebSockex | Minimal change | Unmaintained upstream, fork burden |
| Keep WS disabled | Zero effort | No real-time streaming in prod |

## Verification

1. `mix test` — all 888 tests pass
2. Deploy to Fly.io
3. Check health: `curl https://allsource-query.fly.dev/api/health` — websocket: "healthy"
4. Check logs: `fly logs --app allsource-query | grep CoreWebSocket` — "Connected to Core WebSocket"
