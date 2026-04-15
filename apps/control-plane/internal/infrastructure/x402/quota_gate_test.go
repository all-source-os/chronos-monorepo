package x402

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
)

func setupQuotaGateTest(quotaRemaining, x402Enabled bool) (*gin.Engine, *testCoreClient) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, &mockChainSettler{txHash: "0xtx"})

	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	pricing := &PricingConfig{
		Enabled:          x402Enabled,
		RecipientAddress: "0xRecipient",
		Routes: map[string]RoutePricing{
			"POST /api/v1/events": {
				Amount:   "100",
				Networks: []string{NetworkBaseMainnet},
			},
		},
	}

	checker := &StaticQuotaChecker{HasRemaining: quotaRemaining}

	router := gin.New()
	router.Use(func(c *gin.Context) {
		c.Set("tenant_id", "test-tenant")
		c.Next()
	})
	router.Use(QuotaGatedMiddleware(f, pricing, logger, checker))
	router.POST("/api/v1/events", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})

	return router, mock
}

func TestQuotaGate_QuotaRemaining_PassesThrough(t *testing.T) {
	router, _ := setupQuotaGateTest(true, true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}
}

func TestQuotaGate_QuotaExceeded_Returns402(t *testing.T) {
	router, mock := setupQuotaGateTest(false, true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402, got %d", w.Code)
	}

	// Should log payment.requested
	if len(mock.events) == 0 {
		t.Error("expected payment event")
	}
}

func TestQuotaGate_X402Disabled_PassesThrough(t *testing.T) {
	router, _ := setupQuotaGateTest(false, false)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	// x402 disabled — middleware passes through, handler responds 200
	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (x402 disabled passthrough), got %d", w.Code)
	}
}

func TestQuotaGate_FreeTier_Returns403(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, &mockChainSettler{txHash: "0xtx"})

	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	pricing := &PricingConfig{
		Enabled:          true,
		RecipientAddress: "0xRecipient",
		Routes: map[string]RoutePricing{
			"POST /api/v1/events": {Amount: "100", Networks: []string{NetworkBaseMainnet}},
		},
	}

	// Free-tier tenant: tier gate denies before quota or payment logic runs.
	checker := &StaticQuotaChecker{HasRemaining: true, TierDenied: true}

	router := gin.New()
	router.Use(func(c *gin.Context) {
		c.Set("tenant_id", "free-tenant")
		c.Next()
	})
	router.Use(QuotaGatedMiddleware(f, pricing, logger, checker))
	router.POST("/api/v1/events", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusForbidden {
		t.Errorf("status: want 403, got %d", w.Code)
	}
}

func TestQuotaGate_QuotaExceeded_WithPayment_Settles(t *testing.T) {
	router, mock := setupQuotaGateTest(false, true)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	req.Header.Set(HeaderPayment, makePaymentHeader(t, "0xquota-pay"))
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}

	hasSettled := false
	for _, evt := range mock.events {
		if evt.EventType == EventPaymentSettled {
			hasSettled = true
			break
		}
	}
	if !hasSettled {
		t.Error("expected payment.settled event")
	}
}
