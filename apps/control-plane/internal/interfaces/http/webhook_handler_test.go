package http //nolint:revive // package name intentionally matches directory

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func init() {
	gin.SetMode(gin.TestMode)
}

func signPayload(payload []byte, secret string) string {
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(payload)
	return hex.EncodeToString(mac.Sum(nil))
}

func setupWebhookHandler(t *testing.T) (*WebhookHandler, *gin.Engine) {
	t.Helper()

	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()

	// Create a tenant to work with
	if err := tenantRepo.Save(&entities.Tenant{
		ID:        "tenant-123",
		Name:      "Test Tenant",
		Status:    entities.TenantStatusActive,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}

	updateSubUC := usecases.NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	suspendUC := usecases.NewSuspendTenantUseCase(tenantRepo, auditRepo)
	processUC := usecases.NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC, nil)

	stripeProcessUC := usecases.NewProcessStripeWebhookUseCase(tenantRepo, auditRepo, updateSubUC, suspendUC)

	handler := NewWebhookHandler(processUC, stripeProcessUC)
	router := gin.New()
	router.POST("/api/v1/webhooks/lemonsqueezy", handler.LemonSqueezy)
	router.POST("/api/v1/webhooks/stripe", handler.Stripe)

	return handler, router
}

func TestWebhook_MissingSignature(t *testing.T) {
	_, router := setupWebhookHandler(t)

	body := `{"event_name":"subscription_created","data":{},"meta":{}}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/lemonsqueezy", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}

	var resp map[string]string
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal response: %v", err)
	}
	if resp["error"] != "missing X-Signature header" {
		t.Errorf("unexpected error: %s", resp["error"])
	}
}

func TestWebhook_InvalidSignature(t *testing.T) {
	_, router := setupWebhookHandler(t)

	body := `{"event_name":"subscription_created","data":{},"meta":{}}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/lemonsqueezy", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Signature", "invalid-signature")

	t.Setenv("LEMON_SQUEEZY_WEBHOOK_SECRET", "test-secret")

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}

	var resp map[string]string
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal response: %v", err)
	}
	if resp["error"] != "invalid signature" {
		t.Errorf("unexpected error: %s", resp["error"])
	}
}

func TestWebhook_MissingSecret(t *testing.T) {
	_, router := setupWebhookHandler(t)

	body := `{"event_name":"subscription_created","data":{},"meta":{}}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/lemonsqueezy", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Signature", "some-sig")

	t.Setenv("LEMON_SQUEEZY_WEBHOOK_SECRET", "")

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusInternalServerError {
		t.Errorf("expected 500, got %d", w.Code)
	}
}

func TestWebhook_ValidSignature_SubscriptionCreated(t *testing.T) {
	_, router := setupWebhookHandler(t)

	secret := "test-webhook-secret"
	t.Setenv("LEMON_SQUEEZY_WEBHOOK_SECRET", secret)

	event := map[string]interface{}{
		"event_name": "subscription_created",
		"data": map[string]interface{}{
			"id":   "sub-456",
			"type": "subscriptions",
			"attributes": map[string]interface{}{
				"store_id":     1,
				"customer_id":  789,
				"status":       "active",
				"variant_name": "Pro",
			},
		},
		"meta": map[string]interface{}{
			"custom_data": map[string]interface{}{
				"tenant_id": "tenant-123",
			},
		},
	}

	body, err := json.Marshal(event)
	if err != nil {
		t.Fatalf("marshal event: %v", err)
	}
	sig := signPayload(body, secret)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/lemonsqueezy", strings.NewReader(string(body)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Signature", sig)

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d: %s", w.Code, w.Body.String())
	}
}

func TestWebhook_ValidSignature_SubscriptionCancelled(t *testing.T) {
	_, router := setupWebhookHandler(t)

	secret := "test-webhook-secret"
	t.Setenv("LEMON_SQUEEZY_WEBHOOK_SECRET", secret)

	event := map[string]interface{}{
		"event_name": "subscription_cancelled", //nolint:misspell // LemonSqueezy API event name
		"data": map[string]interface{}{
			"id":   "sub-456",
			"type": "subscriptions",
			"attributes": map[string]interface{}{
				"customer_id":  789,
				"status":       "cancelled", //nolint:misspell // LemonSqueezy API status value
				"variant_name": "Pro",
			},
		},
		"meta": map[string]interface{}{
			"custom_data": map[string]interface{}{
				"tenant_id": "tenant-123",
			},
		},
	}

	body, err := json.Marshal(event)
	if err != nil {
		t.Fatalf("marshal event: %v", err)
	}
	sig := signPayload(body, secret)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/lemonsqueezy", strings.NewReader(string(body)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Signature", sig)

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d: %s", w.Code, w.Body.String())
	}
}

func stripeSignPayload(t *testing.T, payload []byte, secret, timestamp string) string {
	t.Helper()
	signedPayload := timestamp + "." + string(payload)
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(signedPayload))
	return hex.EncodeToString(mac.Sum(nil))
}

func TestStripeWebhook_MissingSignature(t *testing.T) {
	_, router := setupWebhookHandler(t)

	body := `{"id":"evt_1","type":"customer.subscription.created","data":{"object":{}}}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/stripe", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestStripeWebhook_InvalidSignature(t *testing.T) {
	_, router := setupWebhookHandler(t)

	t.Setenv("STRIPE_WEBHOOK_SECRET", "whsec_test")

	body := `{"id":"evt_1","type":"customer.subscription.created","data":{"object":{}}}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/stripe", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Stripe-Signature", "t=12345,v1=invalidsig")

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected 400, got %d", w.Code)
	}
}

func TestStripeWebhook_ValidSignature_SubscriptionCreated(t *testing.T) {
	_, router := setupWebhookHandler(t)

	secret := "whsec_test_secret"
	t.Setenv("STRIPE_WEBHOOK_SECRET", secret)

	event := map[string]interface{}{
		"id":   "evt_stripe_1",
		"type": "customer.subscription.created",
		"data": map[string]interface{}{
			"object": map[string]interface{}{
				"id":       "sub_stripe_1",
				"customer": "cus_stripe_1",
				"status":   "active",
				"metadata": map[string]interface{}{
					"tenant_id": "tenant-123",
					"tier":      "pro",
				},
			},
		},
	}

	body, err := json.Marshal(event)
	if err != nil {
		t.Fatalf("marshal event: %v", err)
	}

	timestamp := "1700000000"
	sig := stripeSignPayload(t, body, secret, timestamp)
	sigHeader := "t=" + timestamp + ",v1=" + sig

	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/stripe", strings.NewReader(string(body)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Stripe-Signature", sigHeader)

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d: %s", w.Code, w.Body.String())
	}
}

func TestStripeWebhook_ValidSignature_SubscriptionDeleted(t *testing.T) {
	_, router := setupWebhookHandler(t)

	secret := "whsec_test_secret"
	t.Setenv("STRIPE_WEBHOOK_SECRET", secret)

	event := map[string]interface{}{
		"id":   "evt_stripe_2",
		"type": "customer.subscription.deleted",
		"data": map[string]interface{}{
			"object": map[string]interface{}{
				"id":       "sub_stripe_2",
				"customer": "cus_stripe_2",
				"status":   "canceled",
				"metadata": map[string]interface{}{
					"tenant_id": "tenant-123",
					"tier":      "pro",
				},
			},
		},
	}

	body, err := json.Marshal(event)
	if err != nil {
		t.Fatalf("marshal event: %v", err)
	}

	timestamp := "1700000000"
	sig := stripeSignPayload(t, body, secret, timestamp)
	sigHeader := "t=" + timestamp + ",v1=" + sig

	req := httptest.NewRequest(http.MethodPost, "/api/v1/webhooks/stripe", strings.NewReader(string(body)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Stripe-Signature", sigHeader)

	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected 200, got %d: %s", w.Code, w.Body.String())
	}
}

func TestVerifyStripeSignature(t *testing.T) {
	secret := "whsec_test"
	payload := []byte(`{"event":"test"}`)
	timestamp := "1700000000"

	sig := stripeSignPayload(t, payload, secret, timestamp)
	sigHeader := "t=" + timestamp + ",v1=" + sig

	if !verifyStripeSignature(payload, sigHeader, secret) {
		t.Error("expected valid stripe signature to pass")
	}
	if verifyStripeSignature(payload, "t=123,v1=bad", secret) {
		t.Error("expected invalid stripe signature to fail")
	}
	if verifyStripeSignature(payload, sigHeader, "wrong-secret") {
		t.Error("expected wrong secret to fail")
	}
	if verifyStripeSignature(payload, "nosigformat", secret) {
		t.Error("expected malformed header to fail")
	}
}

func TestVerifyHMACSignature(t *testing.T) {
	secret := "my-secret-key"
	payload := []byte(`{"event":"test"}`)

	validSig := signPayload(payload, secret)

	if !verifyHMACSignature(payload, validSig, secret) {
		t.Error("expected valid signature to pass")
	}

	if verifyHMACSignature(payload, "bad-signature", secret) {
		t.Error("expected invalid signature to fail")
	}

	if verifyHMACSignature(payload, validSig, "wrong-secret") {
		t.Error("expected wrong secret to fail")
	}
}
