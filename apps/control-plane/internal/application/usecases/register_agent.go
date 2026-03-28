package usecases

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Free-tier quota constants. Used by agent registration and referenced in tests.
const (
	AgentFreeTierEventsQuota  = 10000
	AgentFreeTierQueriesQuota = 1000
)

// KeySignerFunc signs an API key for a given tenant.
// Injected by the transport layer so the use case doesn't depend on JWT internals.
type KeySignerFunc func(tenantID, username string, role entities.Role) (string, error)

// RegisterAgentUseCase handles agent self-registration.
// Creates a tenant with agent metadata, signs an API key, and writes an audit event.
type RegisterAgentUseCase struct {
	createTenantUC *CreateTenantUseCase
	auditRepo      repositories.AuditRepository
	coreClient     clients.CoreClient
	signKey        KeySignerFunc
}

// NewRegisterAgentUseCase creates a new RegisterAgentUseCase.
func NewRegisterAgentUseCase(
	createTenantUC *CreateTenantUseCase,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
	signKey KeySignerFunc,
) *RegisterAgentUseCase {
	return &RegisterAgentUseCase{
		createTenantUC: createTenantUC,
		auditRepo:      auditRepo,
		coreClient:     coreClient,
		signKey:        signKey,
	}
}

// Execute registers a new agent and returns the complete response.
// All business logic lives here — the handler only marshals the result.
func (uc *RegisterAgentUseCase) Execute(ctx context.Context, req dto.RegisterAgentRequest) (*dto.RegisterAgentResponse, error) {
	// Generate deterministic tenant ID from agent name
	slug := entities.TenantSlug(req.AgentName)
	tenantID := fmt.Sprintf("agent-%s", slug)

	// Create tenant with agent metadata
	tenantResp, err := uc.createTenantUC.Execute(dto.CreateTenantRequest{
		ID:          tenantID,
		Name:        req.AgentName,
		Description: fmt.Sprintf("Agent tenant for %s (%s)", req.AgentName, req.AgentType),
		Metadata: map[string]interface{}{
			"agent_type": req.AgentType,
			"subscription": map[string]interface{}{
				"tier":   defaultPlan,
				"status": "active",
			},
			"quota": map[string]interface{}{
				"events_quota":  AgentFreeTierEventsQuota,
				"queries_quota": AgentFreeTierQueriesQuota,
			},
		},
	})
	if err != nil {
		return nil, err
	}

	// Sign API key (injected function — no JWT dependency in use case)
	apiKey, err := uc.signKey(tenantResp.ID, req.AgentName, entities.RoleServiceAccount)
	if err != nil {
		return nil, fmt.Errorf("sign API key: %w", err)
	}

	// Write agent.registered event to Core (non-blocking)
	uc.writeRegisteredEvent(ctx, tenantID, req)

	// Audit log (non-critical)
	auditEvent, _ := entities.NewAuditEvent("agent.registered", "create", "POST", "/agents/register") //nolint:errcheck
	auditEvent.WithResource("agent", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("agent_type", req.AgentType)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.RegisterAgentResponse{
		TenantID: tenantResp.ID,
		APIKey:   apiKey,
		Tier:     defaultPlan,
		Quotas: dto.AgentQuotas{
			EventsQuota:  AgentFreeTierEventsQuota,
			QueriesQuota: AgentFreeTierQueriesQuota,
		},
	}, nil
}

// writeRegisteredEvent writes an agent.registered event to Core for audit trail.
func (uc *RegisterAgentUseCase) writeRegisteredEvent(ctx context.Context, tenantID string, req dto.RegisterAgentRequest) {
	if uc.coreClient == nil {
		return
	}

	_, err := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "agent.registered",
		EntityID:  tenantID,
		Payload: map[string]any{
			"agent_name":    req.AgentName,
			"agent_type":    req.AgentType,
			"tier":          defaultPlan,
			"events_quota":  AgentFreeTierEventsQuota,
			"queries_quota": AgentFreeTierQueriesQuota,
		},
	})
	if err != nil {
		log.Printf("[agent] failed to write agent.registered event for %s: %v", tenantID, err)
	}
}
