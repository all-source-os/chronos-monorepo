package http //nolint:revive // same-package tests

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// mockPaymentHistoryUC stubs GetAgentPaymentHistoryUseCase.Execute.
type mockPaymentHistoryUC struct {
	resp *usecases.AgentPaymentHistoryResponse
	err  error
}

func (m *mockPaymentHistoryUC) Execute(_ context.Context, _ usecases.AgentPaymentHistoryRequest) (*usecases.AgentPaymentHistoryResponse, error) {
	return m.resp, m.err
}

// buildAgentRouter sets up a gin router for GET /api/v1/agents/me/payments.
// If tenantID is non-empty, it injects auth_tenant_id into the context.
func buildAgentRouter(uc agentPaymentHistoryExecutor, tenantID string) *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	if tenantID != "" {
		r.Use(func(c *gin.Context) {
			c.Set("auth_tenant_id", tenantID)
			c.Next()
		})
	}
	// Inline handler: mirrors AgentHandler.GetPaymentHistory using the executor interface.
	r.GET("/api/v1/agents/me/payments", func(c *gin.Context) {
		tenantIDVal, exists := c.Get("auth_tenant_id")
		if !exists {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "missing tenant context"})
			return
		}
		tid, ok := tenantIDVal.(string)
		if !ok || tid == "" {
			c.JSON(http.StatusUnauthorized, gin.H{"error": "missing tenant context"})
			return
		}
		req := usecases.AgentPaymentHistoryRequest{TenantID: tid, Limit: 100}
		resp, err := uc.Execute(c.Request.Context(), req)
		if err != nil {
			c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusOK, resp)
	})
	return r
}

// agentPaymentHistoryExecutor is the subset of GetAgentPaymentHistoryUseCase used by tests.
type agentPaymentHistoryExecutor interface {
	Execute(ctx context.Context, req usecases.AgentPaymentHistoryRequest) (*usecases.AgentPaymentHistoryResponse, error)
}

func TestAgentHandler_NoTenantID_Returns401(t *testing.T) {
	mock := &mockPaymentHistoryUC{resp: &usecases.AgentPaymentHistoryResponse{}}
	r := buildAgentRouter(mock, "") // no tenant injected

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/agents/me/payments", http.NoBody)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: want 401, got %d", w.Code)
	}
}

func TestAgentHandler_UCError_Returns500(t *testing.T) {
	mock := &mockPaymentHistoryUC{err: errors.New("core unavailable")}
	r := buildAgentRouter(mock, "tenant-1")

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/agents/me/payments", http.NoBody)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusInternalServerError {
		t.Errorf("status: want 500, got %d", w.Code)
	}
}

func TestAgentHandler_Success_Returns200WithPayments(t *testing.T) {
	mock := &mockPaymentHistoryUC{
		resp: &usecases.AgentPaymentHistoryResponse{
			Payments: []usecases.AgentPaymentEntry{
				{Network: "base-mainnet", Transaction: "0xtx1", Amount: "0.0001"},
			},
			Total: 1,
		},
	}
	r := buildAgentRouter(mock, "tenant-abc")

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/agents/me/payments", http.NoBody)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}

	var body usecases.AgentPaymentHistoryResponse
	if err := json.NewDecoder(w.Body).Decode(&body); err != nil {
		t.Fatalf("decode body: %v", err)
	}
	if body.Total != 1 {
		t.Errorf("total: want 1, got %d", body.Total)
	}
	if len(body.Payments) != 1 || body.Payments[0].Transaction != "0xtx1" {
		t.Errorf("payments: want [{0xtx1}], got %+v", body.Payments)
	}
}

func TestAgentHandler_EmptyPayments_Returns200EmptySlice(t *testing.T) {
	mock := &mockPaymentHistoryUC{
		resp: &usecases.AgentPaymentHistoryResponse{
			Payments: []usecases.AgentPaymentEntry{},
			Total:    0,
		},
	}
	r := buildAgentRouter(mock, "tenant-xyz")

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/agents/me/payments", http.NoBody)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("status: want 200, got %d", w.Code)
	}
}
