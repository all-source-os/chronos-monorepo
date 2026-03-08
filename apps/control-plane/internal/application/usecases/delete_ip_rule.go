package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeleteIPRuleUseCase handles deleting an IP rule.
type DeleteIPRuleUseCase struct {
	ipRuleRepo repositories.IPRuleRepository
	auditRepo  repositories.AuditRepository
}

// NewDeleteIPRuleUseCase creates a new DeleteIPRuleUseCase.
func NewDeleteIPRuleUseCase(
	ipRuleRepo repositories.IPRuleRepository,
	auditRepo repositories.AuditRepository,
) *DeleteIPRuleUseCase {
	return &DeleteIPRuleUseCase{
		ipRuleRepo: ipRuleRepo,
		auditRepo:  auditRepo,
	}
}

// Execute deletes an IP rule by ID.
func (uc *DeleteIPRuleUseCase) Execute(id string) error {
	if err := uc.ipRuleRepo.Delete(id); err != nil {
		return err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("ip_rule.deleted", "delete", "DELETE", "/admin/security/ip-rules/"+id) //nolint:errcheck
	auditEvent.WithResource("ip_rule", id)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
