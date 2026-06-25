package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CommsAuditTenant is the dedicated system tenant where proactive-comms events
// live, mirroring RecoveryAuditTenant ("admin-recovery") in recovery_audit.go
// and heartbeatTenant ("system") in heartbeat.go. Keeping notices / messages /
// notes out of per-customer queries lets us set retention/compaction on it
// independently, and makes "who messaged whom, when, why, opted-out?" a durable,
// event-sourced query (ADMIN_TENANT_POWER_TOOL §4 Pillar C / §7).
//
// IMPORTANT: this is the audit/admin-view home for the comms events. The
// tenant-FACING notice read (GET /api/v1/notices) queries this same tenant and
// filters by the per-event `tenant_id` payload field, so a tenant only ever sees
// its own notices — it never reads another tenant's row (the events live here,
// not under each customer's own tenant, so a tenant's normal data queries stay
// clean).
const CommsAuditTenant = "admin-comms"

// Comms event-type names. Consumers (admin comms panel, tenant banner, rate
// limiter, audit) filter on these.
const (
	// NoticeCreatedEventType is emitted when an operator posts an in-app notice.
	NoticeCreatedEventType = "admin.notice.created"
	// NoticeDismissedEventType is emitted when a tenant dismisses a notice.
	NoticeDismissedEventType = "admin.notice.dismissed"
	// MessageSentEventType is emitted for every operator→tenant email send
	// (and every skip, with skipped=true) so the comms ledger is complete.
	MessageSentEventType = "admin.message.sent"
	// NoteCreatedEventType is emitted when an operator adds a per-tenant support
	// note. Internal-only — never surfaced to the tenant.
	NoteCreatedEventType = "admin.note.created"
)

// commsRecorder writes durable Core audit events for the comms surface, using the
// EXISTING IngestEvent pattern (heartbeat.go:243 / recovery_audit.go:49). It is
// the single Core-write seam for notices/messages/notes so every comms action is
// event-sourced (no new DB) and replayable.
type commsRecorder struct {
	coreClient clients.CoreClient
}

func newCommsRecorder(coreClient clients.CoreClient) commsRecorder {
	return commsRecorder{coreClient: coreClient}
}

// record writes one comms event to Core under the admin-comms system tenant.
// entityID scopes the event (notice:<id>, note:<tenant_id>, message:<tenant_id>).
// It returns the error so callers that MUST be durable before reporting success
// (a created notice, a sent message) can fail the request if the write fails.
func (r commsRecorder) record(ctx context.Context, eventType, entityID string, payload map[string]any) (string, error) {
	if r.coreClient == nil {
		return "", nil // no Core wired (test mode) — nothing to persist
	}
	if payload == nil {
		payload = map[string]any{}
	}
	if _, ok := payload["at"]; !ok {
		payload["at"] = time.Now().UTC().Format(time.RFC3339Nano)
	}
	resp, err := r.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: eventType,
		EntityID:  entityID,
		TenantID:  CommsAuditTenant,
		Payload:   payload,
	})
	if err != nil {
		return "", err
	}
	if resp == nil {
		return "", nil
	}
	if resp.EventID != "" {
		return resp.EventID, nil
	}
	return resp.ID, nil
}

// queryComms reads comms events of a given type from the admin-comms tenant.
// Used by the tenant-facing notice projection and the rate limiter. Order/limit
// follow Core's QueryEvents contract (CLAUDE.md "Core API").
func (r commsRecorder) queryComms(ctx context.Context, req clients.QueryEventsRequest) ([]clients.EventEntry, error) {
	if r.coreClient == nil {
		return nil, nil
	}
	req.TenantID = CommsAuditTenant
	resp, err := r.coreClient.QueryEvents(ctx, req)
	if err != nil {
		return nil, err
	}
	if resp == nil {
		return nil, nil
	}
	return resp.Events, nil
}
