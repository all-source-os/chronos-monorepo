package x402

import (
	"context"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

// mockWalletLookup implements WalletLookup for testing.
type mockWalletLookup struct {
	walletID string
	address  string
	err      error
}

func (m *mockWalletLookup) GetAgentWallet(_ context.Context, _ string) (walletID, address string, err error) {
	return m.walletID, m.address, m.err
}

// mockCDPSigner implements CDPSigner for testing.
type mockCDPSigner struct {
	sig    string
	err    error
	called bool
}

func (m *mockCDPSigner) SignHash(_ context.Context, _ string, _ []byte) (string, error) {
	m.called = true
	return m.sig, m.err
}

// mockAutoPayFacilitator implements PaymentFacilitator for auto-pay tests.
type mockAutoPayFacilitator struct {
	settleResp *SettleResponse
	settleErr  error
}

func (m *mockAutoPayFacilitator) Verify(_ context.Context, _ *PaymentPayload, _ *PaymentRequired) (*VerifyResponse, error) {
	return &VerifyResponse{IsValid: true}, nil
}

func (m *mockAutoPayFacilitator) Settle(_ context.Context, _ *PaymentPayload, _ *PaymentRequired) (*SettleResponse, error) {
	return m.settleResp, m.settleErr
}

func setupAutoPayRouter(
	quotaRemaining bool,
	walletID, walletAddr string,
	signer *mockCDPSigner,
	facilitator PaymentFacilitator,
	withTenantID bool,
) (*gin.Engine, *testCoreClient) {
	pricing := enabledPricing()
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)
	wallet := &mockWalletLookup{walletID: walletID, address: walletAddr}
	quotaChecker := &StaticQuotaChecker{HasRemaining: quotaRemaining}

	router := gin.New()
	if withTenantID {
		router.Use(func(c *gin.Context) {
			c.Set("tenant_id", "test-tenant")
			c.Next()
		})
	}
	router.Use(AgentAutoPayMiddleware(facilitator, pricing, logger, quotaChecker, wallet, signer))
	router.POST("/api/v1/events", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})
	router.GET("/api/v1/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "healthy"})
	})

	return router, mock
}

func successFacilitator(walletAddr string) *mockAutoPayFacilitator {
	return &mockAutoPayFacilitator{
		settleResp: &SettleResponse{
			Success:     true,
			Transaction: "0xautotx",
			Payer:       walletAddr,
			Network:     NetworkBaseMainnet,
		},
	}
}

func TestAutoPay_QuotaRemaining_PassesThrough(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xsig"}
	router, _ := setupAutoPayRouter(true, "wallet-1", "0xAddr", signer, successFacilitator("0xAddr"), true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (quota available), got %d", w.Code)
	}
	if signer.called {
		t.Error("signer should not be called when quota is available")
	}
}

func TestAutoPay_PricingDisabled_PassesThrough(t *testing.T) {
	pricing := &PricingConfig{Enabled: false}
	mock := &testCoreClient{}
	signer := &mockCDPSigner{sig: "0xsig"}
	wallet := &mockWalletLookup{walletID: "w1", address: "0xAddr"}
	facilitator := &mockAutoPayFacilitator{settleResp: &SettleResponse{Success: true}}

	router := gin.New()
	router.Use(func(c *gin.Context) { c.Set("tenant_id", "t1"); c.Next() })
	router.Use(AgentAutoPayMiddleware(facilitator, pricing, NewEventLogger(mock), &StaticQuotaChecker{HasRemaining: false}, wallet, signer))
	router.POST("/api/v1/events", func(c *gin.Context) { c.JSON(http.StatusOK, gin.H{}) })

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (pricing disabled), got %d", w.Code)
	}
	if signer.called {
		t.Error("signer should not be called when pricing is disabled")
	}
}

func TestAutoPay_QuotaExceeded_NoWallet_Returns402(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xsig"}
	// walletID="" means GetAgentWallet returns ("", "", nil) → no wallet provisioned
	router, _ := setupAutoPayRouter(false, "", "", signer, successFacilitator(""), true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402 (no wallet), got %d", w.Code)
	}
	if w.Header().Get(HeaderPayment) == "" {
		t.Error("expected X-Payment header in standard 402 response")
	}
}

func TestAutoPay_QuotaExceeded_WithWallet_AutoSettles_Returns200(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xcdpsig"}
	router, coreLog := setupAutoPayRouter(false, "wallet-1", "0xAgentAddr", signer, successFacilitator("0xAgentAddr"), true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (auto-pay settled), got %d", w.Code)
	}
	if !signer.called {
		t.Error("expected CDP signer to be called")
	}
	if w.Header().Get(HeaderPaymentResponse) == "" {
		t.Error("expected X-Payment-Response header after settlement")
	}
	hasSettled := false
	for _, evt := range coreLog.events {
		if evt.EventType == EventPaymentSettled {
			hasSettled = true
			break
		}
	}
	if !hasSettled {
		t.Error("expected payment.settled event logged to Core")
	}
}

func TestAutoPay_SignerFails_FallsBackTo402(t *testing.T) {
	signer := &mockCDPSigner{err: errors.New("key unavailable")}
	router, _ := setupAutoPayRouter(false, "wallet-1", "0xAgentAddr", signer, successFacilitator("0xAgentAddr"), true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402 (sign failed fallback), got %d", w.Code)
	}
	if !signer.called {
		t.Error("expected signer to be called (and fail)")
	}
}

func TestAutoPay_SettlementFails_Returns402WithWalletInfo(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xsig"}
	failFacilitator := &mockAutoPayFacilitator{
		settleResp: &SettleResponse{
			Success:      false,
			ErrorReason:  "insufficient_balance",
			ErrorMessage: "wallet balance too low",
		},
	}
	router, _ := setupAutoPayRouter(false, "wallet-1", "0xAgentAddr", signer, failFacilitator, true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402 (settlement failed), got %d", w.Code)
	}
	body := w.Body.String()
	if !strings.Contains(body, "wallet_address") {
		t.Errorf("response should include wallet_address, got: %s", body)
	}
	if !strings.Contains(body, "required_usdc") {
		t.Errorf("response should include required_usdc, got: %s", body)
	}
}

func TestAutoPay_UnpricedRoute_PassesThrough(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xsig"}
	// quota exceeded but route is not priced → pass through
	router, _ := setupAutoPayRouter(false, "wallet-1", "0xAgentAddr", signer, successFacilitator("0xAgentAddr"), true)

	req := httptest.NewRequest(http.MethodGet, "/api/v1/health", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (unpriced route), got %d", w.Code)
	}
	if signer.called {
		t.Error("signer should not be called for unpriced route")
	}
}

func TestAutoPay_NoTenantID_FallsTo402(t *testing.T) {
	signer := &mockCDPSigner{sig: "0xsig"}
	// withTenantID=false: no tenant_id set in context
	router, _ := setupAutoPayRouter(false, "wallet-1", "0xAgentAddr", signer, successFacilitator("0xAgentAddr"), false)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402 (no tenant_id), got %d", w.Code)
	}
}
