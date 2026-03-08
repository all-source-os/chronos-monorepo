package http //nolint:revive // package name intentionally matches directory

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// SLOHandler handles SLO HTTP requests
type SLOHandler struct {
	createSLOUC *usecases.CreateSLOUseCase
	listSLOsUC  *usecases.ListSLOsUseCase
	deleteSLOUC *usecases.DeleteSLOUseCase
}

// NewSLOHandler creates a new SLOHandler
func NewSLOHandler(
	createSLOUC *usecases.CreateSLOUseCase,
	listSLOsUC *usecases.ListSLOsUseCase,
	deleteSLOUC *usecases.DeleteSLOUseCase,
) *SLOHandler {
	return &SLOHandler{
		createSLOUC: createSLOUC,
		listSLOsUC:  listSLOsUC,
		deleteSLOUC: deleteSLOUC,
	}
}

// Create handles POST /api/v1/admin/slos
func (h *SLOHandler) Create(c *gin.Context) {
	var req dto.CreateSLORequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.createSLOUC.Execute(req)
	if err != nil {
		if errors.Is(err, domain.ErrSLOAlreadyExists) {
			c.JSON(http.StatusConflict, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// List handles GET /api/v1/admin/slos
func (h *SLOHandler) List(c *gin.Context) {
	resp, err := h.listSLOsUC.Execute()
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"slos": resp, "total": len(resp)})
}

// Delete handles DELETE /api/v1/admin/slos/:id
func (h *SLOHandler) Delete(c *gin.Context) {
	id := c.Param("id")

	if err := h.deleteSLOUC.Execute(id); err != nil {
		h.handleDomainError(c, err)
		return
	}

	c.JSON(http.StatusOK, gin.H{"message": "SLO deleted"})
}

// handleDomainError maps domain errors to HTTP status codes.
func (h *SLOHandler) handleDomainError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrSLONotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}
