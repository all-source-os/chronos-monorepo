package usecases

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"time"
)

// Recovery guard errors (mapped to HTTP status by the recovery handler).
var (
	// ErrConfirmRequired is returned when a Destructive action is applied
	// (dry_run=false) without the required confirmation.
	ErrConfirmRequired = errors.New("recovery: confirmation required")
	// ErrConfirmMismatch is returned when confirm_tenant_id does not equal the
	// target tenant id, or an echoed confirm_token is invalid/expired.
	ErrConfirmMismatch = errors.New("recovery: confirmation does not match target")
	// ErrPreconditionFailed is returned when a guard precondition is not met
	// (e.g. reprovision on an active tenant with recent events).
	ErrPreconditionFailed = errors.New("recovery: precondition failed")
	// ErrBatchTooLarge is returned when a batch exceeds the absolute ceiling.
	ErrBatchTooLarge = errors.New("recovery: batch exceeds max_tenants ceiling")
	// ErrBatchDestructiveForbidden is returned when a batch requests a
	// Destructive sub-action (only Guarded sub-actions are allowed in batch).
	ErrBatchDestructiveForbidden = errors.New("recovery: destructive sub-actions are forbidden in batch")
)

// Blast-radius constants (§4.3).
const (
	// BatchDefaultMaxTenants is the default cap when the caller omits max_tenants.
	BatchDefaultMaxTenants = 25
	// BatchAbsoluteCeiling is the hard ceiling no batch may exceed.
	BatchAbsoluteCeiling = 100
	// ReprovisionRecentEventWindow is the lookback during which an active tenant
	// that has ingested is considered "live" and reprovision is hard-rejected.
	ReprovisionRecentEventWindow = 24 * time.Hour
	// confirmTokenTTL is how long a dry-run confirm_token stays valid. Short, so
	// an operator must apply soon after previewing.
	confirmTokenTTL = 15 * time.Minute
)

// dryRunPreviewMessage is the message every dry-run preview returns: it tells
// the operator how to turn the preview into an apply. Shared by the recovery
// batch, reap-demo and cohort-notice previews so the three can never drift.
const dryRunPreviewMessage = "dry-run: echo confirm_token to apply"

// RecoveryRequest is the common body for every mutating recovery action
// (§5.1). Action-specific fields ride in Extra.
type RecoveryRequest struct {
	DryRun          bool           `json:"dry_run"`
	Reason          string         `json:"reason"`
	ConfirmTenantID string         `json:"confirm_tenant_id,omitempty"`
	ConfirmToken    string         `json:"confirm_token,omitempty"`
	Actor           string         `json:"-"` // filled from the admin JWT, never client-trusted
	Extra           map[string]any `json:"-"` // action-specific (e.g. snapshot_id)
}

// RecoveryResult is the common response. On dry_run it carries Would (preview)
// and, for token-gated actions, ConfirmToken. On apply it carries Applied=true.
type RecoveryResult struct {
	Action       string         `json:"action"`
	TenantID     string         `json:"tenant_id,omitempty"`
	DryRun       bool           `json:"dry_run"`
	Applied      bool           `json:"applied"`
	Would        map[string]any `json:"would,omitempty"`
	ConfirmToken string         `json:"confirm_token,omitempty"`
	Message      string         `json:"message,omitempty"`
}

// confirmKind selects which confirmation a Destructive action requires.
type confirmKind int

const (
	confirmNone        confirmKind = iota // Guarded actions: no extra confirmation
	confirmTenantID                       // typed-name: confirm_tenant_id == target
	confirmTokenEchoed                    // echoed token from a prior dry-run
)

// recoveryGuard centralizes the server-side guard enforcement so the MCP and a
// raw curl are bound by the same rules the UI renders (§6.3 rule 3). secret is
// the JWT secret, reused to HMAC the confirm tokens (no new secret, no DB).
type recoveryGuard struct {
	secret []byte
}

func newRecoveryGuard(jwtSecret string) recoveryGuard {
	return recoveryGuard{secret: []byte(jwtSecret)}
}

// mintConfirmToken produces a deterministic, short-lived token bound to the
// (action, tenantID) pair. Returned by a dry-run; the apply call must echo it.
// Stateless: validity is an HMAC over action|tenant|timeBucket, so no store is
// needed (consistent with "no PostgreSQL").
func (g recoveryGuard) mintConfirmToken(action, tenantID string) string {
	bucket := time.Now().Unix() / int64(confirmTokenTTL.Seconds())
	return g.signToken(action, tenantID, bucket)
}

// validateConfirmToken accepts a token issued in the current or the immediately
// previous TTL bucket (so a token minted just before a bucket rollover still
// works). Returns true on match.
func (g recoveryGuard) validateConfirmToken(action, tenantID, token string) bool {
	if token == "" {
		return false
	}
	nowBucket := time.Now().Unix() / int64(confirmTokenTTL.Seconds())
	for _, b := range []int64{nowBucket, nowBucket - 1} {
		if hmac.Equal([]byte(g.signToken(action, tenantID, b)), []byte(token)) {
			return true
		}
	}
	return false
}

func (g recoveryGuard) signToken(action, tenantID string, bucket int64) string {
	mac := hmac.New(sha256.New, g.secret)
	_, _ = fmt.Fprintf(mac, "recovery|%s|%s|%d", action, tenantID, bucket) //nolint:errcheck // hash.Write never errors
	return hex.EncodeToString(mac.Sum(nil))
}

// enforceConfirmation applies the guard for an apply (dry_run=false) call.
// For confirmTenantID it requires req.ConfirmTenantID == tenantID.
// For confirmTokenEchoed it requires a valid req.ConfirmToken for (action, tenant).
// confirmNone passes through (Guarded actions still require dry_run discipline at
// the handler, but no typed confirmation).
func (g recoveryGuard) enforceConfirmation(action, tenantID string, req RecoveryRequest, kind confirmKind) error {
	switch kind {
	case confirmTenantID:
		if req.ConfirmTenantID == "" {
			return ErrConfirmRequired
		}
		if req.ConfirmTenantID != tenantID {
			return ErrConfirmMismatch
		}
	case confirmTokenEchoed:
		if req.ConfirmToken == "" {
			return ErrConfirmRequired
		}
		if !g.validateConfirmToken(action, tenantID, req.ConfirmToken) {
			return ErrConfirmMismatch
		}
	case confirmNone:
		// no extra confirmation
	}
	return nil
}
