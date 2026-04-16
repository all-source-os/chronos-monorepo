package x402

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"math/big"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
)

// WalletLookup retrieves a CDP wallet ID and address for a tenant.
// Implemented by the container to avoid a circular dependency.
type WalletLookup interface {
	// GetAgentWallet returns (walletID, address, error) for the given tenant.
	// Returns ("", "", nil) when no wallet has been provisioned.
	GetAgentWallet(ctx context.Context, tenantID string) (walletID, address string, err error)
}

// CDPSigner signs an EIP-3009 hash using a CDP-managed wallet.
// Matches clients.CDPClient.SignHash so the real client satisfies this interface.
type CDPSigner interface {
	SignHash(ctx context.Context, walletID string, hash []byte) (string, error)
}

// AgentAutoPayMiddleware handles x402 payments automatically on behalf of agents
// that have a Coinbase CDP managed wallet. Agents present only their API key —
// no wallet SDK or crypto knowledge required on the agent side.
//
// Flow:
//  1. Quota remaining → pass through (free tier)
//  2. Quota exceeded + no CDP wallet → return 402 (operator must fund a wallet)
//  3. Quota exceeded + CDP wallet + insufficient balance → return 402 with wallet address
//  4. Quota exceeded + CDP wallet + sufficient balance → auto-sign + settle → continue
func AgentAutoPayMiddleware(
	facilitator PaymentFacilitator,
	pricing *PricingConfig,
	logger *EventLogger,
	quotaChecker QuotaChecker,
	walletLookup WalletLookup,
	signer CDPSigner,
) gin.HandlerFunc {
	return func(c *gin.Context) {
		if !pricing.Enabled {
			c.Next()
			return
		}

		routeKey := RouteKey(c.Request.Method, c.Request.URL.Path)
		requirements := pricing.RequirementsForRoute(routeKey)
		if requirements == nil {
			c.Next()
			return
		}

		tenantIDVal, _ := c.Get("tenant_id")
		tenantIDStr, ok := tenantIDVal.(string)
		if !ok {
			tenantIDStr = ""
		}

		// Tier gate: free-tier tenants cannot consume x402 priced routes at
		// all, regardless of quota or CDP wallet balance. Pro-and-above only.
		// Mirrors the check in QuotaGatedMiddleware so both integration paths
		// enforce the same pricing-memo rule.
		if tierAllower, ok := quotaChecker.(X402TierAllower); ok && tenantIDStr != "" {
			if !tierAllower.AllowsX402(tenantIDStr) {
				c.JSON(http.StatusForbidden, gin.H{
					"error":   "tier_not_allowed",
					"message": "x402 agent endpoints require a Pro subscription or higher. Upgrade at https://all-source.xyz/billing",
				})
				c.Abort()
				return
			}
		}

		// Free tier: quota still available
		if tenantIDStr != "" && quotaChecker.HasQuota(tenantIDStr, routeKey) {
			c.Next()
			return
		}

		// Quota exceeded — attempt auto-pay if agent has a CDP wallet
		if walletLookup == nil || signer == nil || tenantIDStr == "" {
			// No CDP integration — fall back to standard 402
			standardPaymentRequired(c, requirements, logger, tenantIDStr, routeKey)
			return
		}

		walletID, walletAddr, err := walletLookup.GetAgentWallet(c.Request.Context(), tenantIDStr)
		if err != nil || walletID == "" {
			// No wallet provisioned — operator needs to call /register and fund the wallet
			standardPaymentRequired(c, requirements, logger, tenantIDStr, routeKey)
			return
		}

		// Pick the first accepted requirement (Base USDC)
		if len(requirements.Accepts) == 0 {
			c.AbortWithStatus(http.StatusInternalServerError)
			return
		}
		req := requirements.Accepts[0]

		// Parse required amount
		requiredAmount := new(big.Int)
		if _, ok := requiredAmount.SetString(req.Amount, 10); !ok {
			c.AbortWithStatus(http.StatusInternalServerError)
			return
		}

		// Build EIP-3009 hash params
		now := time.Now()
		validBefore := big.NewInt(now.Add(5 * time.Minute).Unix())

		var nonce [32]byte
		if _, err := rand.Read(nonce[:]); err != nil {
			c.AbortWithStatus(http.StatusInternalServerError)
			return
		}

		chainID, err := EVMChainID(req.Network)
		if err != nil {
			standardPaymentRequired(c, requirements, logger, tenantIDStr, routeKey)
			return
		}

		hashParams := EIP3009HashParams{
			From:        walletAddr,
			To:          req.PayTo,
			Value:       requiredAmount,
			ValidAfter:  big.NewInt(0),
			ValidBefore: validBefore,
			Nonce:       nonce,
			USDCAddress: req.Asset,
			ChainID:     chainID,
		}

		hash := EIP3009Hash(hashParams)

		// Sign with CDP — server-side MPC, agent never touches the key
		sig, err := signer.SignHash(c.Request.Context(), walletID, hash)
		if err != nil {
			if logger != nil {
				logger.LogFailed(c.Request.Context(), tenantIDStr, "cdp_sign_failed: "+err.Error(), req.Network, routeKey)
			}
			// Fall back to standard 402 so the agent can try with its own wallet
			standardPaymentRequired(c, requirements, logger, tenantIDStr, routeKey)
			return
		}

		// Build ExactEIP3009Payload
		auth := EIP3009Authorization{
			From:        walletAddr,
			To:          req.PayTo,
			Value:       requiredAmount.String(),
			ValidAfter:  "0",
			ValidBefore: validBefore.String(),
			Nonce:       "0x" + fmt.Sprintf("%x", nonce),
		}
		exactBytes, err := json.Marshal(ExactEIP3009Payload{Signature: sig, Authorization: auth})
		if err != nil {
			c.AbortWithStatus(http.StatusInternalServerError)
			return
		}

		payment := &PaymentPayload{
			X402Version: Version,
			Payload:     exactBytes,
			Accepted:    req,
		}

		settleResp, err := facilitator.Settle(c.Request.Context(), payment, requirements)
		if err != nil {
			if logger != nil {
				logger.LogFailed(c.Request.Context(), tenantIDStr, err.Error(), req.Network, routeKey)
			}
			c.JSON(http.StatusInternalServerError, gin.H{"error": "auto_payment_failed", "message": err.Error()})
			c.Abort()
			return
		}

		if !settleResp.Success {
			if logger != nil {
				logger.LogFailed(c.Request.Context(), tenantIDStr, settleResp.ErrorReason, req.Network, routeKey)
			}
			// Settlement failed — likely insufficient balance. Return 402 with wallet info.
			c.JSON(http.StatusPaymentRequired, gin.H{
				"error":          settleResp.ErrorReason,
				"message":        settleResp.ErrorMessage,
				"wallet_address": walletAddr,
				"required_usdc":  req.Amount,
				"network":        req.Network,
			})
			c.Abort()
			return
		}

		if logger != nil {
			logger.LogSettled(c.Request.Context(), tenantIDStr, settleResp.Transaction, settleResp.Payer, settleResp.Network, req.Amount, routeKey)
		}

		receiptEncoded, err := EncodeHeader(SettlementResponse{
			Success:     true,
			Transaction: settleResp.Transaction,
			Network:     settleResp.Network,
			Payer:       settleResp.Payer,
		})
		if err == nil {
			c.Header(HeaderPaymentResponse, receiptEncoded)
		}

		c.Next()
	}
}

// standardPaymentRequired returns a 402 with payment requirements.
func standardPaymentRequired(c *gin.Context, requirements *PaymentRequired, logger *EventLogger, tenantID, routeKey string) {
	if logger != nil && tenantID != "" {
		firstAccept := requirements.Accepts[0]
		logger.LogRequested(c.Request.Context(), tenantID, routeKey, firstAccept.Network, firstAccept.Amount)
	}
	encoded, err := EncodeHeader(requirements)
	if err != nil {
		c.AbortWithStatus(http.StatusInternalServerError)
		return
	}
	c.Header(HeaderPayment, encoded)
	c.JSON(http.StatusPaymentRequired, requirements)
	c.Abort()
}
