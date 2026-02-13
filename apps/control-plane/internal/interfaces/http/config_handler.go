package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// ConfigHandler handles configuration HTTP requests
type ConfigHandler struct {
	createConfigUC *usecases.CreateConfigUseCase
	getConfigUC    *usecases.GetConfigUseCase
	listConfigsUC  *usecases.ListConfigsUseCase
	updateConfigUC *usecases.UpdateConfigUseCase
	deleteConfigUC *usecases.DeleteConfigUseCase
}

// NewConfigHandler creates a new ConfigHandler
func NewConfigHandler(
	createConfigUC *usecases.CreateConfigUseCase,
	getConfigUC *usecases.GetConfigUseCase,
	listConfigsUC *usecases.ListConfigsUseCase,
	updateConfigUC *usecases.UpdateConfigUseCase,
	deleteConfigUC *usecases.DeleteConfigUseCase,
) *ConfigHandler {
	return &ConfigHandler{
		createConfigUC: createConfigUC,
		getConfigUC:    getConfigUC,
		listConfigsUC:  listConfigsUC,
		updateConfigUC: updateConfigUC,
		deleteConfigUC: deleteConfigUC,
	}
}

// Create handles POST /api/v1/config
func (h *ConfigHandler) Create(c *gin.Context) {
	var req dto.CreateConfigRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	updatedBy := h.extractUserID(c)
	resp, err := h.createConfigUC.Execute(req, updatedBy)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// Get handles GET /api/v1/config/:key
func (h *ConfigHandler) Get(c *gin.Context) {
	key := c.Param("key")

	resp, err := h.getConfigUC.Execute(key)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// List handles GET /api/v1/config
func (h *ConfigHandler) List(c *gin.Context) {
	category := c.Query("category")

	resp, err := h.listConfigsUC.Execute(category)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"configs": resp, "total": len(resp)})
}

// Update handles PUT /api/v1/config/:key
func (h *ConfigHandler) Update(c *gin.Context) {
	key := c.Param("key")

	var req dto.UpdateConfigRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	updatedBy := h.extractUserID(c)
	resp, err := h.updateConfigUC.Execute(key, req, updatedBy)
	if err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Delete handles DELETE /api/v1/config/:key
func (h *ConfigHandler) Delete(c *gin.Context) {
	key := c.Param("key")

	if err := h.deleteConfigUC.Execute(key); err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "config entry deleted"})
}

// extractUserID retrieves the caller's user ID from the gin context.
func (h *ConfigHandler) extractUserID(c *gin.Context) string {
	userID, exists := c.Get("auth_user_id")
	if !exists {
		return ""
	}
	if id, ok := userID.(string); ok {
		return id
	}
	return ""
}

// handleDomainError maps domain errors to HTTP status codes.
func (h *ConfigHandler) handleDomainError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrConfigNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
