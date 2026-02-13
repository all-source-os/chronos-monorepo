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

// OperationsHandler handles operations-related HTTP requests
type OperationsHandler struct {
	createSnapshotUC    *usecases.CreateSnapshotUseCase
	triggerCompactionUC *usecases.TriggerCompactionUseCase
	startReplayUC       *usecases.StartReplayUseCase
	getReplayProgressUC *usecases.GetReplayProgressUseCase
	cancelReplayUC      *usecases.CancelReplayUseCase
	listOperationsUC    *usecases.ListOperationsUseCase
	getClusterStatusUC  *usecases.GetClusterStatusUseCase
	coreClient          clients.CoreClient
}

// NewOperationsHandler creates a new OperationsHandler
func NewOperationsHandler(
	createSnapshotUC *usecases.CreateSnapshotUseCase,
	triggerCompactionUC *usecases.TriggerCompactionUseCase,
	startReplayUC *usecases.StartReplayUseCase,
	getReplayProgressUC *usecases.GetReplayProgressUseCase,
	cancelReplayUC *usecases.CancelReplayUseCase,
	listOperationsUC *usecases.ListOperationsUseCase,
	getClusterStatusUC *usecases.GetClusterStatusUseCase,
	coreClient clients.CoreClient,
) *OperationsHandler {
	return &OperationsHandler{
		createSnapshotUC:    createSnapshotUC,
		triggerCompactionUC: triggerCompactionUC,
		startReplayUC:       startReplayUC,
		getReplayProgressUC: getReplayProgressUC,
		cancelReplayUC:      cancelReplayUC,
		listOperationsUC:    listOperationsUC,
		getClusterStatusUC:  getClusterStatusUC,
		coreClient:          coreClient,
	}
}

// CreateSnapshot handles POST /api/v1/operations/snapshots
func (h *OperationsHandler) CreateSnapshot(c *gin.Context) {
	var req dto.CreateSnapshotRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	initiatedBy := h.extractUserID(c)
	op, err := h.createSnapshotUC.Execute(c.Request.Context(), req.TenantID, initiatedBy)
	if err != nil {
		h.handleOperationError(c, err)
		return
	}

	c.JSON(http.StatusOK, h.toOperationResponse(op))
}

// ListSnapshots handles GET /api/v1/operations/snapshots
func (h *OperationsHandler) ListSnapshots(c *gin.Context) {
	ops, err := h.listOperationsUC.Execute(usecases.ListOperationsRequest{
		Type: entities.OperationSnapshot,
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	responses := make([]dto.OperationResponse, len(ops))
	for i, op := range ops {
		responses[i] = h.toOperationResponse(op)
	}

	c.JSON(http.StatusOK, gin.H{"snapshots": responses, "total": len(responses)})
}

// TriggerCompaction handles POST /api/v1/operations/compaction
func (h *OperationsHandler) TriggerCompaction(c *gin.Context) {
	var req dto.TriggerCompactionRequest
	// Body is optional for compaction
	_ = c.ShouldBindJSON(&req) //nolint:errcheck // optional body

	initiatedBy := h.extractUserID(c)
	op, err := h.triggerCompactionUC.Execute(c.Request.Context(), req.Force, initiatedBy)
	if err != nil {
		h.handleOperationError(c, err)
		return
	}

	c.JSON(http.StatusOK, h.toOperationResponse(op))
}

// GetCompactionStats handles GET /api/v1/operations/compaction/stats
func (h *OperationsHandler) GetCompactionStats(c *gin.Context) {
	if h.coreClient == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "core service not configured"})
		return
	}

	stats, err := h.coreClient.GetCompactionStats(c.Request.Context())
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to fetch compaction stats from core"})
		return
	}

	c.JSON(http.StatusOK, stats)
}

// StartReplay handles POST /api/v1/operations/replay
func (h *OperationsHandler) StartReplay(c *gin.Context) {
	var req dto.StartReplayRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	initiatedBy := h.extractUserID(c)
	op, err := h.startReplayUC.Execute(c.Request.Context(), req.EntityID, req.AsOf, req.TenantID, initiatedBy)
	if err != nil {
		h.handleOperationError(c, err)
		return
	}

	c.JSON(http.StatusOK, h.toOperationResponse(op))
}

// GetReplayProgress handles GET /api/v1/operations/replay/:id
func (h *OperationsHandler) GetReplayProgress(c *gin.Context) {
	id := c.Param("id")

	op, err := h.getReplayProgressUC.Execute(c.Request.Context(), id)
	if err != nil {
		h.handleOperationError(c, err)
		return
	}

	c.JSON(http.StatusOK, h.toOperationResponse(op))
}

// CancelReplay handles POST /api/v1/operations/replay/:id/cancel
func (h *OperationsHandler) CancelReplay(c *gin.Context) {
	id := c.Param("id")

	op, err := h.cancelReplayUC.Execute(c.Request.Context(), id)
	if err != nil {
		h.handleOperationError(c, err)
		return
	}

	c.JSON(http.StatusOK, h.toOperationResponse(op))
}

// ListOperations handles GET /api/v1/operations/history
func (h *OperationsHandler) ListOperations(c *gin.Context) {
	limit, _ := strconv.Atoi(c.DefaultQuery("limit", "50"))
	opType := c.Query("type")
	status := c.Query("status")
	tenantID := c.Query("tenant_id")

	ops, err := h.listOperationsUC.Execute(usecases.ListOperationsRequest{
		Type:     entities.OperationType(opType),
		Status:   entities.OperationStatus(status),
		TenantID: tenantID,
		Limit:    limit,
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	responses := make([]dto.OperationResponse, len(ops))
	for i, op := range ops {
		responses[i] = h.toOperationResponse(op)
	}

	c.JSON(http.StatusOK, gin.H{"operations": responses, "total": len(responses)})
}

// GetClusterStatus handles GET /api/v1/cluster/status
func (h *OperationsHandler) GetClusterStatus(c *gin.Context) {
	status, err := h.getClusterStatusUC.Execute(c.Request.Context())
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, dto.ClusterStatusResponse{
		Healthy:       status.Healthy,
		Version:       status.Version,
		EventCount:    status.EventCount,
		StreamCount:   status.StreamCount,
		StorageBytes:  status.StorageBytes,
		UptimeSeconds: status.UptimeSeconds,
		Details:       status.Details,
	})
}

// ClusterHealth handles GET /api/v1/cluster/health (public, no auth)
func (h *OperationsHandler) ClusterHealth(c *gin.Context) {
	if h.coreClient == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"status": "unhealthy",
			"error":  "core service not configured",
		})
		return
	}

	health, err := h.coreClient.HealthCheck(c.Request.Context())
	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"status": "unhealthy",
			"error":  err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"status":  health.Status,
		"version": health.Version,
		"details": health.Details,
	})
}

// extractUserID retrieves the caller's user ID from the gin context.
func (h *OperationsHandler) extractUserID(c *gin.Context) string {
	userID, exists := c.Get("auth_user_id")
	if !exists {
		return ""
	}
	if id, ok := userID.(string); ok {
		return id
	}
	return ""
}

// handleOperationError maps domain errors to HTTP status codes.
func (h *OperationsHandler) handleOperationError(c *gin.Context, err error) {
	switch {
	case errors.Is(err, domain.ErrOperationNotFound):
		c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
	case errors.Is(err, domain.ErrForbidden):
		c.JSON(http.StatusForbidden, gin.H{"error": err.Error()})
	default:
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
	}
}

// toOperationResponse converts an Operation entity to an OperationResponse DTO.
func (h *OperationsHandler) toOperationResponse(op *entities.Operation) dto.OperationResponse {
	return dto.OperationResponse{
		ID:          op.ID,
		Type:        string(op.Type),
		Status:      string(op.Status),
		TenantID:    op.TenantID,
		Parameters:  op.Parameters,
		Result:      op.Result,
		InitiatedBy: op.InitiatedBy,
		StartedAt:   op.StartedAt,
		CompletedAt: op.CompletedAt,
		Error:       op.Error,
		CreatedAt:   op.CreatedAt,
	}
}
