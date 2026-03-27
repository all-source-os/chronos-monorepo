# PRD: Agent Auth & x402 Payments

## Overview

Enable AI agents to self-register, authenticate, and pay for AllSource API usage via the x402 protocol. All new functionality lives in the **Control Plane** (Go) — the existing billing and tenant authority. No Query Service changes.

**Problem:** Agents need 4 steps across 3 services to onboard. There's no programmatic payment mechanism for pay-per-use beyond subscriptions.

**Solution:** Single registration endpoint + self-hosted x402 facilitator in the Control Plane, supporting both Base (EVM) and Solana stablecoins.

## Goals

- One-call agent registration: name in, tenant + API key out
- Self-hosted x402 facilitator verifying and settling payments on-chain
- Pay-per-use pricing for agents without subscriptions (BYOW — bring your own wallet)
- x402 payment events written to Core for audit trail and time-travel billing
- MCP transport support (`_meta["x402/payment"]`) for MCP-connected agents
- Both EVM (Base/USDC) and Solana (SPL/USDC) payment rails

## Quality Gates

### Epic-Level (run once on epic completion)
- `make quality-go` passes (Control Plane — all new code lives here)
- `make quality-elixir-full` passes (verify no QS regressions)

### Story-Level (checked per story)
- **Backend stories:** Verify endpoint returns expected response via curl/test
- **Integration stories:** Verify end-to-end flow with test wallet

## User Stories

### US-001: Agent Registration Endpoint [Backend]
**Description:** As an AI agent, I want to register with AllSource in a single API call so I can start ingesting events immediately.

**Acceptance Criteria:**
- [ ] `POST /api/v1/agents/register` endpoint added to Control Plane router
- [ ] Accepts `{"agent_name": "...", "agent_type": "mcp"|"sdk"|"cli"}` body
- [ ] Creates tenant with `agent_type` metadata and free-tier quotas
- [ ] Creates API key scoped to tenant (format: `ask_...`)
- [ ] Returns `{tenant_id, api_key, tier, quotas, core_url, query_url}` with 201
- [ ] Rejects duplicate `agent_name` per IP with 409 (prevent spam)
- [ ] Rate limited: 10 registrations per IP per hour
- [ ] Unit test covers happy path, duplicate, and rate limit
- [ ] `curl POST /api/v1/agents/register` returns 201 with valid API key

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Agent Registration Use Case & Repository [Backend]
**Description:** As a developer, I need the domain logic for agent registration separated from the HTTP handler.

**Acceptance Criteria:**
- [ ] `RegisterAgentUseCase` created in `internal/application/usecases/`
- [ ] Calls `CreateTenantUseCase` with agent metadata
- [ ] Calls Core's `POST /api/v1/auth/api-keys` to create API key for tenant
- [ ] Writes `agent.registered` event to Core via CoreClient
- [ ] Returns structured response with all fields needed by handler
- [ ] Unit test with in-memory repos verifies tenant + key creation + event write
- [ ] Error cases: Core unreachable, tenant creation fails

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: x402 Payment Types & Scheme Support [Backend]
**Description:** As a developer, I need the core x402 types and payment scheme implementations for EVM and Solana.

**Acceptance Criteria:**
- [ ] `internal/infrastructure/x402/` package created
- [ ] `types.go`: `PaymentRequired`, `PaymentPayload`, `SettlementResponse` structs matching x402 v2 spec
- [ ] `evm.go`: EIP-3009 `transferWithAuthorization` signature verification for Base/USDC
- [ ] `svm.go`: SPL `TransferChecked` signature verification for Solana/USDC
- [ ] Network identifiers use CAIP-2 format (`eip155:8453`, `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp`)
- [ ] `scheme_exact.go`: exact-amount payment scheme (fixed price per call)
- [ ] Unit tests for signature verification (use known test vectors from Coinbase x402 repo)
- [ ] Unit tests for base64 JSON encode/decode of payment headers

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Self-Hosted Facilitator Service [Backend]
**Description:** As the platform operator, I want a self-hosted x402 facilitator so payments are verified and settled without depending on Coinbase's hosted service.

**Acceptance Criteria:**
- [ ] `internal/infrastructure/x402/facilitator.go` implements facilitator logic
- [ ] `POST /x402/verify` endpoint: validates payment signature, checks amount/recipient/nonce
- [ ] `POST /x402/settle` endpoint: submits on-chain transaction (EVM via ethclient, Solana via RPC)
- [ ] Nonce tracking prevents replay attacks (store in memory with TTL, persist to Core events)
- [ ] Settlement timeout: 30 seconds, returns error if tx not confirmed
- [ ] EVM settlement: calls `transferWithAuthorization` on USDC contract
- [ ] Solana settlement: submits SPL transfer instruction
- [ ] Environment config: `X402_EVM_RPC_URL`, `X402_SOLANA_RPC_URL`, `X402_SETTLEMENT_WALLET_KEY`
- [ ] Unit tests for verify (valid sig, invalid sig, expired, wrong amount, replay)
- [ ] Integration test with Base Sepolia testnet (EVM)

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: x402 Payment Middleware [Backend]
**Description:** As the Control Plane, I want to gate API endpoints with x402 payment requirements when agents exceed their free-tier quota.

**Acceptance Criteria:**
- [ ] `internal/infrastructure/x402/middleware.go` implements Go HTTP middleware
- [ ] Middleware checks `PAYMENT-SIGNATURE` header on configured routes
- [ ] If absent: returns HTTP 402 with `PAYMENT-REQUIRED` header (base64 JSON)
- [ ] If present: calls facilitator `/verify`, then `/settle` on success
- [ ] Returns `PAYMENT-RESPONSE` header with tx hash on success
- [ ] Route pricing config: map of `method+path` → `{amount, asset, network, recipient}`
- [ ] Middleware is opt-in per route (not applied globally)
- [ ] Supports both EVM and Solana payment signatures
- [ ] Unit test: request without payment → 402
- [ ] Unit test: request with valid payment → passes through
- [ ] Unit test: request with invalid payment → 400

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: x402 Pricing Configuration [Backend]
**Description:** As the platform operator, I want to configure per-endpoint pricing via environment variables or config file.

**Acceptance Criteria:**
- [ ] `X402_PRICING_CONFIG` env var points to JSON/YAML pricing file
- [ ] Pricing file maps routes to price + accepted payment methods
- [ ] Amounts in smallest unit (USDC has 6 decimals, so 100 = $0.0001)
- [ ] `X402_RECIPIENT_ADDRESS` env var for payment recipient wallet
- [ ] `X402_ENABLED` env var (default: false) — master switch
- [ ] Pricing config loaded at startup, logged (amounts only, no keys)
- [ ] Unit test: config parsing, missing file, invalid JSON

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: x402 Payment Event Logging [Backend]
**Description:** As the platform, I want every x402 payment interaction logged as Core events for audit trail and time-travel billing.

**Acceptance Criteria:**
- [ ] Payment events written to Core via existing CoreClient.IngestEvent:
  - `x402.payment.requested` — agent got 402 response (entity: tenant_id)
  - `x402.payment.verified` — signature verified by facilitator
  - `x402.payment.settled` — on-chain tx confirmed (includes tx_hash)
  - `x402.payment.failed` — verification or settlement failed (includes reason)
- [ ] Event payloads include: amount, asset, network, tx_hash, agent_name, endpoint, timestamp
- [ ] Events are non-blocking (errors logged, don't fail the payment flow)
- [ ] Unit test: mock CoreClient captures correct event types and payloads

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Quota-Gated x402 Flow [Backend]
**Description:** As a free-tier agent, I want to use AllSource for free within my quota and only pay via x402 when I exceed it.

**Acceptance Criteria:**
- [ ] Control Plane checks agent's remaining quota before applying x402 middleware
- [ ] Quota remaining → request proxied to Core/QS normally (no payment)
- [ ] Quota exceeded + `X402_ENABLED=true` → x402 payment gate activated
- [ ] Quota exceeded + `X402_ENABLED=false` → standard 429 rate limit response
- [ ] Successful x402 payment does NOT decrement quota (it's pay-per-use on top)
- [ ] `x402.payment.settled` event includes `quota_state: "exceeded"` metadata
- [ ] Unit test: quota remaining → pass, quota exceeded → 402, x402 disabled → 429

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: MCP Transport x402 Support [Integration]
**Description:** As an AI agent connecting via MCP, I want to pay for tool calls using x402 through the MCP JSON-RPC protocol.

**Acceptance Criteria:**
- [ ] MCP server reads `_meta["x402/payment"]` from JSON-RPC tool call params
- [ ] If tool requires payment and no `_meta["x402/payment"]`: return MCP error with payment requirements
- [ ] If payment present: verify via facilitator, settle, execute tool, return result
- [ ] Payment requirements returned in MCP error `data` field (same schema as HTTP `PAYMENT-REQUIRED`)
- [ ] MCP tool metadata includes `x402_price` field for tool discovery
- [ ] Works with both EVM and Solana payment signatures
- [ ] Integration test: MCP tool call without payment → error with requirements
- [ ] Integration test: MCP tool call with valid mock payment → success

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: Agent Dashboard & Payment History [Backend]
**Description:** As an agent operator, I want to see my agent's payment history and usage via API.

**Acceptance Criteria:**
- [ ] `GET /api/v1/agents/me` returns agent profile (tenant info, quotas, wallet addresses used)
- [ ] `GET /api/v1/agents/me/payments` returns paginated x402 payment history from Core events
- [ ] Response includes: timestamp, amount, asset, network, tx_hash, endpoint, status
- [ ] Supports `?since=` and `?until=` query params for time-range filtering
- [ ] Supports `?network=` filter (evm, solana, all)
- [ ] `GET /api/v1/agents/me/usage` returns current quota usage + x402 spend summary
- [ ] Unit test: payment history query with mock Core events

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: Agent registration must complete in a single HTTP call returning tenant_id + API key
- FR-2: x402 facilitator must verify payment signatures without calling external services (self-hosted)
- FR-3: x402 facilitator must settle payments on-chain within 30 seconds
- FR-4: Replay attacks must be prevented via nonce tracking
- FR-5: Payment events must be written to Core for audit trail (non-blocking)
- FR-6: x402 is only activated when agent exceeds free-tier quota
- FR-7: Both EVM (Base) and Solana payment rails must be supported
- FR-8: MCP transport must support x402 via `_meta["x402/payment"]`
- FR-9: All x402 functionality is behind `X402_ENABLED` feature flag (default: off)
- FR-10: Agent registration is rate-limited to prevent abuse

## Non-Goals (Out of Scope)

- No Query Service changes — all x402 logic lives in Control Plane
- No wallet provisioning in v1 — agents bring their own wallet (BYOW)
- No `upto` payment scheme — only `exact` (fixed price per call)
- No fiat payment rails — crypto only (USDC)
- No web dashboard UI for payments — API only (dashboard is a future story)
- No subscription management changes — LemonSqueezy flow unchanged
- No automated refunds — manual process via admin API

## Technical Considerations

- **Coinbase x402 Go SDK** (`github.com/coinbase/x402/go`) provides EVM types and helpers — use as reference but self-host facilitator logic
- **On-chain interaction**: Use `go-ethereum/ethclient` for EVM, Solana Go SDK for SPL transfers
- **Settlement wallet**: Control Plane needs a hot wallet private key (`X402_SETTLEMENT_WALLET_KEY`) for submitting settlement transactions
- **Nonce storage**: In-memory map with 1-hour TTL initially, backed by Core events for persistence
- **CAIP-2 network IDs**: `eip155:8453` (Base mainnet), `eip155:84532` (Base Sepolia), `solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp` (Solana mainnet)
- **USDC contract addresses**: Base mainnet `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`, Solana `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`

## Success Metrics

- Agent registration: < 2 seconds end-to-end
- x402 verify + settle: < 5 seconds (EVM), < 3 seconds (Solana)
- Payment event logging: < 100ms (non-blocking)
- Zero payment double-spends (nonce tracking)
- 100% of x402 payments have corresponding Core events

## Open Questions

1. What Base Sepolia USDC faucet should we document for agent testing?
2. Should x402 pricing be dynamic (based on load) or static?
3. Settlement wallet key management — env var vs. KMS vs. Coinbase custody?
4. Should the facilitator expose a public `/verify` endpoint for third-party use?
5. Gas sponsorship budget — who pays facilitator gas costs on Base?
