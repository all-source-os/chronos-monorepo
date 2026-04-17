package http

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func setupIPRuleHandler(t *testing.T) *IPRuleHandler {
	t.Helper()
	repo := persistence.NewMemoryIPRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := usecases.NewCreateIPRuleUseCase(repo, auditRepo)
	listUC := usecases.NewListIPRulesUseCase(repo)
	deleteUC := usecases.NewDeleteIPRuleUseCase(repo, auditRepo)
	return NewIPRuleHandler(createUC, listUC, deleteUC)
}

func TestIPRuleHandler_CreateAndList(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	// Create an IP rule
	body := `{"cidr":"192.168.1.0/24","type":"allow","description":"Office network"}`
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "POST", "/api/v1/admin/security/ip-rules", bytes.NewBufferString(body))
	c.Request.Header.Set("Content-Type", "application/json")
	handler.Create(c)

	if w.Code != http.StatusCreated {
		t.Fatalf("Create() status = %d, want %d; body = %s", w.Code, http.StatusCreated, w.Body.String())
	}

	var createResp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &createResp); err != nil {
		t.Fatalf("failed to parse create response: %v", err)
	}
	if createResp["cidr"] != "192.168.1.0/24" {
		t.Errorf("cidr = %v, want 192.168.1.0/24", createResp["cidr"])
	}
	if createResp["type"] != "allow" {
		t.Errorf("type = %v, want allow", createResp["type"])
	}

	// List IP rules
	w = httptest.NewRecorder()
	c, _ = gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "GET", "/api/v1/admin/security/ip-rules", http.NoBody)
	handler.List(c)

	if w.Code != http.StatusOK {
		t.Fatalf("List() status = %d, want %d", w.Code, http.StatusOK)
	}

	var listResp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &listResp); err != nil {
		t.Fatalf("failed to parse list response: %v", err)
	}
	total := listResp["total"].(float64) //nolint:errcheck,forcetypeassert
	if total != 1 {
		t.Errorf("total = %v, want 1", total)
	}
}

func TestIPRuleHandler_Create_InvalidCIDR(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	body := `{"cidr":"not-a-cidr","type":"allow","description":"Bad CIDR"}`
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "POST", "/api/v1/admin/security/ip-rules", bytes.NewBufferString(body))
	c.Request.Header.Set("Content-Type", "application/json")
	handler.Create(c)

	if w.Code != http.StatusBadRequest {
		t.Errorf("Create() with invalid CIDR status = %d, want %d; body = %s", w.Code, http.StatusBadRequest, w.Body.String())
	}
}

func TestIPRuleHandler_Create_InvalidType(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	body := `{"cidr":"10.0.0.0/8","type":"invalid","description":"Bad type"}`
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "POST", "/api/v1/admin/security/ip-rules", bytes.NewBufferString(body))
	c.Request.Header.Set("Content-Type", "application/json")
	handler.Create(c)

	if w.Code != http.StatusBadRequest {
		t.Errorf("Create() with invalid type status = %d, want %d; body = %s", w.Code, http.StatusBadRequest, w.Body.String())
	}
}

func TestIPRuleHandler_Create_MissingFields(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	body := `{"description":"Missing required fields"}`
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "POST", "/api/v1/admin/security/ip-rules", bytes.NewBufferString(body))
	c.Request.Header.Set("Content-Type", "application/json")
	handler.Create(c)

	if w.Code != http.StatusBadRequest {
		t.Errorf("Create() with missing fields status = %d, want %d", w.Code, http.StatusBadRequest)
	}
}

func TestIPRuleHandler_Delete(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	// Create first
	body := `{"cidr":"10.0.0.0/8","type":"block","description":"Block internal"}`
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "POST", "/api/v1/admin/security/ip-rules", bytes.NewBufferString(body))
	c.Request.Header.Set("Content-Type", "application/json")
	handler.Create(c)

	var createResp map[string]interface{}
	_ = json.Unmarshal(w.Body.Bytes(), &createResp) //nolint:errcheck
	ruleID := createResp["id"].(string)             //nolint:errcheck,forcetypeassert

	// Delete it
	w = httptest.NewRecorder()
	c, _ = gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "DELETE", "/api/v1/admin/security/ip-rules/"+ruleID, http.NoBody)
	c.Params = gin.Params{{Key: "id", Value: ruleID}}
	handler.Delete(c)

	if w.Code != http.StatusOK {
		t.Errorf("Delete() status = %d, want %d; body = %s", w.Code, http.StatusOK, w.Body.String())
	}

	// Verify list is empty
	w = httptest.NewRecorder()
	c, _ = gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "GET", "/api/v1/admin/security/ip-rules", http.NoBody)
	handler.List(c)

	var listResp map[string]interface{}
	_ = json.Unmarshal(w.Body.Bytes(), &listResp) //nolint:errcheck
	total := listResp["total"].(float64)          //nolint:errcheck,forcetypeassert
	if total != 0 {
		t.Errorf("total after delete = %v, want 0", total)
	}
}

func TestIPRuleHandler_Delete_NotFound(t *testing.T) {
	gin.SetMode(gin.TestMode)
	handler := setupIPRuleHandler(t)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), "DELETE", "/api/v1/admin/security/ip-rules/nonexistent", http.NoBody)
	c.Params = gin.Params{{Key: "id", Value: "nonexistent"}}
	handler.Delete(c)

	if w.Code != http.StatusNotFound {
		t.Errorf("Delete() not found status = %d, want %d", w.Code, http.StatusNotFound)
	}
}
