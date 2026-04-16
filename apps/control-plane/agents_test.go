package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/gin-gonic/gin"
)

// TestAgentEchoHandler_ReturnsEcho verifies the reference x402 endpoint
// echoes the posted payload and stamps the tenant id from the Gin context.
// The middleware chain (auth, x402 tier gate, autopay) is covered by unit
// tests in internal/infrastructure/x402 — this test only covers the
// handler's own behavior.
func TestAgentEchoHandler_ReturnsEcho(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.Use(func(c *gin.Context) {
		c.Set("tenant_id", "tenant-abc")
		c.Next()
	})
	cp := &ControlPlane{}
	router.POST("/api/v1/agent-echo", cp.AgentEchoHandler)

	body := strings.NewReader(`{"hello":"world","n":42}`)
	req := httptest.NewRequest(http.MethodPost, "/api/v1/agent-echo", body)
	req.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status: want 200, got %d", w.Code)
	}

	var resp map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal response: %v", err)
	}
	if resp["tenant_id"] != "tenant-abc" {
		t.Errorf("tenant_id: want tenant-abc, got %v", resp["tenant_id"])
	}
	echo, _ := resp["echo"].(map[string]any)
	if echo == nil || echo["hello"] != "world" {
		t.Errorf("echo: want {hello:world,...}, got %v", resp["echo"])
	}
	if _, ok := resp["ts"].(string); !ok {
		t.Errorf("ts: expected string, got %T", resp["ts"])
	}
}

// TestAgentEchoHandler_EmptyBody verifies an empty POST is accepted as a ping.
func TestAgentEchoHandler_EmptyBody(t *testing.T) {
	gin.SetMode(gin.TestMode)
	router := gin.New()
	router.Use(func(c *gin.Context) {
		c.Set("tenant_id", "tenant-ping")
		c.Next()
	})
	cp := &ControlPlane{}
	router.POST("/api/v1/agent-echo", cp.AgentEchoHandler)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/agent-echo", http.NoBody)
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Fatalf("status: want 200, got %d", w.Code)
	}
}
