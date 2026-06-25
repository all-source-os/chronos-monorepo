package usecases

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Comms guard / domain errors (mapped to HTTP status by the comms handler).
var (
	// ErrCommsInvalidInput is returned for a malformed comms request (e.g. a
	// notice with no audience, a message with neither template nor subject/body).
	ErrCommsInvalidInput = errors.New("comms: invalid input")
	// ErrCommsUnknownTemplate is returned when a message names a template that is
	// not in the registry.
	ErrCommsUnknownTemplate = errors.New("comms: unknown template")
	// ErrCommsNoRecipient is returned when an operator→tenant message has no
	// deliverable email (tenant metadata carries no `email`).
	ErrCommsNoRecipient = errors.New("comms: tenant has no email address")
	// ErrCommsCohortTooLarge is returned when a notice cohort exceeds the
	// blast-radius ceiling (reuses the recovery cap so a buggy filter can't
	// blast the whole fleet).
	ErrCommsCohortTooLarge = errors.New("comms: cohort exceeds max_recipients ceiling")
)

// Comms blast-radius constants. A cohort notice reuses the SAME ceiling as a
// recovery batch (BatchAbsoluteCeiling) so the safety model is not forked.
const (
	// CommsCohortDefaultMax is the default cap when the caller omits max_recipients.
	CommsCohortDefaultMax = BatchDefaultMaxTenants
	// CommsCohortCeiling is the hard ceiling no cohort notice may exceed.
	CommsCohortCeiling = BatchAbsoluteCeiling
	// commsConfirmAction is the guard action namespace for cohort confirm tokens
	// (kept distinct from "batch"/"reap_demo" so a token can't cross surfaces).
	commsConfirmAction = "notice_cohort"
)

// CommsUseCase implements proactive help: in-app notices, operator→tenant email,
// and per-tenant support notes — opt-out-aware, rate-limited, and audited as Core
// events (ADMIN_TENANT_POWER_TOOL §4 Pillar C / §9 Phase 6 CP half).
//
// Reuse, not rebuild:
//   - email goes through the EXISTING SMTP EmailClient (the path
//     check_usage_warnings.go uses), never a new mailer.
//   - cohort notices go through the EXISTING recovery guard (dry-run preview +
//     echoed confirm_token + blast-radius cap), not a forked safety model.
//   - every write is a durable Core event via the EXISTING IngestEvent pattern;
//     notices/notes/opt-out persist as events + tenant metadata, no new DB.
type CommsUseCase struct {
	tenantRepo  repositories.TenantRepository
	emailClient clients.EmailClient
	recorder    commsRecorder
	guard       recoveryGuard
}

// CommsDeps bundles the CommsUseCase dependencies.
type CommsDeps struct {
	TenantRepo  repositories.TenantRepository
	CoreClient  clients.CoreClient
	EmailClient clients.EmailClient
	JWTSecret   string
}

// NewCommsUseCase creates a CommsUseCase. EmailClient may be nil (email send then
// reports a configuration error); CoreClient may be nil in tests (audit + notice
// reads become no-ops, exactly like the recovery auditor).
func NewCommsUseCase(d CommsDeps) *CommsUseCase {
	return &CommsUseCase{
		tenantRepo:  d.TenantRepo,
		emailClient: d.EmailClient,
		recorder:    newCommsRecorder(d.CoreClient),
		guard:       newRecoveryGuard(d.JWTSecret),
	}
}

// ----------------------------------------------------------------------------
// Notices
// ----------------------------------------------------------------------------

// NoticeAudience selects who a notice targets. Exactly one of TenantID (single)
// or a cohort selector (Tier/Plan/HealthTier) must be set. An empty audience is
// rejected (ErrCommsInvalidInput) — a notice must have a recipient.
type NoticeAudience struct {
	TenantID   string `json:"tenant_id,omitempty"`
	Tier       string `json:"tier,omitempty"`        // billing tier cohort (indie/studio/…)
	Plan       string `json:"plan,omitempty"`        // alias for tier (admin filter vocabulary)
	HealthTier string `json:"health_tier,omitempty"` // signals tier cohort (at_risk/critical/…)
}

// isCohort reports whether this audience targets more than one tenant.
func (a NoticeAudience) isCohort() bool {
	return a.TenantID == "" && (a.Tier != "" || a.Plan != "" || a.HealthTier != "")
}

// CreateNoticeRequest is the use-case input for POST /api/v1/admin/notices.
type CreateNoticeRequest struct {
	Audience     NoticeAudience `json:"audience"`
	Title        string         `json:"title"`
	Body         string         `json:"body"`
	Severity     string         `json:"severity"` // "info" | "warning" | "critical"
	ExpiresAt    string         `json:"expires_at,omitempty"`
	DryRun       bool           `json:"dry_run"`
	ConfirmToken string         `json:"confirm_token,omitempty"`
	Actor        string         `json:"-"` // from the admin JWT, never client-trusted
}

// NoticeCreated is one posted notice in a create response.
type NoticeCreated struct {
	ID       string `json:"id"`
	TenantID string `json:"tenant_id"`
}

// CreateNoticeResult is the response for POST /api/v1/admin/notices. For a single
// tenant it returns the created notice. For a cohort dry-run it returns the
// recipient preview + count + a confirm_token (the recovery blast-radius
// pattern); the apply (with the echoed token) returns the created notices.
type CreateNoticeResult struct {
	DryRun       bool            `json:"dry_run"`
	Cohort       bool            `json:"cohort"`
	Would        *NoticeWould    `json:"would,omitempty"`
	ConfirmToken string          `json:"confirm_token,omitempty"`
	Created      []NoticeCreated `json:"created,omitempty"`
	Count        int             `json:"count"`
	Message      string          `json:"message,omitempty"`
}

// NoticeWould is the cohort dry-run preview: the recipients + count, mutating
// nothing (mirrors RecoveryResult.Would / BatchResult preview).
type NoticeWould struct {
	RecipientTenantIDs []string `json:"recipient_tenant_ids"`
	Count              int      `json:"count"`
}

func normalizeSeverity(s string) string {
	switch strings.ToLower(strings.TrimSpace(s)) {
	case "warning", "warn":
		return "warning"
	case "critical", "crit":
		return "critical"
	default:
		return "info"
	}
}

// CreateNotice posts an in-app notice to a single tenant or a cohort. Single-
// tenant: one admin.notice.created event, returned immediately. Cohort: the
// recovery guard applies — dry_run returns the recipient preview + count + a
// confirm_token; apply requires that echoed token and is bounded by the
// blast-radius ceiling. Every created notice is a durable Core event (no new DB).
func (uc *CommsUseCase) CreateNotice(ctx context.Context, req CreateNoticeRequest) (*CreateNoticeResult, error) {
	if strings.TrimSpace(req.Title) == "" || strings.TrimSpace(req.Body) == "" {
		return nil, fmt.Errorf("%w: title and body are required", ErrCommsInvalidInput)
	}
	severity := normalizeSeverity(req.Severity)

	// Single-tenant notice — no cohort guard, post immediately.
	if !req.Audience.isCohort() {
		tenantID := strings.TrimSpace(req.Audience.TenantID)
		if tenantID == "" {
			return nil, fmt.Errorf("%w: audience requires a tenant_id or a cohort selector", ErrCommsInvalidInput)
		}
		created, err := uc.postNotice(ctx, tenantID, req.Title, req.Body, severity, req.ExpiresAt, req.Actor)
		if err != nil {
			return nil, err
		}
		return &CreateNoticeResult{
			DryRun: false, Cohort: false,
			Created: []NoticeCreated{created}, Count: 1,
			Message: "notice created",
		}, nil
	}

	// Cohort notice — reuse the recovery blast-radius guard.
	recipients, err := uc.matchCohort(req.Audience)
	if err != nil {
		return nil, err
	}
	if len(recipients) > CommsCohortCeiling {
		return nil, ErrCommsCohortTooLarge
	}

	result := &CreateNoticeResult{Cohort: true, DryRun: req.DryRun}
	if req.DryRun {
		result.Would = &NoticeWould{RecipientTenantIDs: recipients, Count: len(recipients)}
		result.Count = len(recipients)
		result.ConfirmToken = uc.guard.mintConfirmToken(commsConfirmAction, cohortTokenScope(req.Audience, len(recipients)))
		// A cohort dry-run writes NOTHING to Core (not even a preview audit): the
		// strong guarantee the acceptance test asserts is "previews recipients +
		// count and mutates nothing". The apply path below is the only thing that
		// emits admin.notice.created events, so a dry-run can never inflate the
		// notice ledger.
		result.Message = "dry-run: echo confirm_token to apply"
		return result, nil
	}

	// Apply guard: echoed confirm_token bound to THIS cohort + count.
	if req.ConfirmToken == "" {
		return nil, ErrConfirmRequired
	}
	if !uc.guard.validateConfirmToken(commsConfirmAction, cohortTokenScope(req.Audience, len(recipients)), req.ConfirmToken) {
		return nil, ErrConfirmMismatch
	}

	for _, tid := range recipients {
		created, postErr := uc.postNotice(ctx, tid, req.Title, req.Body, severity, req.ExpiresAt, req.Actor)
		if postErr != nil {
			// A single failed write is surfaced via Count < len(recipients) rather
			// than aborting the whole cohort.
			continue
		}
		result.Created = append(result.Created, created)
	}
	result.Count = len(result.Created)
	result.Message = "cohort notice created"
	return result, nil
}

// postNotice writes one admin.notice.created event. The notice id is derived
// from (tenant, title, time) so it's stable within a request and usable as the
// dismiss key. Returns the created descriptor.
func (uc *CommsUseCase) postNotice(ctx context.Context, tenantID, title, body, severity, expiresAt, actor string) (NoticeCreated, error) {
	noticeID := newNoticeID(tenantID, title)
	payload := map[string]any{
		"notice_id": noticeID,
		"tenant_id": tenantID,
		"title":     title,
		"body":      body,
		"severity":  severity,
		"actor":     actor,
	}
	if expiresAt != "" {
		payload["expires_at"] = expiresAt
	}
	if _, err := uc.recorder.record(ctx, NoticeCreatedEventType, "notice:"+noticeID, payload); err != nil {
		return NoticeCreated{}, err
	}
	return NoticeCreated{ID: noticeID, TenantID: tenantID}, nil
}

// ListNotices returns the admin view of notices, optionally filtered to one
// tenant. It folds admin.notice.* events into a current view (created minus
// dismissed minus expired), newest first.
func (uc *CommsUseCase) ListNotices(ctx context.Context, tenantID string) ([]NoticeView, error) {
	return uc.projectNotices(ctx, tenantID, false)
}

// TenantNotices returns the ACTIVE notices for the caller's OWN tenant (the
// dashboard banner read). Same projection as the admin view but always scoped to
// one tenant and excluding dismissed/expired — the tenant only ever sees its own.
func (uc *CommsUseCase) TenantNotices(ctx context.Context, tenantID string) ([]NoticeView, error) {
	if strings.TrimSpace(tenantID) == "" {
		return nil, fmt.Errorf("%w: tenant_id required", ErrCommsInvalidInput)
	}
	return uc.projectNotices(ctx, tenantID, true)
}

// NoticeView is a projected notice (current view).
type NoticeView struct {
	ID        string `json:"id"`
	TenantID  string `json:"tenant_id"`
	Title     string `json:"title"`
	Body      string `json:"body"`
	Severity  string `json:"severity"`
	CreatedAt string `json:"created_at"`
	ExpiresAt string `json:"expires_at,omitempty"`
	Dismissed bool   `json:"dismissed,omitempty"`
}

// projectNotices folds admin.notice.created/dismissed events into a current view.
// When activeOnly is set (tenant-facing read), dismissed and expired notices are
// dropped. When tenantFilter is non-empty, only that tenant's notices are kept.
func (uc *CommsUseCase) projectNotices(ctx context.Context, tenantFilter string, activeOnly bool) ([]NoticeView, error) {
	events, err := uc.recorder.queryComms(ctx, clients.QueryEventsRequest{
		EventType: "admin.notice.", // prefix match: created + dismissed
		Limit:     1000,
	})
	if err != nil {
		return nil, err
	}

	type acc struct {
		view      NoticeView
		dismissed bool
	}
	byID := map[string]*acc{}
	for _, e := range events {
		nid := stringField(e.Payload, "notice_id")
		if nid == "" {
			continue
		}
		switch e.EventType {
		case NoticeCreatedEventType:
			tid := stringField(e.Payload, "tenant_id")
			if tenantFilter != "" && tid != tenantFilter {
				continue
			}
			a := byID[nid]
			if a == nil {
				a = &acc{}
				byID[nid] = a
			}
			a.view = NoticeView{
				ID:        nid,
				TenantID:  tid,
				Title:     stringField(e.Payload, "title"),
				Body:      stringField(e.Payload, "body"),
				Severity:  stringField(e.Payload, "severity"),
				CreatedAt: e.Timestamp,
				ExpiresAt: stringField(e.Payload, "expires_at"),
			}
		case NoticeDismissedEventType:
			a := byID[nid]
			if a == nil {
				a = &acc{}
				byID[nid] = a
			}
			a.dismissed = true
		}
	}

	now := time.Now()
	out := []NoticeView{}
	for _, a := range byID {
		if a.view.ID == "" {
			continue // a dismiss with no matching created (or filtered out)
		}
		a.view.Dismissed = a.dismissed
		if activeOnly {
			if a.dismissed {
				continue
			}
			if expired(a.view.ExpiresAt, now) {
				continue
			}
		}
		out = append(out, a.view)
	}
	// Newest first.
	sort.Slice(out, func(i, j int) bool { return out[i].CreatedAt > out[j].CreatedAt })
	return out, nil
}

// DismissNotice writes admin.notice.dismissed for the caller's tenant. The
// dismiss is scoped to (notice_id, tenant_id) so a tenant can only dismiss its
// own notice; a notice belonging to another tenant is rejected as not-found.
func (uc *CommsUseCase) DismissNotice(ctx context.Context, tenantID, noticeID string) error {
	if strings.TrimSpace(tenantID) == "" || strings.TrimSpace(noticeID) == "" {
		return fmt.Errorf("%w: tenant_id and notice_id required", ErrCommsInvalidInput)
	}
	// Verify the notice belongs to this tenant before recording a dismiss.
	views, err := uc.projectNotices(ctx, tenantID, false)
	if err != nil {
		return err
	}
	found := false
	for _, v := range views {
		if v.ID == noticeID {
			found = true
			break
		}
	}
	if !found {
		return domainNotFound("notice")
	}
	_, err = uc.recorder.record(ctx, NoticeDismissedEventType, "notice:"+noticeID, map[string]any{
		"notice_id": noticeID,
		"tenant_id": tenantID,
		"outcome":   "dismissed",
	})
	return err
}

// ----------------------------------------------------------------------------
// Messages (operator → tenant email)
// ----------------------------------------------------------------------------

// SendMessageRequest is the use-case input for POST /api/v1/admin/messages.
// Either Template (rendered from tenant context) or Subject+Body (a bespoke
// message via the "custom" template) must be provided.
type SendMessageRequest struct {
	TenantID string `json:"tenant_id"`
	Template string `json:"template,omitempty"`
	Subject  string `json:"subject,omitempty"`
	Body     string `json:"body,omitempty"`
	DryRun   bool   `json:"dry_run"`
	Actor    string `json:"-"` // from the admin JWT
}

// SendMessageResult is the response for POST /api/v1/admin/messages.
type SendMessageResult struct {
	TenantID   string `json:"tenant_id"`
	Template   string `json:"template"`
	DryRun     bool   `json:"dry_run"`
	Sent       bool   `json:"sent"`
	Skipped    bool   `json:"skipped"`
	SkipReason string `json:"skip_reason,omitempty"` // skipped_opt_out | skipped_rate_limit
	Recipient  string `json:"recipient,omitempty"`
	Subject    string `json:"subject,omitempty"`
	Message    string `json:"message,omitempty"`
}

// Skip reasons recorded in admin.message.sent (skipped=true) and returned to the
// operator. Querying "who was skipped and why?" is event-sourced.
const (
	SkipOptOut    = "skipped_opt_out"
	SkipRateLimit = "skipped_rate_limit"
)

// SendMessage sends an operator→tenant email via the EXISTING SMTP EmailClient,
// honoring per-tenant opt-out (marketing templates only) and a per-tenant per-
// template cooldown (rate limit). Every outcome — sent, skipped_opt_out,
// skipped_rate_limit — emits an admin.message.sent Core event so the ledger is
// complete. On dry_run it resolves the recipient + subject and reports what WOULD
// happen, mutating nothing.
func (uc *CommsUseCase) SendMessage(ctx context.Context, req SendMessageRequest) (*SendMessageResult, error) {
	tenantID := strings.TrimSpace(req.TenantID)
	if tenantID == "" {
		return nil, fmt.Errorf("%w: tenant_id is required", ErrCommsInvalidInput)
	}

	// Resolve the template (or the "custom" pass-through for a bespoke subject/body).
	tmplKey := strings.TrimSpace(req.Template)
	if tmplKey == "" {
		if strings.TrimSpace(req.Subject) == "" || strings.TrimSpace(req.Body) == "" {
			return nil, fmt.Errorf("%w: provide a template or subject+body", ErrCommsInvalidInput)
		}
		tmplKey = "custom"
	}
	tmpl, ok := LookupTemplate(tmplKey)
	if !ok {
		return nil, fmt.Errorf("%w: %q", ErrCommsUnknownTemplate, tmplKey)
	}

	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	recipient := extractCommsEmail(tenant.Metadata)
	renderCtx := uc.renderContextFor(tenant)
	renderCtx.CustomSubject = req.Subject
	renderCtx.CustomBody = req.Body
	subject, body := tmpl.Render(renderCtx)

	result := &SendMessageResult{
		TenantID: tenantID, Template: tmplKey, DryRun: req.DryRun,
		Recipient: recipient, Subject: subject,
	}

	// A message with no deliverable address is a hard error (the operator picked
	// a tenant with no email on file) — surfaced, never silently dropped.
	if recipient == "" {
		return nil, ErrCommsNoRecipient
	}

	// Opt-out check (marketing templates honor it; operational templates are
	// exempt because they are service-critical).
	if tmpl.Category == CategoryMarketing && commsOptedOut(tenant.Metadata) {
		result.Skipped = true
		result.SkipReason = SkipOptOut
		result.Message = "tenant has opted out of marketing comms"
		if !req.DryRun {
			_, _ = uc.recorder.record(ctx, MessageSentEventType, "message:"+tenantID, uc.messagePayload(tenantID, tmplKey, recipient, subject, req.Actor, true, SkipOptOut)) //nolint:errcheck // audit best-effort on skip
		}
		return result, nil
	}

	// Rate-limit check: a repeat of the same template within the cooldown is
	// skipped. The cooldown is read from the last admin.message.sent (sent, not
	// skipped) event for this tenant+template — event-sourced, no new store.
	if !req.DryRun && tmpl.Cooldown > 0 {
		limited, err := uc.rateLimited(ctx, tenantID, tmplKey, tmpl.Cooldown)
		if err == nil && limited {
			result.Skipped = true
			result.SkipReason = SkipRateLimit
			result.Message = "rate-limited: same template sent within the cooldown"
			_, _ = uc.recorder.record(ctx, MessageSentEventType, "message:"+tenantID, uc.messagePayload(tenantID, tmplKey, recipient, subject, req.Actor, true, SkipRateLimit)) //nolint:errcheck // audit best-effort on skip
			return result, nil
		}
	}

	if req.DryRun {
		result.Message = "dry-run: would send"
		return result, nil
	}

	// Send via the EXISTING SMTP client (the check_usage_warnings.go path).
	if uc.emailClient == nil {
		return nil, fmt.Errorf("%w: email client not configured", ErrCommsInvalidInput)
	}
	if err := uc.emailClient.SendEmail(ctx, clients.SendEmailRequest{To: recipient, Subject: subject, Body: body}); err != nil {
		return nil, fmt.Errorf("send message to %s: %w", tenantID, err)
	}

	// Audit AFTER a successful send so admin.message.sent (sent=true) is the
	// durable record the rate limiter reads.
	if _, err := uc.recorder.record(ctx, MessageSentEventType, "message:"+tenantID, uc.messagePayload(tenantID, tmplKey, recipient, subject, req.Actor, false, "")); err != nil {
		return nil, err
	}
	result.Sent = true
	result.Message = "message sent"
	return result, nil
}

func (uc *CommsUseCase) messagePayload(tenantID, template, recipient, subject, actor string, skipped bool, skipReason string) map[string]any {
	p := map[string]any{
		"tenant_id": tenantID,
		"template":  template,
		"recipient": recipient,
		"subject":   subject,
		"actor":     actor,
		"skipped":   skipped,
	}
	if skipReason != "" {
		p["skip_reason"] = skipReason
	}
	return p
}

// rateLimited reports whether an admin.message.sent (sent=true) event exists for
// this tenant+template within the cooldown window.
func (uc *CommsUseCase) rateLimited(ctx context.Context, tenantID, template string, cooldown time.Duration) (bool, error) {
	since := time.Now().Add(-cooldown).UTC().Format(time.RFC3339)
	events, err := uc.recorder.queryComms(ctx, clients.QueryEventsRequest{
		EventType: MessageSentEventType,
		EntityID:  "message:" + tenantID,
		Since:     since,
		Limit:     200,
	})
	if err != nil {
		return false, err
	}
	for _, e := range events {
		if stringField(e.Payload, "template") != template {
			continue
		}
		// Only a SENT message counts toward the cooldown (a prior skip does not).
		if boolField(e.Payload, "skipped") {
			continue
		}
		return true, nil
	}
	return false, nil
}

// ----------------------------------------------------------------------------
// Support notes (internal-only)
// ----------------------------------------------------------------------------

// AddNoteRequest is the input for POST /api/v1/admin/tenants/:id/notes.
type AddNoteRequest struct {
	TenantID string `json:"-"` // from the path
	Body     string `json:"body"`
	Actor    string `json:"-"` // from the admin JWT
}

// NoteView is a projected support note.
type NoteView struct {
	ID        string `json:"id"`
	TenantID  string `json:"tenant_id"`
	Body      string `json:"body"`
	Actor     string `json:"actor"`
	CreatedAt string `json:"created_at"`
}

// AddNote records an internal support note for a tenant as an admin.note.created
// Core event. Notes are never sent to the tenant.
func (uc *CommsUseCase) AddNote(ctx context.Context, req AddNoteRequest) (*NoteView, error) {
	tenantID := strings.TrimSpace(req.TenantID)
	if tenantID == "" || strings.TrimSpace(req.Body) == "" {
		return nil, fmt.Errorf("%w: tenant_id and body are required", ErrCommsInvalidInput)
	}
	noteID := newNoticeID(tenantID, req.Body) // reuse the deterministic id helper
	payload := map[string]any{
		"note_id":   noteID,
		"tenant_id": tenantID,
		"body":      req.Body,
		"actor":     req.Actor,
	}
	if _, err := uc.recorder.record(ctx, NoteCreatedEventType, "note:"+tenantID, payload); err != nil {
		return nil, err
	}
	return &NoteView{ID: noteID, TenantID: tenantID, Body: req.Body, Actor: req.Actor, CreatedAt: time.Now().UTC().Format(time.RFC3339Nano)}, nil
}

// ListNotes returns a tenant's support notes, newest first.
func (uc *CommsUseCase) ListNotes(ctx context.Context, tenantID string) ([]NoteView, error) {
	if strings.TrimSpace(tenantID) == "" {
		return nil, fmt.Errorf("%w: tenant_id required", ErrCommsInvalidInput)
	}
	events, err := uc.recorder.queryComms(ctx, clients.QueryEventsRequest{
		EventType: NoteCreatedEventType,
		EntityID:  "note:" + tenantID,
		Limit:     500,
	})
	if err != nil {
		return nil, err
	}
	out := []NoteView{}
	for _, e := range events {
		out = append(out, NoteView{
			ID:        stringField(e.Payload, "note_id"),
			TenantID:  stringField(e.Payload, "tenant_id"),
			Body:      stringField(e.Payload, "body"),
			Actor:     stringField(e.Payload, "actor"),
			CreatedAt: e.Timestamp,
		})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].CreatedAt > out[j].CreatedAt })
	return out, nil
}

// ----------------------------------------------------------------------------
// Cohort selection + helpers
// ----------------------------------------------------------------------------

// matchCohort resolves a cohort audience to tenant ids. Tier/Plan match the
// effective billing tier (reusing the same effectiveBillingTier helper the fleet
// health + batch selectors use). HealthTier is intentionally NOT resolved here
// (it needs a full per-tenant assessment) — a health-tier cohort is composed by
// the admin from GET /fleet/health?tier=at_risk and posted as explicit single
// notices, exactly as BatchFilter omits health-tier for the same reason.
func (uc *CommsUseCase) matchCohort(a NoticeAudience) ([]string, error) {
	if a.HealthTier != "" {
		return nil, fmt.Errorf("%w: health_tier cohorts are composed from /fleet/health, not resolved here", ErrCommsInvalidInput)
	}
	wantTier := normalizeDunningStatus(a.Tier)
	if wantTier == "" {
		wantTier = normalizeDunningStatus(a.Plan)
	}
	if wantTier == "" {
		return nil, fmt.Errorf("%w: cohort requires a tier/plan selector", ErrCommsInvalidInput)
	}
	all, err := uc.tenantRepo.FindAll()
	if err != nil {
		return nil, err
	}
	out := []string{}
	for _, t := range all {
		sub := extractSubscriptionForHealth(t.Metadata)
		tier := effectiveBillingTier(t.Metadata, sub)
		if normalizeDunningStatus(tier) == wantTier {
			out = append(out, t.ID)
		}
	}
	sort.Strings(out)
	return out, nil
}

// renderContextFor builds the template render context from a tenant's record +
// its metered quotas (so quota_warning interpolates real usage).
func (uc *CommsUseCase) renderContextFor(t *entities.Tenant) CommsRenderContext {
	q := extractQuotaForComms(t.Metadata)
	sub := extractSubscriptionForHealth(t.Metadata)
	return CommsRenderContext{
		TenantName:  t.Name,
		Tier:        effectiveBillingTier(t.Metadata, sub),
		EventsUsed:  q.used,
		EventsQuota: q.quota,
	}
}

// cohortTokenScope binds a cohort confirm_token to the audience + count so a
// token minted for one preview can't be replayed against a different cohort /
// count (same defence as batchTokenScope / reapDemoTokenScope).
func cohortTokenScope(a NoticeAudience, count int) string {
	return cohortDescription(a) + ":" + itoaInt(count)
}

func cohortDescription(a NoticeAudience) string {
	tier := normalizeDunningStatus(a.Tier)
	if tier == "" {
		tier = normalizeDunningStatus(a.Plan)
	}
	if a.HealthTier != "" {
		return "health:" + normalizeDunningStatus(a.HealthTier)
	}
	return "tier:" + tier
}

// newNoticeID derives a short, deterministic id from (scope, content, now) so it
// is stable within a request and safe as an entity-id suffix.
func newNoticeID(scope, content string) string {
	h := sha256.Sum256([]byte(scope + "|" + content + "|" + time.Now().UTC().Format(time.RFC3339Nano)))
	return hex.EncodeToString(h[:8])
}

// expired reports whether an RFC3339 expires_at is in the past (false if empty
// or unparseable — an unparseable expiry never hides a notice).
func expired(expiresAt string, now time.Time) bool {
	if expiresAt == "" {
		return false
	}
	ts, err := time.Parse(time.RFC3339, expiresAt)
	if err != nil {
		ts, err = time.Parse(time.RFC3339Nano, expiresAt)
		if err != nil {
			return false
		}
	}
	return now.After(ts)
}

// commsOptedOut reports whether a tenant has set the comms_opt_out flag. Mirrors
// the web use-notification-preferences shape: a top-level `comms_opt_out` bool.
func commsOptedOut(metadata map[string]interface{}) bool {
	if metadata == nil {
		return false
	}
	return boolField(metadata, "comms_opt_out")
}

// extractCommsEmail reads the tenant email from metadata (the SAME field
// check_usage_warnings.go reads: metadata["email"]).
func extractCommsEmail(metadata map[string]interface{}) string {
	if metadata == nil {
		return ""
	}
	if email, ok := metadata["email"].(string); ok {
		return email
	}
	return ""
}

// commsQuota is the events_used / events_quota pair the quota_warning template
// interpolates. Read from metadata.quotas (the same mirror the billing warnings
// read), tolerant of float64 (JSON) and int.
type commsQuota struct {
	used  int64
	quota int64
}

func extractQuotaForComms(metadata map[string]interface{}) commsQuota {
	out := commsQuota{}
	if metadata == nil {
		return out
	}
	q, ok := metadata["quotas"].(map[string]interface{})
	if !ok {
		return out
	}
	out.used = int64Field(q, "events_used")
	out.quota = int64Field(q, "events_quota")
	return out
}

// --- small metadata accessors (tolerant of float64/int/string) ---

func stringField(m map[string]any, key string) string {
	if m == nil {
		return ""
	}
	if v, ok := m[key].(string); ok {
		return v
	}
	return ""
}

func boolField(m map[string]any, key string) bool {
	if m == nil {
		return false
	}
	if v, ok := m[key].(bool); ok {
		return v
	}
	return false
}

func int64Field(m map[string]any, key string) int64 {
	if m == nil {
		return 0
	}
	switch v := m[key].(type) {
	case float64:
		return int64(v)
	case int64:
		return v
	case int:
		return int64(v)
	}
	return 0
}

// domainNotFound returns the shared domain not-found error so the handler maps it
// to 404, matching the recovery handler's error mapping (it reuses
// domain.ErrTenantNotFound, the not-found sentinel the recovery handler already
// switches on).
func domainNotFound(_ string) error {
	return domain.ErrTenantNotFound
}
