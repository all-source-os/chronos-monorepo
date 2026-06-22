package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// RecoveryAuditTenant is the dedicated system tenant where recovery audit events
// live, mirroring heartbeatTenant ("system") in heartbeat.go. Keeping recovery
// audit out of per-customer queries lets us set retention/compaction on it
// independently, and makes "what recovery ran, when, by whom?" a durable,
// event-sourced query (§4 / §6.3).
const RecoveryAuditTenant = "admin-recovery"

// RecoveryAuditEventPrefix is the event-type namespace for recovery audit
// events: admin.recovery.<action>.
const RecoveryAuditEventPrefix = "admin.recovery."

// RecoveryAuditor writes one durable Core audit event per successful mutating
// recovery action, using the EXISTING IngestEvent pattern (heartbeat.go:243).
// Every Destructive/Guarded apply path calls Record BEFORE returning success —
// a recovery surface that mutates without this is a regression (§6.3 rule 1).
type RecoveryAuditor struct {
	coreClient clients.CoreClient
}

// NewRecoveryAuditor creates a RecoveryAuditor.
func NewRecoveryAuditor(coreClient clients.CoreClient) *RecoveryAuditor {
	return &RecoveryAuditor{coreClient: coreClient}
}

// RecoveryAuditRecord is the payload captured for one recovery action.
type RecoveryAuditRecord struct {
	Action   string         // e.g. "force_resync" (no prefix)
	TenantID string         // the target tenant
	Actor    string         // admin user id from the JWT
	DryRun   bool           // whether this was a preview-only call
	Reason   string         // operator-supplied reason
	Outcome  string         // "applied" | "preview" | "rejected"
	Details  map[string]any // action-specific detail (preview diff, counts, …)
}

// Record writes the audit event to Core. It returns an error so callers can fail
// the apply if the audit write fails (the audit MUST be durable before we report
// success). For dry-run previews the caller may choose to ignore the error since
// no state changed.
func (a *RecoveryAuditor) Record(ctx context.Context, rec RecoveryAuditRecord) error {
	if a.coreClient == nil {
		return nil // no Core wired (test mode) — nothing to persist
	}
	payload := map[string]any{
		"action":    rec.Action,
		"tenant_id": rec.TenantID,
		"actor":     rec.Actor,
		"dry_run":   rec.DryRun,
		"outcome":   rec.Outcome,
		"at":        time.Now().UTC().Format(time.RFC3339Nano),
	}
	if rec.Reason != "" {
		payload["reason"] = rec.Reason
	}
	if rec.Details != nil {
		payload["details"] = rec.Details
	}

	_, err := a.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: RecoveryAuditEventPrefix + rec.Action,
		EntityID:  "recovery:" + rec.TenantID,
		TenantID:  RecoveryAuditTenant,
		Payload:   payload,
	})
	return err
}
