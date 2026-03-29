package clients

import (
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/dgrijalva/jwt-go"
)

// newTestCDPClientWithAPIBase constructs a client that hits the given baseURL for CDP API calls
// and a separate rpcURL for JSON-RPC calls.
func newTestCDPClientWithAPIBase(t *testing.T, apiBase, rpcURL, network string) *CoinbaseCDPClient {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatalf("generate test key: %v", err)
	}
	return &CoinbaseCDPClient{
		keyName:    "organizations/test-org/apiKeys/test-key",
		privateKey: key,
		httpClient: &http.Client{Timeout: 5 * time.Second},
		network:    network,
		rpcURL:     rpcURL,
	}
}

func TestCDPClient_CreateWallet_Success(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || !strings.HasSuffix(r.URL.Path, "/wallets") {
			t.Errorf("unexpected request: %s %s", r.Method, r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{ //nolint:errcheck
			"wallet": map[string]any{
				"id": "wallet-abc",
				"default_address": map[string]string{
					"address_id": "0xDeadBeef",
				},
			},
		})
	}))
	defer srv.Close()

	// Override cdpAPIBase for the test by replacing it in the request URL.
	// Since cdpDo constructs the URL as cdpAPIBase+path, we use a transport that rewrites the host.
	c := newTestCDPClientWithAPIBase(t, srv.URL, srv.URL, "base-mainnet")
	c.httpClient = newRewriteClient(srv.URL)

	wallet, err := c.CreateWallet(context.Background(), "my-agent")
	if err != nil {
		t.Fatalf("CreateWallet: %v", err)
	}
	if wallet.ID != "wallet-abc" {
		t.Errorf("ID: want wallet-abc, got %s", wallet.ID)
	}
	if wallet.Address != "0xDeadBeef" {
		t.Errorf("Address: want 0xDeadBeef, got %s", wallet.Address)
	}
	if wallet.Network != "base-mainnet" {
		t.Errorf("Network: want base-mainnet, got %s", wallet.Network)
	}
}

func TestCDPClient_CreateWallet_APIError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]any{"error": "invalid_key"}) //nolint:errcheck
	}))
	defer srv.Close()

	c := newTestCDPClientWithAPIBase(t, srv.URL, srv.URL, "base-mainnet")
	c.httpClient = newRewriteClient(srv.URL)

	_, err := c.CreateWallet(context.Background(), "my-agent")
	if err == nil {
		t.Fatal("expected error from 401 response, got nil")
	}
	if !strings.Contains(err.Error(), "401") {
		t.Errorf("error should mention HTTP 401, got: %v", err)
	}
}

func TestCDPClient_GetAddress_Success(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{ //nolint:errcheck
			"data": []map[string]string{
				{"address_id": "0xAlice"},
			},
		})
	}))
	defer srv.Close()

	c := newTestCDPClientWithAPIBase(t, srv.URL, srv.URL, "base-mainnet")
	c.httpClient = newRewriteClient(srv.URL)

	addr, err := c.GetAddress(context.Background(), "wallet-123")
	if err != nil {
		t.Fatalf("GetAddress: %v", err)
	}
	if addr != "0xAlice" {
		t.Errorf("address: want 0xAlice, got %s", addr)
	}
}

func TestCDPClient_GetAddress_EmptyData(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{"data": []any{}}) //nolint:errcheck
	}))
	defer srv.Close()

	c := newTestCDPClientWithAPIBase(t, srv.URL, srv.URL, "base-mainnet")
	c.httpClient = newRewriteClient(srv.URL)

	_, err := c.GetAddress(context.Background(), "wallet-empty")
	if err == nil {
		t.Fatal("expected error for empty address list, got nil")
	}
	if !strings.Contains(err.Error(), "no addresses") {
		t.Errorf("error should mention no addresses, got: %v", err)
	}
}

func TestCDPClient_SignHash_Success(t *testing.T) {
	callCount := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		w.Header().Set("Content-Type", "application/json")
		if strings.Contains(r.URL.Path, "addresses") && !strings.Contains(r.URL.Path, "sign_payload") {
			// GetAddress call
			json.NewEncoder(w).Encode(map[string]any{ //nolint:errcheck
				"data": []map[string]string{{"address_id": "0xWalletAddr"}},
			})
		} else {
			// sign_payload call
			json.NewEncoder(w).Encode(map[string]string{"signed_payload": "0xSignature"}) //nolint:errcheck
		}
	}))
	defer srv.Close()

	c := newTestCDPClientWithAPIBase(t, srv.URL, srv.URL, "base-mainnet")
	c.httpClient = newRewriteClient(srv.URL)

	hash := make([]byte, 32)
	sig, err := c.SignHash(context.Background(), "wallet-xyz", hash)
	if err != nil {
		t.Fatalf("SignHash: %v", err)
	}
	if sig != "0xSignature" {
		t.Errorf("sig: want 0xSignature, got %s", sig)
	}
	if callCount != 2 {
		t.Errorf("expected 2 CDP calls (GetAddress + sign_payload), got %d", callCount)
	}
}

func TestCDPClient_SignCDPJWT_ClaimsFormat(t *testing.T) {
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatalf("generate key: %v", err)
	}
	c := &CoinbaseCDPClient{
		keyName:    "organizations/org1/apiKeys/key1",
		privateKey: key,
		httpClient: &http.Client{},
	}

	tokenStr, err := c.signCDPJWT("POST", "/wallets")
	if err != nil {
		t.Fatalf("signCDPJWT: %v", err)
	}

	// Parse without verification to inspect claims.
	parser := jwt.Parser{}
	token, _, err := parser.ParseUnverified(tokenStr, jwt.MapClaims{})
	if err != nil {
		t.Fatalf("parse JWT: %v", err)
	}

	claims, ok := token.Claims.(jwt.MapClaims)
	if !ok {
		t.Fatal("expected MapClaims")
	}

	// kid header
	if token.Header["kid"] != c.keyName {
		t.Errorf("kid: want %s, got %v", c.keyName, token.Header["kid"])
	}
	// iss
	if claims["iss"] != "cdp" {
		t.Errorf("iss: want cdp, got %v", claims["iss"])
	}
	// sub == key name
	if claims["sub"] != c.keyName {
		t.Errorf("sub: want %s, got %v", c.keyName, claims["sub"])
	}
	// uri must be "METHOD hostname/platform/v1/path" (no scheme)
	expectedURI := "POST api.developer.coinbase.com/platform/v1/wallets"
	if claims["uri"] != expectedURI {
		t.Errorf("uri: want %q, got %q", expectedURI, claims["uri"])
	}
	// exp must be set and in the future
	exp, ok := claims["exp"].(float64)
	if !ok {
		t.Fatal("exp claim is not a float64")
	}
	if exp <= float64(time.Now().Unix()) {
		t.Errorf("exp should be in the future, got %v", exp)
	}
}

func TestCDPClient_GetUSDCBalance_MainnetVsSepolia(t *testing.T) {
	var capturedTo string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var req map[string]any
		if err := json.NewDecoder(r.Body).Decode(&req); err == nil {
			if params, ok := req["params"].([]any); ok && len(params) > 0 {
				if call, ok := params[0].(map[string]any); ok {
					if to, ok := call["to"].(string); ok {
						capturedTo = to
					}
				}
			}
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{"result": "0x0"}) //nolint:errcheck
	}))
	defer srv.Close()

	// Mainnet
	mainnet := &CoinbaseCDPClient{network: "base-mainnet", rpcURL: srv.URL, httpClient: srv.Client()}
	_, err := mainnet.GetUSDCBalance(context.Background(), "0x1234")
	if err != nil {
		t.Fatalf("mainnet GetUSDCBalance: %v", err)
	}
	mainnetContract := capturedTo

	// Sepolia
	sepolia := &CoinbaseCDPClient{network: "base-sepolia", rpcURL: srv.URL, httpClient: srv.Client()}
	_, err = sepolia.GetUSDCBalance(context.Background(), "0x1234")
	if err != nil {
		t.Fatalf("sepolia GetUSDCBalance: %v", err)
	}
	sepoliaContract := capturedTo

	if mainnetContract == sepoliaContract {
		t.Errorf("mainnet and sepolia should use different USDC contracts, both got %s", mainnetContract)
	}
	// Verify the known contract addresses
	if mainnetContract != "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" {
		t.Errorf("mainnet USDC contract: want 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913, got %s", mainnetContract)
	}
	if sepoliaContract != "0x036CbD53842c5426634e7929541eC2318f3dCF7e" {
		t.Errorf("sepolia USDC contract: want 0x036CbD53842c5426634e7929541eC2318f3dCF7e, got %s", sepoliaContract)
	}
}

// newRewriteClient returns an http.Client whose transport rewrites all requests to target the
// given base URL. This lets us intercept requests that hard-code cdpAPIBase.
func newRewriteClient(baseURL string) *http.Client {
	base := strings.TrimRight(baseURL, "/")
	return &http.Client{
		Timeout:   5 * time.Second,
		Transport: &rewriteTransport{base: base, inner: http.DefaultTransport},
	}
}

type rewriteTransport struct {
	base  string
	inner http.RoundTripper
}

func (t *rewriteTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	// Keep path and query; replace scheme+host with test server base.
	newURL := t.base + req.URL.Path
	if req.URL.RawQuery != "" {
		newURL += "?" + req.URL.RawQuery
	}
	newReq := req.Clone(req.Context())
	newReq.URL, _ = req.URL.Parse(newURL) //nolint:errcheck
	newReq.Host = ""
	return t.inner.RoundTrip(newReq)
}
