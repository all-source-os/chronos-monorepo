package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ViewAsAuditTenant is the dedicated system tenant where read-only "view as
// tenant" impersonation events live, mirroring RecoveryAuditTenant
// ("admin-recovery", recovery_audit.go) and CommsAuditTenant ("admin-comms",
// comms_audit.go). Keeping view-as audit out of per-customer queries lets us set
// retention/compaction on it independently, and makes "who viewed whose data,
// when, for how long?" a durable, event-sourced query (ADMIN_TENANT_POWER_TOOL
// §5.4 / §7).
const ViewAsAuditTenant = "admin-viewas"

// View-as event-type names (admin.viewas.<phase>). Consumers (the admin frame,
// the audit/security view) filter on these.
const (
	// ViewAsStartedEventType is emitted when an operator mints a view-as token —
	// written BEFORE the token is returned, so a started event always precedes any
	// read made with the token (§5.3).
	ViewAsStartedEventType = "admin.viewas.started"
	// ViewAsStoppedEventType is emitted on teardown: an explicit Exit, OR a lazy
	// "token presented past exp" detection. Every started has a paired stopped so
	// the audit never shows a dangling impersonation (§5.3 / §5.4).
	ViewAsStoppedEventType = "admin.viewas.stopped"
	// ViewAsWriteRefusedEventType is the ALARM event: a view_as token was presented
	// on a mutating request and refused. There is deliberately NO admin.viewas.wrote
	// event because view-as never writes — a write ATTEMPT is itself the alarm
	// (§5.4). Its presence in the ledger is a security signal, not a normal flow.
	ViewAsWriteRefusedEventType = "admin.viewas.write_refused"
)

// View-as stop reasons (the `reason` payload field on a stopped event).
const (
	// ViewAsStopReasonExit is an explicit operator "Exit".
	ViewAsStopReasonExit = "exit"
	// ViewAsStopReasonExpired is a lazy teardown when a token is seen past its exp.
	ViewAsStopReasonExpired = "expired"
)

// ViewAsAuditor writes the durable Core audit events for the read-only view-as
// surface, using the EXISTING IngestEvent pattern (heartbeat.go:243 /
// recovery_audit.go:49 / comms_audit.go:66). It is the single Core-write seam for
// started/stopped/write-refused so every impersonation event is event-sourced (no
// new DB) and replayable. CoreClient may be nil in tests → every Record* is a
// no-op, exactly like RecoveryAuditor.
type ViewAsAuditor struct {
	coreClient clients.CoreClient
}

// NewViewAsAuditor creates a ViewAsAuditor.
func NewViewAsAuditor(coreClient clients.CoreClient) *ViewAsAuditor {
	return &ViewAsAuditor{coreClient: coreClient}
}

// record writes one view-as event to Core under the admin-viewas system tenant.
// entityID scopes the event to the viewed tenant (viewas:<tenant_id>) so a
// started/stopped pair (and any write-refused) line up on one entity.
func (a *ViewAsAuditor) record(ctx context.Context, eventType, tenantID string, payload map[string]any) error {
	if a == nil || a.coreClient == nil {
		return nil // no Core wired (test mode) — nothing to persist
	}
	if payload == nil {
		payload = map[string]any{}
	}
	payload["tenant_id"] = tenantID
	if _, ok := payload["at"]; !ok {
		payload["at"] = time.Now().UTC().Format(time.RFC3339Nano)
	}
	_, err := a.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: eventType,
		EntityID:  "viewas:" + tenantID,
		TenantID:  ViewAsAuditTenant,
		Payload:   payload,
	})
	return err
}

// RecordStarted writes admin.viewas.started. Called BEFORE the token is returned
// so the audit trail cannot show a view-as read with no preceding start. expUnix
// is the token's expiry so the audit captures the time-box. Returns the error so
// the mint can fail if the audit write fails (the start MUST be durable).
func (a *ViewAsAuditor) RecordStarted(ctx context.Context, tenantID, actor string, expUnix int64) error {
	return a.record(ctx, ViewAsStartedEventType, tenantID, map[string]any{
		"actor":   actor,
		"act_as":  actor,
		"exp":     expUnix,
		"ttl_sec": int64(time.Until(time.Unix(expUnix, 0)).Seconds()),
	})
}

// RecordStopped writes admin.viewas.stopped on teardown. reason is one of
// ViewAsStopReason{Exit,Expired}. Pairs with the RecordStarted event on the same
// entity (viewas:<tenant_id>) so the audit shows a clean start/stop bracket.
func (a *ViewAsAuditor) RecordStopped(ctx context.Context, tenantID, actor, reason string) error {
	return a.record(ctx, ViewAsStoppedEventType, tenantID, map[string]any{
		"actor":  actor,
		"act_as": actor,
		"reason": reason,
	})
}

// RecordWriteRefused writes admin.viewas.write_refused — the ALARM when a view_as
// token is presented on a mutating request and refused server-side. method/path
// capture WHAT was attempted. This is best-effort from the refusal middleware (the
// refusal itself is the hard guard; the event is the durable alarm).
func (a *ViewAsAuditor) RecordWriteRefused(ctx context.Context, tenantID, actor, method, path string) error {
	return a.record(ctx, ViewAsWriteRefusedEventType, tenantID, map[string]any{
		"actor":  actor,
		"act_as": actor,
		"method": method,
		"path":   path,
	})
}
