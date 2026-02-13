// Package http provides HTTP handlers for the control plane API.
package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// PolicyHandler handles policy-related HTTP requests
type PolicyHandler struct {
	evaluatePolicyUC *usecases.EvaluatePolicyUseCase
	createPolicyUC   *usecases.CreatePolicyUseCase
	getPolicyUC      *usecases.GetPolicyUseCase
	listPoliciesUC   *usecases.ListPoliciesUseCase
	updatePolicyUC   *usecases.UpdatePolicyUseCase
	deletePolicyUC   *usecases.DeletePolicyUseCase
}

// NewPolicyHandler creates a new PolicyHandler
func NewPolicyHandler(
	evaluatePolicyUC *usecases.EvaluatePolicyUseCase,
	createPolicyUC *usecases.CreatePolicyUseCase,
	getPolicyUC *usecases.GetPolicyUseCase,
	listPoliciesUC *usecases.ListPoliciesUseCase,
	updatePolicyUC *usecases.UpdatePolicyUseCase,
	deletePolicyUC *usecases.DeletePolicyUseCase,
) *PolicyHandler {
	return &PolicyHandler{
		evaluatePolicyUC: evaluatePolicyUC,
		createPolicyUC:   createPolicyUC,
		getPolicyUC:      getPolicyUC,
		listPoliciesUC:   listPoliciesUC,
		updatePolicyUC:   updatePolicyUC,
		deletePolicyUC:   deletePolicyUC,
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

// Create handles POST /api/v1/policies
func (h *PolicyHandler) Create(c *gin.Context) {
	var req dto.CreatePolicyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.createPolicyUC.Execute(req)
	if err != nil {
		if errors.Is(err, domain.ErrPolicyAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// Get handles GET /api/v1/policies/:id
func (h *PolicyHandler) Get(c *gin.Context) {
	id := c.Param("id")

	resp, err := h.getPolicyUC.Execute(id)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// List handles GET /api/v1/policies
func (h *PolicyHandler) List(c *gin.Context) {
	resource := c.Query("resource")

	resp, err := h.listPoliciesUC.Execute(resource)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"policies": resp, "total": len(resp)})
}

// Update handles PUT /api/v1/policies/:id
func (h *PolicyHandler) Update(c *gin.Context) {
	id := c.Param("id")

	var req dto.UpdatePolicyRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.updatePolicyUC.Execute(id, req)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Delete handles DELETE /api/v1/policies/:id
func (h *PolicyHandler) Delete(c *gin.Context) {
	id := c.Param("id")

	err := h.deletePolicyUC.Execute(id)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "policy deleted"})
}

// Enable handles POST /api/v1/policies/:id/enable
func (h *PolicyHandler) Enable(c *gin.Context) {
	id := c.Param("id")

	enabled := true
	req := dto.UpdatePolicyRequest{Enabled: &enabled}

	resp, err := h.updatePolicyUC.Execute(id, req)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Disable handles POST /api/v1/policies/:id/disable
func (h *PolicyHandler) Disable(c *gin.Context) {
	id := c.Param("id")

	enabled := false
	req := dto.UpdatePolicyRequest{Enabled: &enabled}

	resp, err := h.updatePolicyUC.Execute(id, req)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// handleDomainError maps domain errors to HTTP status codes.
func (h *PolicyHandler) handleDomainError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrPolicyNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrForbidden):
		c.JSON(http.StatusForbidden, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
