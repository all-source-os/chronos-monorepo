package usecases

import (
	"strings"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// defaultPlan is the default subscription tier when none is set.
const defaultPlan = "free"

// ListTenantsRequest holds pagination and filter parameters.
type ListTenantsRequest struct {
	Offset int
	Limit  int
	Status string // optional: "active", "suspended", "deleted"; empty means all
	Search string // optional: case-insensitive substring match on tenant name
	Plan   string // optional: filter by subscription tier (free, pro, team)
}

// ListTenantsResponse wraps the paginated result.
type ListTenantsResponse struct {
	Tenants []*dto.TenantResponse `json:"tenants"`
	Total   int                   `json:"total"`
}

// ListTenantsUseCase handles listing tenants with pagination and status filter.
type ListTenantsUseCase struct {
	tenantRepo repositories.TenantRepository
}

// NewListTenantsUseCase creates a new ListTenantsUseCase.
func NewListTenantsUseCase(tenantRepo repositories.TenantRepository) *ListTenantsUseCase {
	return &ListTenantsUseCase{tenantRepo: tenantRepo}
}

// Execute retrieves tenants with optional status filter and pagination.
func (uc *ListTenantsUseCase) Execute(req ListTenantsRequest) (*ListTenantsResponse, error) {
	var tenants []*entities.Tenant
	var err error

	// Filter by status if specified
	switch entities.TenantStatus(req.Status) {
	case entities.TenantStatusActive:
		tenants, err = uc.tenantRepo.FindActive()
	case entities.TenantStatusSuspended, entities.TenantStatusDeleted, entities.TenantStatusArchived:
		tenants, err = uc.tenantRepo.FindAll()
	default:
		tenants, err = uc.tenantRepo.FindAll()
	}
	if err != nil {
		return nil, err
	}

	// Apply in-memory status filter for non-active statuses (suspended, deleted, archived)
	if req.Status != "" && req.Status != string(entities.TenantStatusActive) {
		filtered := make([]*entities.Tenant, 0)
		for _, t := range tenants {
			if string(t.Status) == req.Status {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	// Apply search filter (case-insensitive substring match on name)
	if req.Search != "" {
		filtered := make([]*entities.Tenant, 0, len(tenants))
		for _, t := range tenants {
			if strings.Contains(strings.ToLower(t.Name), strings.ToLower(req.Search)) {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	// Apply plan filter (match subscription tier in metadata)
	if req.Plan != "" {
		filtered := make([]*entities.Tenant, 0, len(tenants))
		for _, t := range tenants {
			tenantPlan := extractPlan(t)
			if strings.EqualFold(tenantPlan, req.Plan) {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	total := len(tenants)

	// Apply pagination
	if req.Offset > 0 {
		if req.Offset >= len(tenants) {
			tenants = nil
		} else {
			tenants = tenants[req.Offset:]
		}
	}
	if req.Limit > 0 && req.Limit < len(tenants) {
		tenants = tenants[:req.Limit]
	}

	// Convert to DTOs
	responses := make([]*dto.TenantResponse, len(tenants))
	for i, t := range tenants {
		responses[i] = &dto.TenantResponse{
			ID:          t.ID,
			Name:        t.Name,
			Description: t.Description,
			Status:      string(t.Status),
			CreatedAt:   t.CreatedAt,
			UpdatedAt:   t.UpdatedAt,
			Metadata:    t.Metadata,
		}
	}

	return &ListTenantsResponse{
		Tenants: responses,
		Total:   total,
	}, nil
}

// ExecuteAdmin retrieves tenants with admin-level enrichment (plan, event_count, member_count)
// and page-based pagination.
func (uc *ListTenantsUseCase) ExecuteAdmin(req ListTenantsRequest) (*dto.AdminListTenantsResponse, error) {
	var tenants []*entities.Tenant
	var err error

	// Fetch tenants based on status filter
	switch entities.TenantStatus(req.Status) {
	case entities.TenantStatusActive:
		tenants, err = uc.tenantRepo.FindActive()
	case entities.TenantStatusSuspended, entities.TenantStatusDeleted, entities.TenantStatusArchived:
		tenants, err = uc.tenantRepo.FindAll()
	default:
		tenants, err = uc.tenantRepo.FindAll()
	}
	if err != nil {
		return nil, err
	}

	// Apply in-memory status filter for non-active statuses
	if req.Status != "" && req.Status != string(entities.TenantStatusActive) {
		filtered := make([]*entities.Tenant, 0)
		for _, t := range tenants {
			if string(t.Status) == req.Status {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	// Apply search filter
	if req.Search != "" {
		filtered := make([]*entities.Tenant, 0, len(tenants))
		for _, t := range tenants {
			if strings.Contains(strings.ToLower(t.Name), strings.ToLower(req.Search)) {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	// Apply plan filter
	if req.Plan != "" {
		filtered := make([]*entities.Tenant, 0, len(tenants))
		for _, t := range tenants {
			tenantPlan := extractPlan(t)
			if strings.EqualFold(tenantPlan, req.Plan) {
				filtered = append(filtered, t)
			}
		}
		tenants = filtered
	}

	total := len(tenants)

	// Page-based pagination
	page := 1
	perPage := 20
	if req.Offset > 0 {
		page = req.Offset // reuse Offset as page for admin endpoint
	}
	if req.Limit > 0 {
		perPage = req.Limit // reuse Limit as perPage for admin endpoint
	}

	totalPages := total / perPage
	if total%perPage != 0 {
		totalPages++
	}
	if totalPages == 0 {
		totalPages = 1
	}

	// Calculate slice bounds from page-based pagination
	start := (page - 1) * perPage
	if start >= total {
		tenants = nil
	} else {
		end := start + perPage
		if end > total {
			end = total
		}
		tenants = tenants[start:end]
	}

	// Convert to admin DTOs with enrichment
	responses := make([]*dto.AdminTenantResponse, len(tenants))
	for i, t := range tenants {
		responses[i] = &dto.AdminTenantResponse{
			ID:          t.ID,
			Name:        t.Name,
			Plan:        extractPlan(t),
			Status:      string(t.Status),
			CreatedAt:   t.CreatedAt,
			EventCount:  extractEventCount(t),
			MemberCount: extractMemberCount(t),
		}
	}

	return &dto.AdminListTenantsResponse{
		Tenants:    responses,
		Total:      total,
		Page:       page,
		PerPage:    perPage,
		TotalPages: totalPages,
	}, nil
}

// extractPlan reads the subscription tier from tenant metadata.
// Returns defaultPlan ("free") if not set.
func extractPlan(t *entities.Tenant) string {
	if t.Metadata == nil {
		return defaultPlan
	}
	sub, ok := t.Metadata["subscription"]
	if !ok {
		return defaultPlan
	}
	subMap, ok := sub.(map[string]interface{})
	if !ok {
		return defaultPlan
	}
	tier, ok := subMap["tier"].(string)
	if !ok || tier == "" {
		return defaultPlan
	}
	return tier
}

// extractEventCount reads the events_used from tenant metadata quotas.
func extractEventCount(t *entities.Tenant) int64 {
	if t.Metadata == nil {
		return 0
	}
	quotas, ok := t.Metadata["quotas"]
	if !ok {
		return 0
	}
	quotaMap, ok := quotas.(map[string]interface{})
	if !ok {
		return 0
	}
	count, ok := quotaMap["events_used"]
	if !ok {
		return 0
	}
	switch v := count.(type) {
	case float64:
		return int64(v)
	case int64:
		return v
	case int:
		return int64(v)
	default:
		return 0
	}
}

// extractMemberCount reads the member_count from tenant metadata.
func extractMemberCount(t *entities.Tenant) int {
	if t.Metadata == nil {
		return 0
	}
	count, ok := t.Metadata["member_count"]
	if !ok {
		return 0
	}
	switch v := count.(type) {
	case float64:
		return int(v)
	case int:
		return v
	case int64:
		return int(v)
	default:
		return 0
	}
}
