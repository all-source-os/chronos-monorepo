package usecases

import (
	"context"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestRegisterAgent(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createTenantUC := NewCreateTenantUseCase(tenantRepo, auditRepo)
	mock := &mockCoreClient{}

	uc := NewRegisterAgentUseCase(createTenantUC, auditRepo, mock)

	t.Run("registers agent and creates tenant", func(t *testing.T) {
		mock.events = nil

		resp, err := uc.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "my-claude-agent",
			AgentType: "mcp",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}

		if resp.ID != "agent-my-claude-agent" {
			t.Errorf("expected tenant ID agent-my-claude-agent, got %s", resp.ID)
		}
		if resp.Name != "my-claude-agent" {
			t.Errorf("expected name my-claude-agent, got %s", resp.Name)
		}
		if resp.Status != "active" {
			t.Errorf("expected status active, got %s", resp.Status)
		}

		// Verify metadata
		agentType, ok := resp.Metadata["agent_type"].(string)
		if !ok || agentType != "mcp" {
			t.Errorf("expected agent_type mcp, got %v", resp.Metadata["agent_type"])
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
		ucNilCore := NewRegisterAgentUseCase(createTenantUC, auditRepo, nil)

		resp, err := ucNilCore.Execute(context.Background(), dto.RegisterAgentRequest{
			AgentName: "no-core-agent",
			AgentType: "cli",
		})
		if err != nil {
			t.Fatalf("Execute failed: %v", err)
		}
		if resp.ID != "agent-no-core-agent" {
			t.Errorf("expected agent-no-core-agent, got %s", resp.ID)
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
		if resp.ID != "agent-my-agent-with-spaces" {
			t.Errorf("expected agent-my-agent-with-spaces, got %s", resp.ID)
		}
	})
}
