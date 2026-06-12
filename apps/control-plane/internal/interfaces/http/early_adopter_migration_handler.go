package http //nolint:revive // package name intentionally matches directory

import (
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// EarlyAdopterMigrationHandler exposes the early-adopter migration as an admin
// endpoint (replacing the RUN_EARLY_ADOPTER_MIGRATION boot-env hack). Admin-
// scoped, idempotent, supports dry_run, and every run is audit-logged.
type EarlyAdopterMigrationHandler struct {
	migrateUC *usecases.MigrateEarlyAdoptersUseCase
}

// NewEarlyAdopterMigrationHandler constructs the handler.
func NewEarlyAdopterMigrationHandler(migrateUC *usecases.MigrateEarlyAdoptersUseCase) *EarlyAdopterMigrationHandler {
	return &EarlyAdopterMigrationHandler{migrateUC: migrateUC}
}

// Run executes a migration. Body is optional: an empty body uses defaults
// (voucher tier "studio", 365 days, no owner, not a dry run). Pass dry_run:true
// to preview without writing.
func (h *EarlyAdopterMigrationHandler) Run(c *gin.Context) {
	if h.migrateUC == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "migration use case unavailable"})
		return
	}
	var req usecases.MigrateEarlyAdoptersRequest
	// Tolerate an empty/malformed body — defaults are safe; the destructive bits
	// (writes) only happen for hosted-free tenants regardless of params.
	_ = c.ShouldBindJSON(&req) //nolint:errcheck
	c.JSON(http.StatusOK, h.migrateUC.Execute(req, time.Now()))
}
