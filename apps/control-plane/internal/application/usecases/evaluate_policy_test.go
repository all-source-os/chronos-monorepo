package usecases

import (
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestEvaluatePolicyUseCase_Execute(t *testing.T) {
	policyRepo := persistence.NewMemoryPolicyRepository()
	useCase := NewEvaluatePolicyUseCase(policyRepo)

	t.Run("Prevent default tenant deletion", func(t *testing.T) {
		req := dto.EvaluatePolicyRequest{
			Resource: "tenant",
			Attributes: map[string]interface{}{
				"tenant_id": "default",
				"operation": "delete",
			},
		}

		resp, err := useCase.Execute(req)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if resp.Allowed {
			t.Error("Should not allow deletion of default tenant")
		}
		if resp.Action != "deny" {
			t.Errorf("Action = %v, want deny", resp.Action)
		}
	})

	t.Run("Allow non-default tenant deletion", func(t *testing.T) {
		req := dto.EvaluatePolicyRequest{
			Resource: "tenant",
			Attributes: map[string]interface{}{
				"tenant_id": "tenant-1",
				"operation": "delete",
			},
		}

		resp, err := useCase.Execute(req)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if !resp.Allowed {
			t.Error("Should allow deletion of non-default tenant")
		}
	})

	t.Run("Require admin for tenant creation", func(t *testing.T) {
		req := dto.EvaluatePolicyRequest{
			Resource: "tenant",
			Attributes: map[string]interface{}{
				"operation": "create",
				"role":      "Developer",
			},
		}

		resp, err := useCase.Execute(req)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if resp.Allowed {
			t.Error("Should not allow non-admin to create tenant")
		}
	})

	t.Run("Allow admin tenant creation", func(t *testing.T) {
		req := dto.EvaluatePolicyRequest{
			Resource: "tenant",
			Attributes: map[string]interface{}{
				"operation": "create",
				"role":      "Admin",
			},
		}

		resp, err := useCase.Execute(req)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if !resp.Allowed {
			t.Error("Should allow admin to create tenant")
		}
	})
}
