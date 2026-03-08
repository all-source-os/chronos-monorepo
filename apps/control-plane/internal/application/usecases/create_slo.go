package usecases

import (
	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreateSLOUseCase handles creating a new SLO.
type CreateSLOUseCase struct {
	sloRepo   repositories.SLORepository
	auditRepo repositories.AuditRepository
}

// NewCreateSLOUseCase creates a new CreateSLOUseCase.
func NewCreateSLOUseCase(
	sloRepo repositories.SLORepository,
	auditRepo repositories.AuditRepository,
) *CreateSLOUseCase {
	return &CreateSLOUseCase{
		sloRepo:   sloRepo,
		auditRepo: auditRepo,
	}
}

// Execute creates a new SLO with validation.
func (uc *CreateSLOUseCase) Execute(req dto.CreateSLORequest) (*dto.SLOResponse, error) {
	id := uuid.New().String()

	slo, err := entities.NewSLO(
		id,
		req.Name,
		req.Metric,
		req.TargetPercent,
		entities.SLOWindow(req.Window),
	)
	if err != nil {
		return nil, err
	}

	if err := uc.sloRepo.Save(slo); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("slo.created", "create", "POST", "/admin/slos") //nolint:errcheck
	auditEvent.WithResource("slo", slo.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return toSLOResponse(slo), nil
}

func toSLOResponse(slo *entities.SLO) *dto.SLOResponse {
	return &dto.SLOResponse{
		ID:            slo.ID,
		Name:          slo.Name,
		Metric:        slo.Metric,
		TargetPercent: slo.TargetPercent,
		Window:        string(slo.Window),
		CreatedAt:     slo.CreatedAt,
		UpdatedAt:     slo.UpdatedAt,
	}
}
