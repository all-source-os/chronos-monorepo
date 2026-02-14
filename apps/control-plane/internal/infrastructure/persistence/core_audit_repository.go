package persistence

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CoreAuditRepository implements AuditRepository by delegating to Core via REST.
type CoreAuditRepository struct {
	client clients.CoreClient
}

// NewCoreAuditRepository creates a new CoreAuditRepository.
func NewCoreAuditRepository(client clients.CoreClient) *CoreAuditRepository {
	return &CoreAuditRepository{client: client}
}

// Log sends an audit event to Core.
func (r *CoreAuditRepository) Log(event *entities.AuditEvent) error {
	ctx := context.Background()
	return r.client.LogAuditEvent(ctx, clients.AuditEventRequest{
		TenantID:     event.TenantID,
		Action:       event.EventType,
		ActorType:    "user",
		ActorID:      event.UserID,
		ActorName:    event.Username,
		Outcome:      outcomeFromEvent(event),
		ResourceType: event.Resource,
		ResourceID:   event.ResourceID,
		IPAddress:    event.IPAddress,
		UserAgent:    event.UserAgent,
		ErrorMessage: event.Error,
		Metadata:     event.Metadata,
	})
}

// FindByUser retrieves audit events for a specific user.
func (r *CoreAuditRepository) FindByUser(userID string, limit int) ([]*entities.AuditEvent, error) {
	ctx := context.Background()
	resp, err := r.client.QueryAuditEvents(ctx, clients.AuditQueryParams{
		UserID: userID,
		Limit:  limit,
	})
	if err != nil {
		return nil, err
	}
	return coreAuditEventsToEntities(resp.Events), nil
}

// FindByTenant retrieves audit events for a specific tenant.
func (r *CoreAuditRepository) FindByTenant(tenantID string, limit int) ([]*entities.AuditEvent, error) {
	ctx := context.Background()
	resp, err := r.client.QueryAuditEvents(ctx, clients.AuditQueryParams{
		TenantID: tenantID,
		Limit:    limit,
	})
	if err != nil {
		return nil, err
	}
	return coreAuditEventsToEntities(resp.Events), nil
}

// FindByTimeRange retrieves audit events within a time range.
func (r *CoreAuditRepository) FindByTimeRange(start, end time.Time) ([]*entities.AuditEvent, error) {
	ctx := context.Background()
	resp, err := r.client.QueryAuditEvents(ctx, clients.AuditQueryParams{
		Start: start.Format(time.RFC3339),
		End:   end.Format(time.RFC3339),
	})
	if err != nil {
		return nil, err
	}
	return coreAuditEventsToEntities(resp.Events), nil
}

// FindErrors retrieves security/error audit events.
func (r *CoreAuditRepository) FindErrors(limit int) ([]*entities.AuditEvent, error) {
	ctx := context.Background()
	resp, err := r.client.QueryAuditEvents(ctx, clients.AuditQueryParams{
		SecurityOnly: true,
		Limit:        limit,
	})
	if err != nil {
		return nil, err
	}
	return coreAuditEventsToEntities(resp.Events), nil
}

func outcomeFromEvent(event *entities.AuditEvent) string {
	if event.Error != "" {
		return "failure"
	}
	return "success"
}

func coreAuditEventsToEntities(items []clients.AuditEventItem) []*entities.AuditEvent {
	result := make([]*entities.AuditEvent, 0, len(items))
	for i := range items {
		item := &items[i]
		ts, _ := time.Parse(time.RFC3339, item.Timestamp) //nolint:errcheck // best-effort timestamp parsing
		event := &entities.AuditEvent{
			Timestamp:  ts,
			EventType:  item.Action,
			TenantID:   item.TenantID,
			Resource:   item.ResourceType,
			ResourceID: item.ResourceID,
			IPAddress:  item.IPAddress,
			Error:      item.ErrorMessage,
			Metadata:   item.Metadata,
		}
		if actorID, ok := item.Actor["user_id"].(string); ok {
			event.UserID = actorID
		}
		if username, ok := item.Actor["username"].(string); ok {
			event.Username = username
		}
		result = append(result, event)
	}
	return result
}
