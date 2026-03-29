# x402 Auto-Pay Setup

Chronos Control Plane supports [x402](https://github.com/coinbase/x402) machine-to-machine payments on Base (USDC). When an agent exhausts its free-tier quota, the Control Plane can automatically settle micro-payments using a Coinbase CDP managed wallet — no wallet SDK or private keys on the agent side.

## How it works

1. **Free tier** — Every agent gets a quota of events/queries per billing period. Requests within quota pass through at no cost.
2. **Quota exceeded + no wallet** — The Control Plane returns a standard HTTP 402 with payment requirements.
3. **Quota exceeded + CDP wallet provisioned** — The Control Plane auto-signs an EIP-3009 authorization using the agent's CDP-managed wallet, settles via `https://x402.org/facilitator`, and continues the request. The agent sees a `200 OK` with an `X-Payment-Response` receipt header.

## Required environment variables

### Pricing config

```
X402_PRICING_CONFIG=/path/to/pricing.json
```

Copy `docs/x402-pricing.example.json`, set your recipient address and per-route amounts, and point this env var at it. Without this file, x402 is disabled.

### Coinbase CDP (for agent auto-pay)

```
COINBASE_CDP_KEY_NAME=organizations/{orgId}/apiKeys/{keyId}
COINBASE_CDP_PRIVATE_KEY="-----BEGIN EC PRIVATE KEY-----\n...\n-----END EC PRIVATE KEY-----"
COINBASE_CDP_NETWORK=base-mainnet          # or base-sepolia for testing
BASE_RPC_URL=https://mainnet.base.org      # optional, defaults to mainnet
```

Obtain these from the [Coinbase Developer Platform](https://portal.cdp.coinbase.com/). The private key may contain literal `\n` — the Control Plane handles both literal and real newlines.

Without CDP credentials, agent registration still works but no CDP wallets are provisioned. Agents that exceed quota will receive a standard 402.

### Facilitator (optional override)

```
X402_FACILITATOR_URL=https://x402.org/facilitator  # default, Coinbase-hosted
```

Override to point at a self-hosted facilitator. The self-hosted facilitator requires a local Base node and private key for on-chain settlement.

## Funding an agent wallet

When an agent registers via `POST /api/v1/agents/register`, the response includes a `wallet_address` field (if CDP is configured). Send USDC to that address on Base before the agent exhausts its free quota.

The wallet address is also returned in 402 responses when the balance is insufficient:

```json
{
  "error": "insufficient_balance",
  "wallet_address": "0x...",
  "required_usdc": "100",
  "network": "eip155:8453"
}
```

## USDC amounts

Amounts are in atomic units (6 decimal places):

| Amount string | USDC value |
|--------------|------------|
| `"100"`      | $0.0001    |
| `"1000"`     | $0.001     |
| `"1000000"`  | $1.00      |

## Quota dimensions

The quota checker maps route keys to quota dimensions:

| Route pattern     | Quota dimension          |
|-------------------|--------------------------|
| `POST /api/v1/events` | `events_used` / `events_quota` |
| All other routes  | `queries_used` / `queries_quota` |

Quota values are stored in tenant metadata under the `quota` key. A value of `-1` means unlimited.

## Using Base Sepolia (testnet)

For staging and integration testing, use Base Sepolia instead of mainnet:

```
COINBASE_CDP_NETWORK=base-sepolia
BASE_RPC_URL=https://sepolia.base.org
```

In your pricing config, reference the Sepolia USDC contract and network:

```json
{
  "routes": [
    {
      "path": "/api/v1/events",
      "amount": "100",
      "networks": ["eip155:84532"],
      "asset": "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
    }
  ],
  "payTo": "0xYourRecipientAddress"
}
```

Fund test wallets using the [Circle USDC faucet](https://faucet.circle.com) — select "Base Sepolia" and paste the `wallet_address` from the agent registration response.

## End-to-end staging validation

Use Base Sepolia to validate the full auto-pay flow before going to mainnet:

1. **Deploy** with Sepolia env vars (`COINBASE_CDP_NETWORK=base-sepolia`, `BASE_RPC_URL=https://sepolia.base.org`).

2. **Register an agent** and capture the wallet address:
   ```
   POST /api/v1/agents/register
   {"agent_name": "test-agent", "agent_type": "mcp"}
   ```
   Copy `wallet_address` from the response.

3. **Fund the wallet** via the [Circle USDC faucet](https://faucet.circle.com) using the address from step 2. Wait ~30 seconds for the transaction to confirm.

4. **Exhaust the quota** by sending events, then make one more request. If the wallet has sufficient balance you should see `200 OK` with an `X-Payment-Response` header rather than a `402`.

5. **Inspect settlement in Core:**
   ```
   GET /api/v1/events/query?event_type=x402.payment.settled
   ```
   The settled event payload includes `tx_hash`, `amount`, and `payer`.

6. **Verify registration event** includes `wallet_address`:
   ```
   GET /api/v1/events/query?event_type=agent.registered&entity_id=agent-test-agent
   ```

## Operational notes — Nonce deduplication

The EIP-3009 `nonce` field is a random 32-byte value generated per payment request. The in-process nonce tracker (`NonceTracker`) prevents replay attacks within a single Control Plane process lifetime by recording used nonces.

**Multi-instance deployments**: Each process maintains its own in-memory nonce set. To prevent cross-instance replay attacks, share nonce state via Redis (set `X402_NONCE_STORE=redis://...`) or rely on the facilitator's server-side deduplication. The `ValidBefore` timestamp (5 minutes from signing) bounds the replay window even without shared state.
