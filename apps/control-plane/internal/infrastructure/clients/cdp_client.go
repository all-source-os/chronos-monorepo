package clients

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/rand"
	"crypto/x509"
	"encoding/hex"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"math/big"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/cenkalti/backoff/v5"
	"github.com/dgrijalva/jwt-go"
)

const cdpAPIBase = "https://api.developer.coinbase.com/platform/v1"

// CDPWallet holds the managed wallet info returned by Coinbase CDP.
type CDPWallet struct {
	ID      string `json:"id"`
	Address string `json:"address"`
	Network string `json:"network"` // e.g. "base-mainnet"
}

// CDPClient provisions and operates managed wallets through Coinbase Developer Platform.
type CDPClient interface {
	// CreateWallet creates a new MPC-managed wallet on Base for the given agent.
	// Returns the wallet ID and its default address.
	CreateWallet(ctx context.Context, agentName string) (*CDPWallet, error)

	// GetAddress returns the primary address for an existing wallet.
	GetAddress(ctx context.Context, walletID string) (string, error)

	// GetUSDCBalance returns the USDC balance in atomic units (6 decimals) for an address on Base.
	GetUSDCBalance(ctx context.Context, address string) (*big.Int, error)

	// SignHash signs an arbitrary 32-byte EIP-712 hash using the CDP-managed key.
	// Returns the 65-byte hex-encoded signature (r||s||v).
	// Callers should compute the hash (e.g. x402.EIP3009Hash) before calling this.
	SignHash(ctx context.Context, walletID string, hash []byte) (string, error)
}

// CoinbaseCDPClient implements CDPClient using the Coinbase Developer Platform REST API.
// Authentication uses ES256 JWTs signed with the CDP API key (no SDK dependency).
type CoinbaseCDPClient struct {
	keyName    string // e.g. "organizations/{orgId}/apiKeys/{keyId}"
	privateKey *ecdsa.PrivateKey
	httpClient *http.Client
	network    string // CDP network name, e.g. "base-mainnet"
	rpcURL     string // Base JSON-RPC endpoint for balance queries
}

// NewCoinbaseCDPClient creates a CDP client from environment variables:
//
//	COINBASE_CDP_KEY_NAME   — API key name (organizations/.../apiKeys/...)
//	COINBASE_CDP_PRIVATE_KEY — EC P-256 private key in PEM format
//	COINBASE_CDP_NETWORK    — "base-mainnet" (default) or "base-sepolia"
//	BASE_RPC_URL            — JSON-RPC URL for Base (default: https://mainnet.base.org)
func NewCoinbaseCDPClient() (*CoinbaseCDPClient, error) {
	keyName := os.Getenv("COINBASE_CDP_KEY_NAME")
	if keyName == "" {
		return nil, fmt.Errorf("COINBASE_CDP_KEY_NAME not set")
	}

	pemData := os.Getenv("COINBASE_CDP_PRIVATE_KEY")
	if pemData == "" {
		return nil, fmt.Errorf("COINBASE_CDP_PRIVATE_KEY not set")
	}
	// PEM may have literal \n instead of real newlines (common in env vars)
	pemData = strings.ReplaceAll(pemData, `\n`, "\n")

	privateKey, err := parseECPrivateKey(pemData)
	if err != nil {
		return nil, fmt.Errorf("parse CDP private key: %w", err)
	}

	network := os.Getenv("COINBASE_CDP_NETWORK")
	if network == "" {
		network = "base-mainnet"
	}

	rpcURL := os.Getenv("BASE_RPC_URL")
	if rpcURL == "" {
		rpcURL = "https://mainnet.base.org"
	}

	return &CoinbaseCDPClient{
		keyName:    keyName,
		privateKey: privateKey,
		httpClient: &http.Client{Timeout: 30 * time.Second},
		network:    network,
		rpcURL:     rpcURL,
	}, nil
}

// CreateWallet provisions a new managed wallet on Base for the given agent.
func (c *CoinbaseCDPClient) CreateWallet(ctx context.Context, agentName string) (*CDPWallet, error) {
	body, _ := json.Marshal(map[string]any{ //nolint:errcheck
		"network_id": c.network,
		"metadata":   map[string]string{"name": agentName},
	})

	var result struct {
		Wallet struct {
			ID             string `json:"id"`
			DefaultAddress struct {
				AddressID string `json:"address_id"`
			} `json:"default_address"`
		} `json:"wallet"`
	}

	if err := c.cdpPost(ctx, "/wallets", body, &result); err != nil {
		return nil, fmt.Errorf("create CDP wallet: %w", err)
	}

	return &CDPWallet{
		ID:      result.Wallet.ID,
		Address: result.Wallet.DefaultAddress.AddressID,
		Network: c.network,
	}, nil
}

// GetAddress returns the primary address of an existing wallet.
func (c *CoinbaseCDPClient) GetAddress(ctx context.Context, walletID string) (string, error) {
	var result struct {
		Data []struct {
			AddressID string `json:"address_id"`
		} `json:"data"`
	}
	if err := c.cdpGet(ctx, "/wallets/"+walletID+"/addresses", &result); err != nil {
		return "", fmt.Errorf("get CDP wallet address: %w", err)
	}
	if len(result.Data) == 0 {
		return "", fmt.Errorf("wallet %s has no addresses", walletID)
	}
	return result.Data[0].AddressID, nil
}

// GetUSDCBalance queries the USDC balance for an address via Base JSON-RPC.
// Uses eth_call on the USDC contract — no CDP auth required.
func (c *CoinbaseCDPClient) GetUSDCBalance(ctx context.Context, address string) (*big.Int, error) {
	// balanceOf(address) = 0x70a08231 + address left-padded to 32 bytes
	addr := strings.TrimPrefix(strings.ToLower(address), "0x")
	data := "0x70a08231" + fmt.Sprintf("%064s", addr)

	usdcContract := "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" // Base mainnet USDC
	if c.network == "base-sepolia" {
		usdcContract = "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
	}

	payload, _ := json.Marshal(map[string]any{ //nolint:errcheck
		"jsonrpc": "2.0",
		"method":  "eth_call",
		"params": []any{
			map[string]string{"to": usdcContract, "data": data},
			"latest",
		},
		"id": 1,
	})

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.rpcURL, bytes.NewReader(payload))
	if err != nil {
		return nil, fmt.Errorf("build rpc request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("rpc call: %w", err)
	}
	defer resp.Body.Close() //nolint:errcheck

	var rpcResp struct {
		Result string `json:"result"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&rpcResp); err != nil {
		return nil, fmt.Errorf("decode rpc response: %w", err)
	}

	hexStr := strings.TrimPrefix(rpcResp.Result, "0x")
	if hexStr == "" {
		return big.NewInt(0), nil
	}
	balance := new(big.Int)
	balance.SetString(hexStr, 16)
	return balance, nil
}

// SignHash asks CDP to sign an arbitrary 32-byte hash using the wallet's MPC key.
// Use this with a pre-computed EIP-712 hash (e.g. from x402.EIP3009Hash).
// Returns the 65-byte hex-encoded signature (r||s||v).
func (c *CoinbaseCDPClient) SignHash(ctx context.Context, walletID string, hash []byte) (string, error) {
	address, err := c.GetAddress(ctx, walletID)
	if err != nil {
		return "", err
	}

	hashHex := "0x" + hex.EncodeToString(hash)

	body, _ := json.Marshal(map[string]string{"unsigned_payload": hashHex}) //nolint:errcheck

	var result struct {
		SignedPayload string `json:"signed_payload"`
	}
	signPath := "/wallets/" + walletID + "/addresses/" + address + "/sign_payload"
	if err := c.cdpPost(ctx, signPath, body, &result); err != nil {
		return "", fmt.Errorf("CDP sign_payload: %w", err)
	}

	return result.SignedPayload, nil
}

// --- CDP HTTP helpers ---

func (c *CoinbaseCDPClient) cdpPost(ctx context.Context, path string, body []byte, out any) error {
	return c.cdpDo(ctx, http.MethodPost, path, body, out)
}

func (c *CoinbaseCDPClient) cdpGet(ctx context.Context, path string, out any) error {
	return c.cdpDo(ctx, http.MethodGet, path, nil, out)
}

// cdpDo executes a CDP API request with automatic retry on transient failures.
// Retries on 429 (rate limit) and 5xx (server errors) using exponential backoff,
// with a 10-second total elapsed time limit.
// Permanent 4xx errors (except 429) are not retried.
func (c *CoinbaseCDPClient) cdpDo(ctx context.Context, method, path string, body []byte, out any) error {
	attempt := func() (struct{}, error) {
		// Re-sign JWT per attempt: each token scopes to one request with a fresh nonce.
		authToken, err := c.signCDPJWT(method, path)
		if err != nil {
			return struct{}{}, backoff.Permanent(fmt.Errorf("sign CDP JWT: %w", err))
		}

		var reqBody *bytes.Reader
		if body != nil {
			reqBody = bytes.NewReader(body)
		} else {
			reqBody = bytes.NewReader(nil)
		}

		req, err := http.NewRequestWithContext(ctx, method, cdpAPIBase+path, reqBody)
		if err != nil {
			return struct{}{}, backoff.Permanent(fmt.Errorf("build CDP request: %w", err))
		}
		req.Header.Set("Authorization", "Bearer "+authToken)
		req.Header.Set("Content-Type", "application/json")

		resp, err := c.httpClient.Do(req)
		if err != nil {
			// Network errors are transient — allow retry.
			return struct{}{}, fmt.Errorf("CDP request %s %s: %w", method, path, err)
		}
		defer resp.Body.Close() //nolint:errcheck

		if resp.StatusCode == http.StatusTooManyRequests || resp.StatusCode >= 500 {
			// Transient: rate limit or server error — allow retry.
			var errBody map[string]any
			if decErr := json.NewDecoder(resp.Body).Decode(&errBody); decErr != nil {
				errBody = nil
			}
			return struct{}{}, fmt.Errorf("CDP API %s %s: HTTP %d %v", method, path, resp.StatusCode, errBody)
		}

		if resp.StatusCode >= 400 {
			// Permanent: client error (4xx except 429) — no retry.
			var errBody map[string]any
			if decErr := json.NewDecoder(resp.Body).Decode(&errBody); decErr != nil {
				errBody = nil
			}
			return struct{}{}, backoff.Permanent(fmt.Errorf("CDP API %s %s: HTTP %d %v", method, path, resp.StatusCode, errBody))
		}

		return struct{}{}, json.NewDecoder(resp.Body).Decode(out)
	}

	_, err := backoff.Retry(ctx, attempt, backoff.WithMaxElapsedTime(10*time.Second))
	return err
}

// signCDPJWT creates the ES256 JWT required for Coinbase CDP API authentication.
// The `uri` claim encodes the HTTP method and path, scoping the token to one request.
func (c *CoinbaseCDPClient) signCDPJWT(method, path string) (string, error) {
	nonce := make([]byte, 16)
	if _, err := rand.Read(nonce); err != nil {
		return "", fmt.Errorf("generate nonce: %w", err)
	}

	now := time.Now()
	// uri format: "METHOD hostname/path" (no scheme, no query)
	uri := method + " api.developer.coinbase.com" + "/platform/v1" + path

	claims := jwt.MapClaims{
		"iss": "cdp",
		"sub": c.keyName,
		"nbf": now.Unix(),
		"exp": now.Add(2 * time.Minute).Unix(),
		"uri": uri,
	}

	token := jwt.NewWithClaims(jwt.SigningMethodES256, claims)
	token.Header["kid"] = c.keyName

	return token.SignedString(c.privateKey)
}

// parseECPrivateKey parses an EC P-256 private key from PEM format.
func parseECPrivateKey(pemData string) (*ecdsa.PrivateKey, error) {
	block, _ := pem.Decode([]byte(pemData))
	if block == nil {
		return nil, fmt.Errorf("failed to decode PEM block")
	}

	key, err := x509.ParseECPrivateKey(block.Bytes)
	if err != nil {
		// Try PKCS8 format
		pkcs8Key, pkcs8Err := x509.ParsePKCS8PrivateKey(block.Bytes)
		if pkcs8Err != nil {
			return nil, fmt.Errorf("parse EC key (SEC1: %w, PKCS8: %w)", err, pkcs8Err)
		}
		ecKey, ok := pkcs8Key.(*ecdsa.PrivateKey)
		if !ok {
			return nil, fmt.Errorf("PKCS8 key is not EC")
		}
		return ecKey, nil
	}
	return key, nil
}
