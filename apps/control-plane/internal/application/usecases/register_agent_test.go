package usecases

import (
	"context"
	"fmt"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// cdpTestCoreClient extends the shared mockCoreClient with SetConfig tracking.
type cdpTestCoreClient struct {
	clients.CoreClient
	events  []clients.IngestEventRequest
	configs map[string]string
	setErr  error // if non-nil, SetConfig returns this error
}

func newCDPTestCoreClient() *cdpTestCoreClient {
	return &cdpTestCoreClient{configs: make(map[string]string)}
}

func (m *cdpTestCoreClient) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	m.events = append(m.events, req)
	return &clients.IngestEventResponse{EventID: "evt-test-001"}, nil
}

func (m *cdpTestCoreClient) SetConfig(_ context.Context, req clients.SetConfigRequest) error {
	if m.setErr != nil {
		return m.setErr
	}
	if s, ok := req.Value.(string); ok {
		m.configs[req.Key] = s
	}
	return nil
}

// mockCDPProvisioner is a controllable CDPWalletProvisioner for tests.
type mockCDPProvisioner struct {
	wallet *clients.CDPWallet
	err    error
}

func (m *mockCDPProvisioner) CreateWallet(_ context.Context, _ string) (*clients.CDPWallet, error) {
	return m.wallet, m.err
}

// stubKeySigner returns a predictable key for testing.
func stubKeySigner(tenantID, username string, role entities.Role) (string, error) {
	return fmt.Sprintf("test-key-%s-%s-%s", tenantID, username, role), nil
}

// failingKeySigner simulates a key signing failure.
func failingKeySigner(_, _ string, _ entities.Role) (string, error) {
	return "", fmt.Errorf("signing unavailable")
}

func TestRegisterAgent(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, mock, stubKeySigner)

	t.Run("registers agent and returns complete response", func(t *testing.T) {
		mock.events = nil

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "my-claude-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		// Verify response fields (use case builds the full response)
		if resp.TenantID != "agent-my-claude-agent" {
			t.Errorf("expected tenant_id agent-my-claude-agent, got %s", resp.TenantID)
		}
		if resp.Tier != "free" {
			t.Errorf("expected tier free, got %s", resp.Tier)
		}
		if resp.Quotas.EventsQuota != AgentFreeTierEventsQuota {
			t.Errorf("expected events_quota %d, got %d", AgentFreeTierEventsQuota, resp.Quotas.EventsQuota)
		}
		if resp.Quotas.QueriesQuota != AgentFreeTierQueriesQuota {
			t.Errorf("expected queries_quota %d, got %d", AgentFreeTierQueriesQuota, resp.Quotas.QueriesQuota)
		}

		// Verify API key was signed via injected function
		expected := "test-key-agent-my-claude-agent-my-claude-agent-serviceaccount"
		if resp.APIKey != expected {
			t.Errorf("expected api_key %q, got %q", expected, resp.APIKey)
		}

		// Verify Core event was written
		if len(mock.events) != 1 {
			t.Fatalf("expected 1 Core event, got %d", len(mock.events))
		}
		if mock.events[0].EventType != "agent.registered" {
			t.Errorf("expected agent.registered, got %s", mock.events[0].EventType)
		}
		if mock.events[0].EntityID != "agent-my-claude-agent" {
			t.Errorf("expected entity_id agent-my-claude-agent, got %s", mock.events[0].EntityID)
		}
	})

	t.Run("rejects duplicate agent name", func(t *testing.T) {
		_, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "my-claude-agent",
			AgentType: "sdk",
		})
		if err == nil {
			t.Fatal("expected error for duplicate agent name")
		}
	})

	t.Run("works without Core client", func(t *testing.T) {
		ucNilCore := NewRegisterAgentUseCase(createTenantUC, auditRepo, nil, stubKeySigner)

		resp, err := ucNilCore.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "no-core-agent",
			AgentType: "cli",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}
		if resp.TenantID != "agent-no-core-agent" {
			t.Errorf("expected agent-no-core-agent, got %s", resp.TenantID)
		}
	})

	t.Run("normalizes agent name to slug", func(t *testing.T) {
		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "My Agent With Spaces",
			AgentType: "sdk",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}
		if resp.TenantID != "agent-my-agent-with-spaces" {
			t.Errorf("expected agent-my-agent-with-spaces, got %s", resp.TenantID)
		}
	})

	t.Run("returns error when key signing fails", func(t *testing.T) {
		ucBadSigner := NewRegisterAgentUseCase(createTenantUC, auditRepo, mock, failingKeySigner)

		_, err := ucBadSigner.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "fail-signer-agent",
			AgentType: "mcp",
		})
		if err == nil {
			t.Fatal("expected error when key signing fails")
		}
	})
}

func TestRegisterAgentWithCDP(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)

	t.Run("CDP happy path — wallet provisioned, stored, address in response", func(t *testing.T) {
		core := newCDPTestCoreClient()
		cdp := &mockCDPProvisioner{
			wallet: &clients.CDPWallet{ID: "wallet-001", Address: "0xdeadbeef", Network: "base-sepolia"},
		}
		uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, core, stubKeySigner).WithCDP(cdp)

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "cdp-happy-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		if resp.WalletAddress != "0xdeadbeef" {
			t.Errorf("WalletAddress = %q, want 0xdeadbeef", resp.WalletAddress)
		}

		// Config should be stored
		key := "agent:agent-cdp-happy-agent:cdp_wallet"
		if core.configs[key] != "wallet-001|0xdeadbeef" {
			t.Errorf("stored config = %q, want wallet-001|0xdeadbeef", core.configs[key])
		}

		// agent.registered event should include wallet_address
		var registered *clients.IngestEventRequest
		for i := range core.events {
			if core.events[i].EventType == "agent.registered" {
				registered = &core.events[i]
				break
			}
		}
		if registered == nil {
			t.Fatal("agent.registered event not found")
		}
		if registered.Payload["wallet_address"] != "0xdeadbeef" {
			t.Errorf("event payload wallet_address = %v, want 0xdeadbeef", registered.Payload["wallet_address"])
		}
	})

	t.Run("CreateWallet fails — empty wallet_address, provision failure event written", func(t *testing.T) {
		core := newCDPTestCoreClient()
		cdp := &mockCDPProvisioner{err: fmt.Errorf("CDP unavailable")}
		uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, core, stubKeySigner).WithCDP(cdp)

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "cdp-create-fail-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed (registration should succeed even on CDP error): %v", err)
		}

		if resp.WalletAddress != "" {
			t.Errorf("WalletAddress = %q, want empty on CDP failure", resp.WalletAddress)
		}

		// provision failure event must be written
		var failureEvent *clients.IngestEventRequest
		for i := range core.events {
			if core.events[i].EventType == "agent.cdp_wallet_provision_failed" {
				failureEvent = &core.events[i]
				break
			}
		}
		if failureEvent == nil {
			t.Fatal("agent.cdp_wallet_provision_failed event not found")
		}
		reason, ok := failureEvent.Payload["reason"].(string)
		if !ok || reason == "" {
			t.Error("provision failure event has empty or non-string reason")
		}
	})

	t.Run("SetConfig fails — wallet_address suppressed, failure event written", func(t *testing.T) {
		core := newCDPTestCoreClient()
		core.setErr = fmt.Errorf("Core config store unavailable")
		cdp := &mockCDPProvisioner{
			wallet: &clients.CDPWallet{ID: "wallet-002", Address: "0xfeedface", Network: "base-sepolia"},
		}
		uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, core, stubKeySigner).WithCDP(cdp)

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "cdp-store-fail-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		// Address must not be returned when storage failed
		if resp.WalletAddress != "" {
			t.Errorf("WalletAddress = %q, want empty when store fails", resp.WalletAddress)
		}

		// provision failure event must be written
		var failureEvent *clients.IngestEventRequest
		for i := range core.events {
			if core.events[i].EventType == "agent.cdp_wallet_provision_failed" {
				failureEvent = &core.events[i]
				break
			}
		}
		if failureEvent == nil {
			t.Fatal("agent.cdp_wallet_provision_failed event not found")
		}
	})

	t.Run("No CDP — no wallet, no failure event", func(t *testing.T) {
		core := newCDPTestCoreClient()
		uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, core, stubKeySigner)
		// WithCDP not called — cdp is nil

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "no-cdp-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		if resp.WalletAddress != "" {
			t.Errorf("WalletAddress = %q, want empty when no CDP configured", resp.WalletAddress)
		}

		for _, ev := range core.events {
			if ev.EventType == "agent.cdp_wallet_provision_failed" {
				t.Errorf("unexpected failure event when CDP not configured")
			}
		}
	})
}
