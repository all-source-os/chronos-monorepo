package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// GetAdminTenantDetailUseCase retrieves full tenant detail for admin view,
// including plan, quotas, subscription metadata, and member count.
type GetAdminTenantDetailUseCase struct {
	tenantRepo repositories.TenantRepository
	coreClient clients.CoreClient
}

// NewGetAdminTenantDetailUseCase creates a new GetAdminTenantDetailUseCase.
func NewGetAdminTenantDetailUseCase(
	tenantRepo repositories.TenantRepository,
	coreClient clients.CoreClient,
) *GetAdminTenantDetailUseCase {
	return &GetAdminTenantDetailUseCase{
		tenantRepo: tenantRepo,
		coreClient: coreClient,
	}
}

// Execute retrieves a tenant by ID and enriches it with admin-level detail.
func (uc *GetAdminTenantDetailUseCase) Execute(ctx context.Context, id string) (*dto.AdminTenantDetailResponse, error) {
	tenant, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	plan := extractPlanInfoFromTenant(tenant)
	quotas := extractQuotaInfoFromTenant(tenant)
	subscription := extractSubscriptionInfoFromTenant(tenant)

	// Source real per-tenant counts from Core for the detail view (single tenant
	// — one stats call + one config read, cheap). Fall back to the metadata
	// mirror on any miss so a count is always a guarded number.
	events := extractEventCount(tenant)
	members := extractMemberCount(tenant)
	if uc.coreClient != nil {
		if stats, statsErr := uc.coreClient.GetTenantStats(ctx, id); statsErr == nil && stats != nil {
			events = stats.EventCount
		}
		if n, ok := memberCountFromCore(ctx, uc.coreClient, id); ok {
			members = n
		}
	}

	return &dto.AdminTenantDetailResponse{
		ID:           tenant.ID,
		Name:         tenant.Name,
		Description:  tenant.Description,
		Status:       string(tenant.Status),
		CreatedAt:    tenant.CreatedAt,
		UpdatedAt:    tenant.UpdatedAt,
		Metadata:     tenant.Metadata,
		Plan:         plan,
		Quotas:       quotas,
		Subscription: subscription,
		EventCount:   events,
		MemberCount:  members,
	}, nil
}

// extractPlanInfoFromTenant pulls plan details from tenant metadata.
func extractPlanInfoFromTenant(t *entities.Tenant) dto.PlanInfo {
	plan := dto.PlanInfo{
		Name: defaultPlan,
		Tier: defaultPlan,
	}
	if t.Metadata == nil {
		return plan
	}
	// Check nested subscription map first (matches extractPlan pattern)
	if sub, ok := t.Metadata["subscription"]; ok {
		if subMap, ok := sub.(map[string]interface{}); ok {
			if tier, ok := subMap["tier"].(string); ok && tier != "" {
				plan.Tier = tier
				plan.Name = tier
			}
			if name, ok := subMap["plan_name"].(string); ok && name != "" {
				plan.Name = name
			}
		}
	}
	// Also check flat metadata keys
	if name, ok := t.Metadata["plan_name"].(string); ok {
		plan.Name = name
	}
	if tier, ok := t.Metadata["plan_tier"].(string); ok {
		plan.Tier = tier
	}
	return plan
}

// extractQuotaInfoFromTenant pulls quota limits from tenant metadata.
func extractQuotaInfoFromTenant(t *entities.Tenant) dto.TenantQuotasResponse {
	quotas := dto.TenantQuotasResponse{}
	if t.Metadata == nil {
		return quotas
	}
	if q, ok := t.Metadata["quotas"]; ok {
		if qMap, ok := q.(map[string]interface{}); ok {
			if v, ok := qMap["event_limit"]; ok {
				quotas.EventLimit = toInt64Val(v)
			}
			if v, ok := qMap["query_limit"]; ok {
				quotas.QueryLimit = toInt64Val(v)
			}
			if v, ok := qMap["storage_limit_mb"]; ok {
				quotas.StorageLimitMB = toInt64Val(v)
			}
		}
	}
	return quotas
}

// extractSubscriptionInfoFromTenant pulls subscription metadata from tenant metadata.
func extractSubscriptionInfoFromTenant(t *entities.Tenant) *dto.SubscriptionInfo {
	if t.Metadata == nil {
		return nil
	}
	sub, ok := t.Metadata["subscription"]
	if !ok {
		return nil
	}
	subMap, ok := sub.(map[string]interface{})
	if !ok {
		return nil
	}
	provider, _ := subMap["provider"].(string)      //nolint:errcheck // safe type assertion
	externalID, _ := subMap["external_id"].(string) //nolint:errcheck // safe type assertion
	if provider == "" && externalID == "" {
		return nil
	}
	status, _ := subMap["status"].(string)      //nolint:errcheck // safe type assertion
	planName, _ := subMap["plan_name"].(string) //nolint:errcheck // safe type assertion

	return &dto.SubscriptionInfo{
		Provider:   provider,
		ExternalID: externalID,
		Status:     status,
		PlanName:   planName,
	}
}

// toInt64Val converts an interface value to int64, handling common JSON number types.
func toInt64Val(v interface{}) int64 {
	switch n := v.(type) {
	case float64:
		return int64(n)
	case int64:
		return n
	case int:
		return int64(n)
	case float32:
		return int64(n)
	default:
		return 0
	}
}
