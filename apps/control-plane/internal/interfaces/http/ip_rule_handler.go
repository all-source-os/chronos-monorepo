package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// IPRuleHandler handles IP rule HTTP requests
type IPRuleHandler struct {
	createIPRuleUC *usecases.CreateIPRuleUseCase
	listIPRulesUC  *usecases.ListIPRulesUseCase
	deleteIPRuleUC *usecases.DeleteIPRuleUseCase
}

// NewIPRuleHandler creates a new IPRuleHandler
func NewIPRuleHandler(
	createIPRuleUC *usecases.CreateIPRuleUseCase,
	listIPRulesUC *usecases.ListIPRulesUseCase,
	deleteIPRuleUC *usecases.DeleteIPRuleUseCase,
) *IPRuleHandler {
	return &IPRuleHandler{
		createIPRuleUC: createIPRuleUC,
		listIPRulesUC:  listIPRulesUC,
		deleteIPRuleUC: deleteIPRuleUC,
	}
}

// Create handles POST /api/v1/admin/security/ip-rules
func (h *IPRuleHandler) Create(c *gin.Context) {
	var req dto.CreateIPRuleRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.createIPRuleUC.Execute(req)
	if err != nil {
		if errors.Is(err, domain.ErrIPRuleAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// List handles GET /api/v1/admin/security/ip-rules
func (h *IPRuleHandler) List(c *gin.Context) {
	resp, err := h.listIPRulesUC.Execute()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"ip_rules": resp, "total": len(resp)})
}

// Delete handles DELETE /api/v1/admin/security/ip-rules/:id
func (h *IPRuleHandler) Delete(c *gin.Context) {
	id := c.Param("id")

	if err := h.deleteIPRuleUC.Execute(id); err != nil {
		if errors.Is(err, domain.ErrIPRuleNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "IP rule deleted"})
}
