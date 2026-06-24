package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

// With DEMO_ENABLED unset (the default), POST /api/v1/demo/start must return 403
// and create NOTHING. The gate runs before any Core/registration call, so an
// empty ControlPlane proves the early-return path: if a tenant were created we'd
// have to reach cp.client (nil here) and panic — the absence of a panic plus the
// 403 is the "creates nothing" assertion. This is the gap-1 recurrence guard:
// a stray status probe hitting /demo/start can never mint a tenant again.
func TestDemoStart_DisabledByDefault(t *testing.T) {
	t.Setenv("DEMO_ENABLED", "") // explicit: unset/empty => disabled

	gin.SetMode(gin.TestMode)
	r := gin.New()
	cp := &ControlPlane{} // no client wired — any tenant-creation path would panic
	r.POST("/api/v1/demo/start", cp.DemoStartHandler)

	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/demo/start", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusForbidden {
		t.Fatalf("DEMO_ENABLED unset: expected 403, got %d (%s)", w.Code, w.Body.String())
	}
	if !strings.Contains(w.Body.String(), "demo_disabled") {
		t.Errorf("expected demo_disabled error body, got %s", w.Body.String())
	}
}

// demoEnabled honors true/1/yes (case-insensitive, trimmed) and treats anything
// else — including unset — as OFF.
func TestDemoEnabled_Parsing(t *testing.T) {
	cases := map[string]bool{
		"":        false,
		"false":   false,
		"0":       false,
		"no":      false,
		"off":     false,
		"true":    true,
		"TRUE":    true,
		"  true ": true,
		"1":       true,
		"yes":     true,
		"YES":     true,
	}
	for in, want := range cases {
		t.Setenv("DEMO_ENABLED", in)
		if got := demoEnabled(); got != want {
			t.Errorf("demoEnabled(%q) = %v, want %v", in, got, want)
		}
	}
}
