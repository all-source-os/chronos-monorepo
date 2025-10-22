package http

import (
	"net/http"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/gin-gonic/gin"
)

// TenantHandler handles tenant-related HTTP requests
type TenantHandler struct {
	createTenantUC *usecases.CreateTenantUseCase
}

// NewTenantHandler creates a new TenantHandler
func NewTenantHandler(createTenantUC *usecases.CreateTenantUseCase) *TenantHandler {
	return &TenantHandler{
		createTenantUC: createTenantUC,
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
		if err == domain.ErrTenantAlreadyExists {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}
