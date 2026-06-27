package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/x402"
)

// extractionStubCore stubs GetTenant so a test can drive the extraction gate's
// verdict through tenant metadata. Embedding the interface means only GetTenant
// is implemented; HasExtractionQuota exercises no other CoreClient method.
type extractionStubCore struct {
	clients.CoreClient
	tenant *clients.TenantResponse
}

func (m *extractionStubCore) GetTenant(_ context.Context, _ string) (*clients.TenantResponse, error) {
	return m.tenant, nil
}

// extractionTenant builds a tenant whose "quotas" metadata carries the
// extraction allowance + meter the gate reads.
func extractionTenant(quota, used float64) *clients.TenantResponse {
	return &clients.TenantResponse{
		ID: "tenant-real",
		Metadata: map[string]any{
			"quotas": map[string]any{
				"extraction_tokens_quota": quota,
				"extraction_tokens_used":  used,
			},
		},
	}
}

func TestProxyExtraction_NoTenant401(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/extraction/chat/completions", strings.NewReader(`{"model":"x"}`))
	w := callHandler(cp.ProxyExtraction, req, "")
	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: got %d, want 401", w.Code)
	}
}

// The hard gate: a tenant at/over its extraction allowance is blocked with 402
// and the request must never reach an LLM — no spend before the gate.
func TestProxyExtraction_OverQuota402_DoesNotHitLLM(t *testing.T) {
	hit := false
	_, srv := newFakeBackend(func(w http.ResponseWriter, _ *http.Request) {
		hit = true
		w.WriteHeader(http.StatusOK)
	})
	defer srv.Close()
	t.Setenv("EXTRACTION_LLM_URL", srv.URL+"/v1/chat/completions")
	t.Setenv("EXTRACTION_LLM_API_KEY", "sk-provider-secret")

	cp := newTestCP(t, "http://unused", "http://unused")
	// used == quota → blocked.
	cp.extractionGate = x402.NewCoreQuotaChecker(&extractionStubCore{tenant: extractionTenant(100, 100)})

	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/extraction/chat/completions", strings.NewReader(`{"model":"x","messages":[]}`))
	w := callHandler(cp.ProxyExtraction, req, "tenant-real")

	if w.Code != http.StatusPaymentRequired {
		t.Fatalf("status: got %d, want 402 (body=%s)", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), "extraction_quota_exceeded") {
		t.Errorf("body: got %q, want extraction_quota_exceeded", w.Body.String())
	}
	if hit {
		t.Error("LLM upstream was hit despite 402 gate — extraction must be blocked BEFORE any spend")
	}
}

// Under quota: forward the OpenAI request to AllSource's server-side LLM, copy
// the response back, and replace the tenant's inbound bearer with the provider
// key so the provider secret never reaches the tenant and the tenant's key never
// reaches the provider.
func TestProxyExtraction_UnderQuota_ForwardsAndHidesProviderKey(t *testing.T) {
	backend, srv := newFakeBackend(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"choices":[{"message":{"content":"{}"}}],"usage":{"total_tokens":42}}`)) //nolint:errcheck // test response
	})
	defer srv.Close()
	t.Setenv("EXTRACTION_LLM_URL", srv.URL+"/v1/chat/completions")
	t.Setenv("EXTRACTION_LLM_API_KEY", "sk-provider-secret")

	cp := newTestCP(t, "http://unused", "http://unused")
	// Well under allowance → allowed.
	cp.extractionGate = x402.NewCoreQuotaChecker(&extractionStubCore{tenant: extractionTenant(1_000_000, 10)})

	reqBody := `{"model":"gpt-4o-mini","messages":[{"role":"user","content":"hi"}]}`
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/extraction/chat/completions", strings.NewReader(reqBody))
	// The tenant's ask_ key arrives as the inbound bearer (PRIME_LLM_API_KEY). It
	// must NOT be what gets forwarded upstream.
	req.Header.Set("Authorization", "Bearer ask_tenant_secret")
	w := callHandler(cp.ProxyExtraction, req, "tenant-real")

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if backend.path != "/v1/chat/completions" {
		t.Errorf("upstream path: got %q, want /v1/chat/completions", backend.path)
	}
	if backend.auth != "Bearer sk-provider-secret" {
		t.Errorf("upstream auth: got %q, want server-side provider key", backend.auth)
	}
	if strings.Contains(backend.auth, "ask_tenant_secret") {
		t.Error("tenant ask_ key leaked to the upstream LLM — provider key must replace it")
	}
	if !strings.Contains(string(backend.body), `"gpt-4o-mini"`) {
		t.Errorf("upstream body: got %q, want the forwarded chat-completions request", string(backend.body))
	}
	if !strings.Contains(w.Body.String(), `"total_tokens":42`) {
		t.Errorf("response copy: got %q, want upstream JSON copied back", w.Body.String())
	}
}

// Hosted extraction is opt-in infra: with EXTRACTION_LLM_URL unset the route
// 503s (the tenant should BYO instead), even when the gate would allow.
func TestProxyExtraction_Unconfigured503(t *testing.T) {
	t.Setenv("EXTRACTION_LLM_URL", "")
	cp := newTestCP(t, "http://unused", "http://unused") // nil gate → gate skipped (fail open)
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/extraction/chat/completions", strings.NewReader(`{"model":"x"}`))
	w := callHandler(cp.ProxyExtraction, req, "tenant-real")
	if w.Code != http.StatusServiceUnavailable {
		t.Fatalf("status: got %d, want 503 (body=%s)", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), "hosted_extraction_unconfigured") {
		t.Errorf("body: got %q, want hosted_extraction_unconfigured", w.Body.String())
	}
}
