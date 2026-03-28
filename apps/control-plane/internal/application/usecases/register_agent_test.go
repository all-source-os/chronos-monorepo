package usecases

import (
	"context"
	"fmt"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

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
