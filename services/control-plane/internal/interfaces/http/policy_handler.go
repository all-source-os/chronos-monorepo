package http

import (
	"net/http"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/gin-gonic/gin"
)

// PolicyHandler handles policy-related HTTP requests
type PolicyHandler struct {
	evaluatePolicyUC *usecases.EvaluatePolicyUseCase
}

// NewPolicyHandler creates a new PolicyHandler
func NewPolicyHandler(evaluatePolicyUC *usecases.EvaluatePolicyUseCase) *PolicyHandler {
	return &PolicyHandler{
		evaluatePolicyUC: evaluatePolicyUC,
	}
}

// Evaluate handles POST /api/v1/policies/evaluate
func (h *PolicyHandler) Evaluate(c *gin.Context) {
	var req dto.EvaluatePolicyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.evaluatePolicyUC.Execute(req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}
