// Package http provides HTTP handlers for the control plane API.
package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// TenantHandler handles tenant-related HTTP requests
type TenantHandler struct {
	createTenantUC   *usecases.CreateTenantUseCase
	getTenantUC      *usecases.GetTenantUseCase
	listTenantsUC    *usecases.ListTenantsUseCase
	updateTenantUC   *usecases.UpdateTenantUseCase
	suspendTenantUC  *usecases.SuspendTenantUseCase
	activateTenantUC *usecases.ActivateTenantUseCase
	deleteTenantUC   *usecases.DeleteTenantUseCase
	coreClient       clients.CoreClient
}

// NewTenantHandler creates a new TenantHandler
func NewTenantHandler(
	createTenantUC *usecases.CreateTenantUseCase,
	getTenantUC *usecases.GetTenantUseCase,
	listTenantsUC *usecases.ListTenantsUseCase,
	updateTenantUC *usecases.UpdateTenantUseCase,
	suspendTenantUC *usecases.SuspendTenantUseCase,
	activateTenantUC *usecases.ActivateTenantUseCase,
	deleteTenantUC *usecases.DeleteTenantUseCase,
	coreClient clients.CoreClient,
) *TenantHandler {
	return &TenantHandler{
		createTenantUC:   createTenantUC,
		getTenantUC:      getTenantUC,
		listTenantsUC:    listTenantsUC,
		updateTenantUC:   updateTenantUC,
		suspendTenantUC:  suspendTenantUC,
		activateTenantUC: activateTenantUC,
		deleteTenantUC:   deleteTenantUC,
		coreClient:       coreClient,
	}
}

// Create handles POST /api/v1/tenants
func (h *TenantHandler) Create(c *gin.Context) {
	var req dto.CreateTenantRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.createTenantUC.Execute(req)
	if err != nil {
		if errors.Is(err, domain.ErrTenantAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// List handles GET /api/v1/tenants
func (h *TenantHandler) List(c *gin.Context) {
	offset, _ := strconv.Atoi(c.DefaultQuery("offset", "0")) //nolint:errcheck // default is valid int
	limit, _ := strconv.Atoi(c.DefaultQuery("limit", "0"))   //nolint:errcheck // default is valid int
	status := c.Query("status")

	req := usecases.ListTenantsRequest{
		Offset: offset,
		Limit:  limit,
		Status: status,
	}

	resp, err := h.listTenantsUC.Execute(req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Get handles GET /api/v1/tenants/:id
func (h *TenantHandler) Get(c *gin.Context) {
	id := c.Param("id")

	resp, err := h.getTenantUC.Execute(id)
	if err != nil {
		if errors.Is(err, domain.ErrTenantNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Update handles PUT /api/v1/tenants/:id
func (h *TenantHandler) Update(c *gin.Context) {
	id := c.Param("id")

	var req dto.UpdateTenantRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.updateTenantUC.Execute(id, req)
	if err != nil {
		if errors.Is(err, domain.ErrTenantNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Suspend handles POST /api/v1/tenants/:id/suspend
func (h *TenantHandler) Suspend(c *gin.Context) {
	id := c.Param("id")
	role := h.extractRole(c)

	resp, err := h.suspendTenantUC.Execute(c.Request.Context(), id, role)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Activate handles POST /api/v1/tenants/:id/activate
func (h *TenantHandler) Activate(c *gin.Context) {
	id := c.Param("id")
	role := h.extractRole(c)

	resp, err := h.activateTenantUC.Execute(c.Request.Context(), id, role)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Delete handles DELETE /api/v1/tenants/:id
func (h *TenantHandler) Delete(c *gin.Context) {
	id := c.Param("id")
	role := h.extractRole(c)

	err := h.deleteTenantUC.Execute(c.Request.Context(), id, role)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "tenant deleted"})
}

// Stats handles GET /api/v1/tenants/:id/stats
func (h *TenantHandler) Stats(c *gin.Context) {
	id := c.Param("id")

	if h.coreClient == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "core service not configured"})
		return
	}

	stats, err := h.coreClient.GetTenantStats(c.Request.Context(), id)
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to fetch tenant stats from core"})
		return
	}

	c.JSON(http.StatusOK, dto.TenantStatsResponse{
		TenantID:     stats.TenantID,
		EventCount:   stats.EventCount,
		StreamCount:  stats.StreamCount,
		StorageBytes: stats.StorageUsed,
	})
}

// extractRole retrieves the caller's role from the gin context.
// The auth middleware stores it under "auth_role" for cross-package access.
func (h *TenantHandler) extractRole(c *gin.Context) entities.Role {
	role, exists := c.Get("auth_role")
	if !exists {
		return ""
	}
	if r, ok := role.(entities.Role); ok {
		return r
	}
	return ""
}

// handleDomainError maps domain errors to HTTP status codes.
func (h *TenantHandler) handleDomainError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrTenantNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrForbidden):
		c.JSON(http.StatusForbidden, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
