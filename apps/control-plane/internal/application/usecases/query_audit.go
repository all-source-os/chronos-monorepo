package usecases

import (
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// QueryAuditUseCase handles querying audit events.
type QueryAuditUseCase struct {
	auditRepo repositories.AuditRepository
}

// NewQueryAuditUseCase creates a new QueryAuditUseCase.
func NewQueryAuditUseCase(auditRepo repositories.AuditRepository) *QueryAuditUseCase {
	return &QueryAuditUseCase{auditRepo: auditRepo}
}

// Execute queries audit events based on filters.
func (uc *QueryAuditUseCase) Execute(req dto.AuditQueryRequest) ([]*dto.AuditEventResponse, error) {
	limit := req.Limit
	if limit <= 0 {
		limit = 100
	}

	var events []*entities.AuditEvent
	var err error

	switch {
	case req.Errors:
		events, err = uc.auditRepo.FindErrors(limit)
	case req.UserID != "":
		events, err = uc.auditRepo.FindByUser(req.UserID, limit)
	case req.TenantID != "":
		events, err = uc.auditRepo.FindByTenant(req.TenantID, limit)
	case req.Start != "" && req.End != "":
		start, parseErr := time.Parse(time.RFC3339, req.Start)
		if parseErr != nil {
			return nil, parseErr
		}
		end, parseErr := time.Parse(time.RFC3339, req.End)
		if parseErr != nil {
			return nil, parseErr
		}
		events, err = uc.auditRepo.FindByTimeRange(start, end)
	default:
		// Default: return recent errors (general audit trail)
		events, err = uc.auditRepo.FindErrors(limit)
	}

	if err != nil {
		return nil, err
	}

	responses := make([]*dto.AuditEventResponse, len(events))
	for i, event := range events {
		responses[i] = &dto.AuditEventResponse{
			Timestamp:  event.Timestamp,
			EventType:  event.EventType,
			UserID:     event.UserID,
			Username:   event.Username,
			TenantID:   event.TenantID,
			Action:     event.Action,
			Resource:   event.Resource,
			ResourceID: event.ResourceID,
			Method:     event.Method,
			Path:       event.Path,
			StatusCode: event.StatusCode,
			Duration:   event.Duration,
			IPAddress:  event.IPAddress,
			Error:      event.Error,
			Metadata:   event.Metadata,
		}
	}
	return responses, nil
}
