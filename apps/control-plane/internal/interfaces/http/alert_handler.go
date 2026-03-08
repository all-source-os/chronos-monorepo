package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// AlertHandler handles alert rule HTTP requests
type AlertHandler struct {
	createAlertRuleUC *usecases.CreateAlertRuleUseCase
	listAlertRulesUC  *usecases.ListAlertRulesUseCase
	updateAlertRuleUC *usecases.UpdateAlertRuleUseCase
	deleteAlertRuleUC *usecases.DeleteAlertRuleUseCase
}

// NewAlertHandler creates a new AlertHandler
func NewAlertHandler(
	createAlertRuleUC *usecases.CreateAlertRuleUseCase,
	listAlertRulesUC *usecases.ListAlertRulesUseCase,
	updateAlertRuleUC *usecases.UpdateAlertRuleUseCase,
	deleteAlertRuleUC *usecases.DeleteAlertRuleUseCase,
) *AlertHandler {
	return &AlertHandler{
		createAlertRuleUC: createAlertRuleUC,
		listAlertRulesUC:  listAlertRulesUC,
		updateAlertRuleUC: updateAlertRuleUC,
		deleteAlertRuleUC: deleteAlertRuleUC,
	}
}

// Create handles POST /api/v1/admin/alerts
func (h *AlertHandler) Create(c *gin.Context) {
	var req dto.CreateAlertRuleRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.createAlertRuleUC.Execute(req)
	if err != nil {
		if errors.Is(err, domain.ErrAlertRuleAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// List handles GET /api/v1/admin/alerts
func (h *AlertHandler) List(c *gin.Context) {
	resp, err := h.listAlertRulesUC.Execute()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"alerts": resp, "total": len(resp)})
}

// Update handles PUT /api/v1/admin/alerts/:id
func (h *AlertHandler) Update(c *gin.Context) {
	id := c.Param("id")

	var req dto.UpdateAlertRuleRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.updateAlertRuleUC.Execute(id, req)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Delete handles DELETE /api/v1/admin/alerts/:id
func (h *AlertHandler) Delete(c *gin.Context) {
	id := c.Param("id")

	if err := h.deleteAlertRuleUC.Execute(id); err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "alert rule deleted"})
}

// handleDomainError maps domain errors to HTTP status codes.
func (h *AlertHandler) handleDomainError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrAlertRuleNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
