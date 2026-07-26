package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
)

// doCORS runs a single request through corsMiddleware and returns the recorder.
func doCORS(t *testing.T, allowed map[string]struct{}, method, origin string) *httptest.ResponseRecorder {
	t.Helper()
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(corsMiddleware(allowed))
	r.GET("/x", func(c *gin.Context) { c.Status(http.StatusOK) })

	req := httptest.NewRequestWithContext(context.Background(), method, "/x", http.NoBody)
	if origin != "" {
		req.Header.Set("Origin", origin)
	}
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

// An allowlisted origin gets its Origin echoed WITH credentials — the only case
// where cookies / Authorization are usable cross-site.
func TestCORS_AllowlistedOriginGetsCredentials(t *testing.T) {
	allowed := map[string]struct{}{"https://admin.all-source.xyz": {}}
	w := doCORS(t, allowed, http.MethodGet, "https://admin.all-source.xyz")

	if got := w.Header().Get("Access-Control-Allow-Origin"); got != "https://admin.all-source.xyz" {
		t.Fatalf("Allow-Origin = %q, want the echoed origin", got)
	}
	if got := w.Header().Get("Access-Control-Allow-Credentials"); got != "true" {
		t.Fatalf("Allow-Credentials = %q, want true", got)
	}
	if got := w.Header().Get("Vary"); got != "Origin" {
		t.Fatalf("Vary = %q, want Origin", got)
	}
}

// A non-allowlisted origin must NEVER receive a credentialed grant. It may read
// public endpoints under "*", but the browser will refuse to send cookies or
// expose a credentialed response. This is the security boundary the change adds.
func TestCORS_UnknownOriginNeverGetsCredentials(t *testing.T) {
	allowed := map[string]struct{}{"https://admin.all-source.xyz": {}}
	w := doCORS(t, allowed, http.MethodGet, "https://evil.example.com")

	if got := w.Header().Get("Access-Control-Allow-Origin"); got != "*" {
		t.Fatalf("Allow-Origin = %q, want * (public, non-credentialed)", got)
	}
	if got := w.Header().Get("Access-Control-Allow-Credentials"); got != "" {
		t.Fatalf("Allow-Credentials = %q, want empty for a non-allowlisted origin", got)
	}
}

// A preflight from an allowlisted origin short-circuits to 204 with the
// credentialed grant intact.
func TestCORS_PreflightAllowlisted(t *testing.T) {
	allowed := map[string]struct{}{"https://admin.all-source.xyz": {}}
	w := doCORS(t, allowed, http.MethodOptions, "https://admin.all-source.xyz")

	if w.Code != http.StatusNoContent {
		t.Fatalf("OPTIONS status = %d, want 204", w.Code)
	}
	if got := w.Header().Get("Access-Control-Allow-Credentials"); got != "true" {
		t.Fatalf("preflight Allow-Credentials = %q, want true", got)
	}
}

// allowedCORSOrigins is sourced from the frontend allowlist env, so adding the
// admin panel is config-only (ALLOWED_FRONTEND_URLS) — no code change.
func TestAllowedCORSOrigins_FromFrontendAllowlistEnv(t *testing.T) {
	t.Setenv("FRONTEND_URL", "https://www.all-source.xyz")
	t.Setenv("ALLOWED_FRONTEND_URLS", "https://admin.all-source.xyz, https://all-source.xyz/")

	set := allowedCORSOrigins()
	for _, want := range []string{
		"https://www.all-source.xyz",
		"https://admin.all-source.xyz",
		"https://all-source.xyz",
	} {
		if _, ok := set[want]; !ok {
			t.Errorf("allowlist missing %q (set: %v)", want, set)
		}
	}
	if _, ok := set["https://evil.example.com"]; ok {
		t.Error("allowlist must not contain an origin that was never configured")
	}
}
