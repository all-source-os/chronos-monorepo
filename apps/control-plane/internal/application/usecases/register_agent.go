package usecases

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Legacy free-tier quota constants. Self-registered agents now start a 14-day
// trial (prompt 048) and get Trial*Quota instead — these are retained only for
// reference/back-compat and the agent-register test's historical assertions.
const (
	AgentFreeTierEventsQuota  = 10000
	AgentFreeTierQueriesQuota = 1000
)

// KeySignerFunc signs an API key for a given tenant.
// Injected by the transport layer so the use case doesn't depend on JWT internals.
type KeySignerFunc func(tenantID, username string, role entities.Role) (string, error)

// CDPWalletProvisioner creates a managed wallet for an agent.
// Implemented by clients.CDPClient; defined here to avoid importing the infrastructure layer.
type CDPWalletProvisioner interface {
	CreateWallet(ctx context.Context, agentName string) (*clients.CDPWallet, error)
}

// RegisterAgentUseCase handles agent self-registration.
// Creates a tenant with agent metadata, signs an API key, and writes an audit event.
// When a CDPWalletProvisioner is provided, also creates a managed Base wallet so the
// agent can pay x402-gated endpoints without holding any private keys.
type RegisterAgentUseCase struct {
	createTenantUC *CreateTenantUseCase
	auditRepo      repositories.AuditRepository
	coreClient     clients.CoreClient
	signKey        KeySignerFunc
	cdp            CDPWalletProvisioner // optional — nil disables wallet provisioning
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

// WithCDP attaches a CDP wallet provisioner. When set, each registered agent gets
// a managed Base wallet — funded by the operator — enabling seamless x402 auto-pay.
func (uc *RegisterAgentUseCase) WithCDP(cdp CDPWalletProvisioner) *RegisterAgentUseCase {
	uc.cdp = cdp
	return uc
}

// Execute registers a new agent and returns the complete response.
// All business logic lives here — the handler only marshals the result.
func (uc *RegisterAgentUseCase) Execute(ctx context.Context, req dto.RegisterAgentRequest) (*dto.RegisterAgentResponse, error) {
	// Generate deterministic tenant ID from agent name
	slug := entities.TenantSlug(req.AgentName)
	tenantID := fmt.Sprintf("agent-%s", slug)

	// A self-registered agent is a self-service signup, so it starts a 14-day
	// trial, NOT a permanent free tier (prompt 048). Same shared tier + expiry
	// stamp as onboard / OAuth / anonymous-trial so the 14-day clock can't drift.
	// The scheduler's trial-expiry sweep suspends the agent's tenant once
	// trial_expires_at passes (reversible — operator/claim can re-activate).
	subscription, _ := TrialSubscriptionMetadata(time.Now())

	// Create tenant with agent metadata
	tenantResp, err := uc.createTenantUC.Execute(dto.CreateTenantRequest{
		ID:          tenantID,
		Name:        req.AgentName,
		Description: fmt.Sprintf("Agent tenant for %s (%s)", req.AgentName, req.AgentType),
		Metadata: map[string]interface{}{
			"agent_type":   req.AgentType,
			"subscription": subscription,
			"quota":        TrialQuotaMetadata(),
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

	// Provision a CDP managed wallet so the agent can auto-pay x402 routes.
	// Non-blocking: wallet failure doesn't block registration — agent still gets API key.
	var walletAddress string
	if uc.cdp != nil {
		wallet, walletErr := uc.cdp.CreateWallet(ctx, req.AgentName)
		if walletErr != nil {
			log.Printf("[agent] CDP wallet provision failed for %s: %v (x402 auto-pay unavailable)", tenantID, walletErr)
			uc.writeProvisionFailureEvent(ctx, tenantID, "create_wallet_failed: "+walletErr.Error())
		} else {
			if storeErr := uc.storeCDPWallet(ctx, tenantID, wallet); storeErr != nil {
				log.Printf("[agent] CDP wallet store failed for %s: %v (x402 auto-pay unavailable)", tenantID, storeErr)
				uc.writeProvisionFailureEvent(ctx, tenantID, "store_wallet_failed: "+storeErr.Error())
				// walletAddress intentionally left empty — don't return an address that lookup will never find
			} else {
				walletAddress = wallet.Address
			}
		}
	}

	// Write agent.registered event to Core (non-blocking)
	uc.writeRegisteredEvent(ctx, tenantID, req, walletAddress)

	// Audit log (non-critical)
	auditEvent, _ := entities.NewAuditEvent("agent.registered", "create", "POST", "/agents/register") //nolint:errcheck
	auditEvent.WithResource("agent", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("agent_type", req.AgentType)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &dto.RegisterAgentResponse{
		TenantID:      tenantResp.ID,
		APIKey:        apiKey,
		Tier:          TrialTierName,
		WalletAddress: walletAddress,
		Quotas: dto.AgentQuotas{
			EventsQuota:  TrialEventsQuota,
			QueriesQuota: TrialQueriesQuota,
		},
	}, nil
}

// storeCDPWallet persists the wallet ID and address in Core's config store
// under a stable key so the auto-pay middleware can look it up per request.
// Returns an error if Core is unavailable — the caller must zero out walletAddress
// in the response to avoid returning an address the lookup will never find.
func (uc *RegisterAgentUseCase) storeCDPWallet(ctx context.Context, tenantID string, wallet *clients.CDPWallet) error {
	if uc.coreClient == nil {
		return nil
	}
	key := "agent:" + tenantID + ":cdp_wallet"
	if err := uc.coreClient.SetConfig(ctx, clients.SetConfigRequest{
		Key:   key,
		Value: wallet.ID + "|" + wallet.Address, // pipe-delimited: walletID|address
	}); err != nil {
		return fmt.Errorf("store CDP wallet config: %w", err)
	}
	return nil
}

// writeProvisionFailureEvent emits an agent.cdp_wallet_provision_failed event to Core
// so operators can detect silent wallet provisioning failures in the audit trail.
func (uc *RegisterAgentUseCase) writeProvisionFailureEvent(ctx context.Context, tenantID, reason string) {
	if uc.coreClient == nil {
		return
	}
	_, err := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "agent.cdp_wallet_provision_failed",
		EntityID:  tenantID,
		Payload:   map[string]any{"reason": reason},
	})
	if err != nil {
		log.Printf("[agent] failed to write cdp provision failure event for %s: %v", tenantID, err)
	}
}

// writeRegisteredEvent writes an agent.registered event to Core for audit trail.
func (uc *RegisterAgentUseCase) writeRegisteredEvent(ctx context.Context, tenantID string, req dto.RegisterAgentRequest, walletAddress string) {
	if uc.coreClient == nil {
		return
	}

	payload := map[string]any{
		"agent_name":    req.AgentName,
		"agent_type":    req.AgentType,
		"tier":          TrialTierName,
		"events_quota":  TrialEventsQuota,
		"queries_quota": TrialQueriesQuota,
	}
	if walletAddress != "" {
		payload["wallet_address"] = walletAddress
	}

	_, err := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "agent.registered",
		EntityID:  tenantID,
		Payload:   payload,
	})
	if err != nil {
		log.Printf("[agent] failed to write agent.registered event for %s: %v", tenantID, err)
	}
}
