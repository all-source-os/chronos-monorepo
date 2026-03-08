package usecases

import (
	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreateIPRuleUseCase handles creating a new IP rule.
type CreateIPRuleUseCase struct {
	ipRuleRepo repositories.IPRuleRepository
	auditRepo  repositories.AuditRepository
}

// NewCreateIPRuleUseCase creates a new CreateIPRuleUseCase.
func NewCreateIPRuleUseCase(
	ipRuleRepo repositories.IPRuleRepository,
	auditRepo repositories.AuditRepository,
) *CreateIPRuleUseCase {
	return &CreateIPRuleUseCase{
		ipRuleRepo: ipRuleRepo,
		auditRepo:  auditRepo,
	}
}

// Execute creates a new IP rule with validation.
func (uc *CreateIPRuleUseCase) Execute(req dto.CreateIPRuleRequest) (*dto.IPRuleResponse, error) {
	id := uuid.New().String()

	rule, err := entities.NewIPRule(
		id,
		req.CIDR,
		entities.IPRuleType(req.Type),
		req.Description,
		req.CreatedBy,
	)
	if err != nil {
		return nil, err
	}

	if err := uc.ipRuleRepo.Save(rule); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("ip_rule.created", "create", "POST", "/admin/security/ip-rules") //nolint:errcheck
	auditEvent.WithResource("ip_rule", rule.ID)
	auditEvent.AddMetadata("cidr", rule.CIDR)
	auditEvent.AddMetadata("type", string(rule.Type))
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return toIPRuleResponse(rule), nil
}

func toIPRuleResponse(rule *entities.IPRule) *dto.IPRuleResponse {
	return &dto.IPRuleResponse{
		ID:          rule.ID,
		CIDR:        rule.CIDR,
		Type:        string(rule.Type),
		Description: rule.Description,
		CreatedAt:   rule.CreatedAt,
		CreatedBy:   rule.CreatedBy,
	}
}
