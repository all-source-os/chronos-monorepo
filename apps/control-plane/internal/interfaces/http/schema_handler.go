package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/application/usecases"
)

// SchemaHandler handles schema governance HTTP requests
type SchemaHandler struct {
	registerSchemaUC *usecases.RegisterSchemaUseCase
	listSchemasUC    *usecases.ListSchemasUseCase
	validateEventUC  *usecases.ValidateEventUseCase
}

// NewSchemaHandler creates a new SchemaHandler
func NewSchemaHandler(
	registerSchemaUC *usecases.RegisterSchemaUseCase,
	listSchemasUC *usecases.ListSchemasUseCase,
	validateEventUC *usecases.ValidateEventUseCase,
) *SchemaHandler {
	return &SchemaHandler{
		registerSchemaUC: registerSchemaUC,
		listSchemasUC:    listSchemasUC,
		validateEventUC:  validateEventUC,
	}
}

// Register handles POST /api/v1/schemas
func (h *SchemaHandler) Register(c *gin.Context) {
	var req dto.SchemaGovernanceRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.registerSchemaUC.Execute(c.Request.Context(), req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusCreated, resp)
}

// List handles GET /api/v1/schemas
func (h *SchemaHandler) List(c *gin.Context) {
	resp, err := h.listSchemasUC.Execute(c.Request.Context())
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"schemas": resp, "total": len(resp)})
}

// Validate handles POST /api/v1/schemas/validate
func (h *SchemaHandler) Validate(c *gin.Context) {
	var req dto.ValidateEventRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	resp, err := h.validateEventUC.Execute(c.Request.Context(), req)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}
