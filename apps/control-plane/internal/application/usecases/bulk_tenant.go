package usecases

import (
	"context"
	"fmt"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// MaxBulkTenants is the maximum number of tenants allowed in a single bulk operation.
const MaxBulkTenants = 100

// BulkTenantAction defines the supported bulk actions.
type BulkTenantAction string

// BulkTenantAction constants define the supported bulk operations.
const (
	BulkActionSuspend BulkTenantAction = "suspend"
	BulkActionArchive BulkTenantAction = "archive"
	BulkActionExport  BulkTenantAction = "export"
)

// BulkTenantRequest represents a request to perform a bulk operation on tenants.
type BulkTenantRequest struct {
	Action    BulkTenantAction `json:"action"`
	TenantIDs []string         `json:"tenant_ids"`
}

// BulkTenantResultItem represents the result of a bulk action on a single tenant.
type BulkTenantResultItem struct {
	TenantID string `json:"tenant_id"`
	Success  bool   `json:"success"`
	Error    string `json:"error,omitempty"`
}

// BulkTenantResponse represents the response from a bulk tenant operation.
type BulkTenantResponse struct {
	Action      string                 `json:"action"`
	Total       int                    `json:"total"`
	Succeeded   int                    `json:"succeeded"`
	Failed      int                    `json:"failed"`
	Results     []BulkTenantResultItem `json:"results"`
	OperationID string                 `json:"operation_id,omitempty"`
}

// BulkTenantUseCase handles bulk operations on tenants.
type BulkTenantUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewBulkTenantUseCase creates a new BulkTenantUseCase.
func NewBulkTenantUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *BulkTenantUseCase {
	return &BulkTenantUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute performs the bulk operation. Requires Admin role.
func (uc *BulkTenantUseCase) Execute(ctx context.Context, req BulkTenantRequest, role entities.Role) (*BulkTenantResponse, error) {
	// Check Admin role
	if role != entities.RoleAdmin {
		return nil, domain.ErrForbidden
	}

	// Validate request
	if len(req.TenantIDs) == 0 {
		return nil, fmt.Errorf("%w: tenant_ids must not be empty", domain.ErrInvalidInput)
	}
	if len(req.TenantIDs) > MaxBulkTenants {
		return nil, fmt.Errorf("%w: maximum %d tenants per bulk operation", domain.ErrInvalidInput, MaxBulkTenants)
	}

	switch req.Action {
	case BulkActionSuspend:
		return uc.executeSuspend(ctx, req.TenantIDs)
	case BulkActionArchive:
		return uc.executeArchive(ctx, req.TenantIDs)
	case BulkActionExport:
		return uc.executeExport(ctx, req.TenantIDs)
	default:
		return nil, fmt.Errorf("%w: unsupported action %q, must be one of: suspend, archive, export", domain.ErrInvalidInput, req.Action)
	}
}

func (uc *BulkTenantUseCase) executeSuspend(ctx context.Context, tenantIDs []string) (*BulkTenantResponse, error) {
	return uc.executeStatusChange(tenantIDs, BulkActionSuspend, func(t *entities.Tenant) { t.Suspend() }, "tenant.bulk_suspended", "suspend")
}

func (uc *BulkTenantUseCase) executeArchive(ctx context.Context, tenantIDs []string) (*BulkTenantResponse, error) {
	return uc.executeStatusChange(tenantIDs, BulkActionArchive, func(t *entities.Tenant) { t.Archive() }, "tenant.bulk_archived", "archive")
}

// executeStatusChange is a shared helper for bulk status-change operations (suspend, archive).
func (uc *BulkTenantUseCase) executeStatusChange(
	tenantIDs []string,
	action BulkTenantAction,
	applyStatus func(*entities.Tenant),
	auditEventType, auditAction string,
) (*BulkTenantResponse, error) {
	results := make([]BulkTenantResultItem, 0, len(tenantIDs))
	succeeded := 0

	for _, id := range tenantIDs {
		item := BulkTenantResultItem{TenantID: id}

		tenant, err := uc.tenantRepo.FindByID(id)
		if err != nil {
			item.Success = false
			item.Error = err.Error()
			results = append(results, item)
			continue
		}

		applyStatus(tenant)
		if err := uc.tenantRepo.Update(tenant); err != nil {
			item.Success = false
			item.Error = err.Error()
			results = append(results, item)
			continue
		}

		// Audit log
		auditEvent, _ := entities.NewAuditEvent(auditEventType, auditAction, "POST", "/admin/tenants/bulk") //nolint:errcheck
		auditEvent.WithResource("tenant", id).WithTenant(id)
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

		item.Success = true
		succeeded++
		results = append(results, item)
	}

	return &BulkTenantResponse{
		Action:    string(action),
		Total:     len(tenantIDs),
		Succeeded: succeeded,
		Failed:    len(tenantIDs) - succeeded,
		Results:   results,
	}, nil
}

func (uc *BulkTenantUseCase) executeExport(ctx context.Context, tenantIDs []string) (*BulkTenantResponse, error) {
	// Export is async — validate all tenant IDs exist, then return an operation ID.
	results := make([]BulkTenantResultItem, 0, len(tenantIDs))
	succeeded := 0

	for _, id := range tenantIDs {
		item := BulkTenantResultItem{TenantID: id}

		_, err := uc.tenantRepo.FindByID(id)
		if err != nil {
			item.Success = false
			item.Error = err.Error()
			results = append(results, item)
			continue
		}

		item.Success = true
		succeeded++
		results = append(results, item)
	}

	// Generate an operation ID for async tracking
	operationID := fmt.Sprintf("export-%d", time.Now().UnixNano())

	// Audit log for the bulk export request
	auditEvent, _ := entities.NewAuditEvent("tenant.bulk_export", "export", "POST", "/admin/tenants/bulk") //nolint:errcheck
	auditEvent.AddMetadata("tenant_ids", tenantIDs).AddMetadata("operation_id", operationID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &BulkTenantResponse{
		Action:      string(BulkActionExport),
		Total:       len(tenantIDs),
		Succeeded:   succeeded,
		Failed:      len(tenantIDs) - succeeded,
		Results:     results,
		OperationID: operationID,
	}, nil
}
