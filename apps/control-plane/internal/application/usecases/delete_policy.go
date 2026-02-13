package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeletePolicyUseCase handles deleting a policy.
type DeletePolicyUseCase struct {
	policyRepo repositories.PolicyRepository
	auditRepo  repositories.AuditRepository
}

// NewDeletePolicyUseCase creates a new DeletePolicyUseCase.
func NewDeletePolicyUseCase(
	policyRepo repositories.PolicyRepository,
	auditRepo repositories.AuditRepository,
) *DeletePolicyUseCase {
	return &DeletePolicyUseCase{
		policyRepo: policyRepo,
		auditRepo:  auditRepo,
	}
}

// Execute finds the existing policy, deletes it from the repository, and logs an audit event.
func (uc *DeletePolicyUseCase) Execute(id string) error {
	// Verify policy exists
	if _, err := uc.policyRepo.FindByID(id); err != nil {
		return err
	}

	// Delete from repository
	if err := uc.policyRepo.Delete(id); err != nil {
		return err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("policy.deleted", "delete", "DELETE", "/policies/"+id) //nolint:errcheck
	auditEvent.WithResource("policy", id)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
