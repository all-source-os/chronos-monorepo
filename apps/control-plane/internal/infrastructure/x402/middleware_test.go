package x402

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
)

func init() {
	gin.SetMode(gin.TestMode)
}

func setupMiddlewareTest(pricing *PricingConfig, verifyPayer, settleTx string, settleErr error) (*gin.Engine, *testCoreClient) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	verifier := &mockChainVerifier{payer: verifyPayer}
	settler := &mockChainSettler{txHash: settleTx, err: settleErr}
	f.RegisterEVM(verifier, settler)

	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	router := gin.New()
	router.Use(func(c *gin.Context) {
		c.Set("tenant_id", "test-tenant")
		c.Next()
	})
	router.Use(Middleware(f, pricing, logger))
	router.POST("/api/v1/events", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "ok"})
	})
	router.GET("/api/v1/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"status": "healthy"})
	})

	return router, mock
}

func enabledPricing() *PricingConfig {
	return &PricingConfig{
		Enabled:          true,
		RecipientAddress: "0xRecipient",
		Routes: map[string]RoutePricing{
			"POST /api/v1/events": {
				Amount:      "100",
				Networks:    []string{NetworkBaseMainnet},
				Description: "Event ingestion",
			},
		},
	}
}

func makePaymentHeader(t *testing.T, nonce string) string {
	t.Helper()
	auth := EIP3009Authorization{
		From: "0xPayer", To: "0xRecipient", Value: "100",
		ValidAfter: "0", ValidBefore: "9999999999", Nonce: nonce,
	}
	exactBytes, err := json.Marshal(ExactEIP3009Payload{
		Signature: "0xSig", Authorization: auth,
	})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	payment := PaymentPayload{
		X402Version: Version,
		Payload:     exactBytes,
		Accepted: PaymentRequirements{
			Scheme: SchemeExact, Network: NetworkBaseMainnet, Amount: "100", PayTo: "0xRecipient",
		},
	}
	encoded, err := EncodeHeader(payment)
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	return encoded
}

func TestMiddleware_NoPayment_Returns402(t *testing.T) {
	router, mock := setupMiddlewareTest(enabledPricing(), "0xPayer", "0xtx", nil)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusPaymentRequired {
		t.Errorf("status: want 402, got %d", w.Code)
	}

	paymentHeader := w.Header().Get(HeaderPayment)
	if paymentHeader == "" {
		t.Error("expected X-Payment header")
	}

	var requirements PaymentRequired
	if err := DecodeHeader(paymentHeader, &requirements); err != nil {
		t.Fatalf("decode header: %v", err)
	}
	if len(requirements.Accepts) != 1 {
		t.Fatalf("accepts: want 1, got %d", len(requirements.Accepts))
	}
	if requirements.Accepts[0].Amount != "100" {
		t.Errorf("amount: want 100, got %s", requirements.Accepts[0].Amount)
	}

	if len(mock.events) != 1 {
		t.Fatalf("events: want 1, got %d", len(mock.events))
	}
	if mock.events[0].EventType != EventPaymentRequested {
		t.Errorf("event: want %s, got %s", EventPaymentRequested, mock.events[0].EventType)
	}
}

func TestMiddleware_ValidPayment_PassesThrough(t *testing.T) {
	router, mock := setupMiddlewareTest(enabledPricing(), "0xPayer", "0xdeadbeef", nil)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	req.Header.Set(HeaderPayment, makePaymentHeader(t, "0xunique1"))
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}

	receiptHeader := w.Header().Get(HeaderPaymentResponse)
	if receiptHeader == "" {
		t.Error("expected X-Payment-Response header")
	}

	var receipt SettlementResponse
	if err := DecodeHeader(receiptHeader, &receipt); err != nil {
		t.Fatalf("decode receipt: %v", err)
	}
	if !receipt.Success {
		t.Error("expected success receipt")
	}
	if receipt.Transaction != "0xdeadbeef" {
		t.Errorf("tx: want 0xdeadbeef, got %s", receipt.Transaction)
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

func TestMiddleware_InvalidPaymentHeader_Returns400(t *testing.T) {
	router, _ := setupMiddlewareTest(enabledPricing(), "", "", nil)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	req.Header.Set(HeaderPayment, "not-valid-base64!!!")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("status: want 400, got %d", w.Code)
	}
}

func TestMiddleware_DisabledConfig_PassesThrough(t *testing.T) {
	pricing := &PricingConfig{Enabled: false}
	router, _ := setupMiddlewareTest(pricing, "", "", nil)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200 (passthrough), got %d", w.Code)
	}
}

func TestMiddleware_UnpricedRoute_PassesThrough(t *testing.T) {
	router, _ := setupMiddlewareTest(enabledPricing(), "", "", nil)

	req := httptest.NewRequest(http.MethodGet, "/api/v1/health", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}
}

func TestMiddleware_SettlementFails_ReturnsError(t *testing.T) {
	router, mock := setupMiddlewareTest(enabledPricing(), "0xPayer", "", context.DeadlineExceeded)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", http.NoBody)
	req.Header.Set(HeaderPayment, makePaymentHeader(t, "0xfail1"))
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code == http.StatusOK {
		t.Error("expected non-200 when settlement fails")
	}

	hasFailed := false
	for _, evt := range mock.events {
		if evt.EventType == EventPaymentFailed {
			hasFailed = true
			break
		}
	}
	if !hasFailed {
		t.Error("expected payment.failed event")
	}
}
