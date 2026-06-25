package http //nolint:revive // package name intentionally matches directory

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

const commsTestJWTSecret = "comms-test-secret"

// commsMockCore implements just the CoreClient methods the comms use case touches
// (IngestEvent for audit + writes, QueryEvents for projections/rate-limit). The
// embedded interface makes the rest compile (and panic if hit) — same pattern as
// fleetMockCore. It also stores the ingested events so a later QueryEvents can
// project them, which is what lets one test write a notice and then read it back.
type commsMockCore struct {
	clients.CoreClient

	ingested []clients.IngestEventRequest
	ingestN  int
}

func (m *commsMockCore) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	m.ingested = append(m.ingested, req)
	m.ingestN++
	return &clients.IngestEventResponse{EventID: "evt-" + commsItoa(m.ingestN)}, nil
}

// QueryEvents folds the in-memory ingested events back out, honoring the
// event_type prefix filter, entity_id, and tenant_id the comms use case sends.
// Timestamps are assigned at ingest order so newest-first projection is stable.
func (m *commsMockCore) QueryEvents(_ context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	out := []clients.EventEntry{}
	for i, e := range m.ingested {
		if req.TenantID != "" && e.TenantID != req.TenantID {
			continue
		}
		if req.EntityID != "" && e.EntityID != req.EntityID {
			continue
		}
		if req.EventType != "" && !hasPrefix(e.EventType, req.EventType) {
			continue
		}
		out = append(out, clients.EventEntry{
			ID:        "e" + commsItoa(i),
			EventType: e.EventType,
			EntityID:  e.EntityID,
			// monotonically increasing RFC3339Nano so sort-by-timestamp is stable
			Timestamp: time.Unix(0, int64(i+1)).UTC().Format(time.RFC3339Nano),
			Payload:   e.Payload,
		})
	}
	return &clients.QueryEventsResponse{Events: out, Count: len(out)}, nil
}

// commsEvents returns the events recorded under the admin-comms system tenant.
func (m *commsMockCore) commsEvents(eventType string) []clients.IngestEventRequest {
	out := []clients.IngestEventRequest{}
	for _, e := range m.ingested {
		if e.TenantID == usecases.CommsAuditTenant && (eventType == "" || e.EventType == eventType) {
			out = append(out, e)
		}
	}
	return out
}

// fakeMailer captures sends so a test can assert an operator→tenant email went
// through the EmailClient seam without an SMTP server (the mocked mailer).
type fakeMailer struct {
	sent []clients.SendEmailRequest
	err  error
}

func (f *fakeMailer) SendEmail(_ context.Context, req clients.SendEmailRequest) error {
	if f.err != nil {
		return f.err
	}
	f.sent = append(f.sent, req)
	return nil
}

type commsTestEnv struct {
	repo   *persistence.MemoryTenantRepository
	core   *commsMockCore
	mailer *fakeMailer
	uc     *usecases.CommsUseCase
	h      *CommsHandler
}

func newCommsTestEnv(t *testing.T) *commsTestEnv {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	core := &commsMockCore{}
	mailer := &fakeMailer{}
	uc := usecases.NewCommsUseCase(usecases.CommsDeps{
		TenantRepo:  repo,
		CoreClient:  core,
		EmailClient: mailer,
		JWTSecret:   commsTestJWTSecret,
	})
	return &commsTestEnv{repo: repo, core: core, mailer: mailer, uc: uc, h: NewCommsHandler(uc)}
}

// router builds the real admin router (AdminAuthMiddleware) plus the tenant-facing
// notice routes behind a tiny middleware that injects auth_tenant_id (the same key
// AuthMiddleware sets), so the caller-scoped behaviour is exercised without the
// full external auth client.
func (e *commsTestEnv) router(callerTenant string) *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()

	admin := r.Group("/api/v1/admin")
	admin.Use(AdminAuthMiddleware(commsTestJWTSecret))
	admin.POST("/notices", e.h.CreateNotice)
	admin.GET("/notices", e.h.ListNotices)
	admin.POST("/messages", e.h.SendMessage)
	admin.POST("/tenants/:id/notes", e.h.AddNote)
	admin.GET("/tenants/:id/notes", e.h.ListNotes)

	api := r.Group("/api/v1")
	api.Use(func(c *gin.Context) {
		c.Set("auth_tenant_id", callerTenant)
		c.Next()
	})
	api.GET("/notices", e.h.TenantNotices)
	api.POST("/notices/:id/dismiss", e.h.DismissNotice)
	return r
}

func commsAdminToken(t *testing.T) string {
	t.Helper()
	claims := &AdminClaims{
		UserID: "admin-1", Username: "admin", Email: "admin@all-source.xyz",
		TenantID: "system", Role: entities.RoleAdmin,
		StandardClaims: jwt.StandardClaims{ExpiresAt: time.Now().Add(time.Hour).Unix(), IssuedAt: time.Now().Unix()},
	}
	s, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(commsTestJWTSecret))
	if err != nil {
		t.Fatalf("sign token: %v", err)
	}
	return s
}

func doReq(t *testing.T, r *gin.Engine, method, path, token string, body any) *httptest.ResponseRecorder {
	t.Helper()
	var buf bytes.Buffer
	if body != nil {
		if err := json.NewEncoder(&buf).Encode(body); err != nil {
			t.Fatalf("encode body: %v", err)
		}
	}
	req := httptest.NewRequestWithContext(context.Background(), method, path, &buf)
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func seedTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id, name string, meta map[string]interface{}) {
	t.Helper()
	tn := &entities.Tenant{ID: id, Name: name, Status: entities.TenantStatusActive, CreatedAt: time.Now(), UpdatedAt: time.Now(), Metadata: meta}
	if err := repo.Save(tn); err != nil {
		t.Fatalf("seed tenant: %v", err)
	}
}

// ============================================================================
// Notices: create → tenant read → dismiss (Phase 6 acceptance)
// ============================================================================

func TestNotice_CreateTenantReadDismiss(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-1", "Tenant One", map[string]interface{}{"email": "owner@t1.example"})
	r := env.router("t-1")
	token := commsAdminToken(t)

	// 1) Admin creates a notice for t-1 → writes admin.notice.created.
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/notices", token, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{TenantID: "t-1"},
		Title:    "Heads up", Body: "Your sync stalled", Severity: "warning",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("create notice: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var created usecases.CreateNoticeResult
	mustJSON(t, w, &created)
	if created.Count != 1 || len(created.Created) != 1 {
		t.Fatalf("expected 1 created notice, got %+v", created)
	}
	noticeID := created.Created[0].ID
	if got := env.core.commsEvents(usecases.NoticeCreatedEventType); len(got) != 1 {
		t.Fatalf("expected one admin.notice.created event, got %d", len(got))
	}

	// 2) Tenant-facing GET /api/v1/notices returns it.
	w = doReq(t, r, http.MethodGet, "/api/v1/notices", "", nil)
	if w.Code != http.StatusOK {
		t.Fatalf("tenant notices: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var listed struct {
		Notices []usecases.NoticeView `json:"notices"`
		Count   int                   `json:"count"`
	}
	mustJSON(t, w, &listed)
	if listed.Count != 1 || listed.Notices[0].ID != noticeID {
		t.Fatalf("expected tenant to see notice %s, got %+v", noticeID, listed)
	}
	if listed.Notices[0].Title != "Heads up" {
		t.Errorf("title = %q, want Heads up", listed.Notices[0].Title)
	}

	// 3) Dismiss → writes admin.notice.dismissed and the notice drops from the
	//    active tenant read.
	w = doReq(t, r, http.MethodPost, "/api/v1/notices/"+noticeID+"/dismiss", "", nil)
	if w.Code != http.StatusOK {
		t.Fatalf("dismiss: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	if got := env.core.commsEvents(usecases.NoticeDismissedEventType); len(got) != 1 {
		t.Fatalf("expected one admin.notice.dismissed event, got %d", len(got))
	}
	w = doReq(t, r, http.MethodGet, "/api/v1/notices", "", nil)
	mustJSON(t, w, &listed)
	if listed.Count != 0 {
		t.Errorf("after dismiss, tenant should see 0 active notices, got %d", listed.Count)
	}
}

// A tenant must never read another tenant's notice.
func TestNotice_TenantScopedRead(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-1", "One", map[string]interface{}{"email": "a@x"})
	seedTenant(t, env.repo, "t-2", "Two", map[string]interface{}{"email": "b@x"})
	token := commsAdminToken(t)

	// Notice for t-1.
	rAdmin := env.router("t-1")
	w := doReq(t, rAdmin, http.MethodPost, "/api/v1/admin/notices", token, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{TenantID: "t-1"}, Title: "x", Body: "y",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("create: %d (%s)", w.Code, w.Body.String())
	}

	// t-2 reads its own notices → none.
	rT2 := env.router("t-2")
	w = doReq(t, rT2, http.MethodGet, "/api/v1/notices", "", nil)
	var listed struct {
		Count int `json:"count"`
	}
	mustJSON(t, w, &listed)
	if listed.Count != 0 {
		t.Errorf("t-2 should see 0 notices (t-1's notice is not theirs), got %d", listed.Count)
	}
}

// ============================================================================
// Cohort notice: dry-run previews + confirm-token-gated apply (Phase 6)
// ============================================================================

func TestNotice_CohortDryRunThenTokenGatedApply(t *testing.T) {
	env := newCommsTestEnv(t)
	// Two indie tenants + one studio tenant. The cohort selects tier=indie.
	seedTenant(t, env.repo, "t-a", "A", indieMeta())
	seedTenant(t, env.repo, "t-b", "B", indieMeta())
	seedTenant(t, env.repo, "t-c", "C", studioMeta())
	r := env.router("system")
	token := commsAdminToken(t)

	// Dry-run previews recipients + count and mutates nothing (returns a token).
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/notices", token, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{Tier: "indie"},
		Title:    "Fleet notice", Body: "Read me", Severity: "info", DryRun: true,
	})
	if w.Code != http.StatusOK {
		t.Fatalf("dry-run: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var preview usecases.CreateNoticeResult
	mustJSON(t, w, &preview)
	if !preview.Cohort || preview.Would == nil || preview.Would.Count != 2 {
		t.Fatalf("dry-run should preview 2 indie recipients, got %+v", preview)
	}
	if preview.ConfirmToken == "" {
		t.Fatal("dry-run should return a confirm_token")
	}
	// Mutated nothing: no admin.notice.created events yet.
	if got := env.core.commsEvents(usecases.NoticeCreatedEventType); len(got) != 0 {
		t.Fatalf("dry-run must not create notices, got %d", len(got))
	}

	// Apply WITHOUT the token → 400.
	w = doReq(t, r, http.MethodPost, "/api/v1/admin/notices", token, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{Tier: "indie"}, Title: "Fleet notice", Body: "Read me",
	})
	if w.Code != http.StatusBadRequest {
		t.Fatalf("apply without token: expected 400, got %d (%s)", w.Code, w.Body.String())
	}

	// Apply WITH the echoed token → creates one notice per recipient.
	w = doReq(t, r, http.MethodPost, "/api/v1/admin/notices", token, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{Tier: "indie"}, Title: "Fleet notice", Body: "Read me",
		ConfirmToken: preview.ConfirmToken,
	})
	if w.Code != http.StatusOK {
		t.Fatalf("apply with token: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var applied usecases.CreateNoticeResult
	mustJSON(t, w, &applied)
	if applied.Count != 2 {
		t.Fatalf("apply should create 2 notices, got %+v", applied)
	}
	if got := env.core.commsEvents(usecases.NoticeCreatedEventType); len(got) != 2 {
		t.Fatalf("expected 2 admin.notice.created events after apply, got %d", len(got))
	}
}

// ============================================================================
// Opt-out + rate limit (Phase 6)
// ============================================================================

// A marketing template to an opted-out tenant is skipped with skipped_opt_out
// audited; the email is NOT sent.
func TestMessage_OptedOutSkipped(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-opt", "Opted Out", map[string]interface{}{
		"email": "opt@x.example", "comms_opt_out": true,
	})
	r := env.router("system")
	token := commsAdminToken(t)

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/messages", token, usecases.SendMessageRequest{
		TenantID: "t-opt", Template: "onboarding_nudge", // marketing → honors opt-out
	})
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var res usecases.SendMessageResult
	mustJSON(t, w, &res)
	if !res.Skipped || res.SkipReason != usecases.SkipOptOut {
		t.Fatalf("expected skipped_opt_out, got %+v", res)
	}
	if len(env.mailer.sent) != 0 {
		t.Errorf("opted-out tenant must not receive email, got %d sends", len(env.mailer.sent))
	}
	// The skip is audited (admin.message.sent with skipped=true).
	evs := env.core.commsEvents(usecases.MessageSentEventType)
	if len(evs) != 1 {
		t.Fatalf("expected one admin.message.sent (skip) event, got %d", len(evs))
	}
	if evs[0].Payload["skip_reason"] != usecases.SkipOptOut {
		t.Errorf("audit skip_reason = %v, want %s", evs[0].Payload["skip_reason"], usecases.SkipOptOut)
	}
}

// A second quota_warning within the cooldown is rate-limited.
func TestMessage_RateLimitedWithinCooldown(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-q", "Quota Co", map[string]interface{}{
		"email":  "q@x.example",
		"quotas": map[string]interface{}{"events_used": float64(900), "events_quota": float64(1000)},
	})
	r := env.router("system")
	token := commsAdminToken(t)

	// First quota_warning sends (operational → opt-out exempt).
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/messages", token, usecases.SendMessageRequest{
		TenantID: "t-q", Template: "quota_warning",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("first send: %d (%s)", w.Code, w.Body.String())
	}
	var first usecases.SendMessageResult
	mustJSON(t, w, &first)
	if !first.Sent {
		t.Fatalf("first quota_warning should send, got %+v", first)
	}
	if len(env.mailer.sent) != 1 {
		t.Fatalf("expected 1 email after first send, got %d", len(env.mailer.sent))
	}

	// Second quota_warning within the cooldown is rate-limited (no new email).
	w = doReq(t, r, http.MethodPost, "/api/v1/admin/messages", token, usecases.SendMessageRequest{
		TenantID: "t-q", Template: "quota_warning",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("second send: %d (%s)", w.Code, w.Body.String())
	}
	var second usecases.SendMessageResult
	mustJSON(t, w, &second)
	if !second.Skipped || second.SkipReason != usecases.SkipRateLimit {
		t.Fatalf("second quota_warning should be rate-limited, got %+v", second)
	}
	if len(env.mailer.sent) != 1 {
		t.Errorf("rate-limited send must not email; expected still 1, got %d", len(env.mailer.sent))
	}
}

// ============================================================================
// Operator → tenant message sends via the mocked mailer + emits admin.message.sent
// ============================================================================

func TestMessage_SendsViaMailerAndAudits(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-m", "Msg Co", map[string]interface{}{"email": "to@m.example"})
	r := env.router("system")
	token := commsAdminToken(t)

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/messages", token, usecases.SendMessageRequest{
		TenantID: "t-m", Subject: "Hello from ops", Body: "We noticed something",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var res usecases.SendMessageResult
	mustJSON(t, w, &res)
	if !res.Sent {
		t.Fatalf("expected sent=true, got %+v", res)
	}
	if len(env.mailer.sent) != 1 {
		t.Fatalf("expected exactly one email via the mailer, got %d", len(env.mailer.sent))
	}
	if env.mailer.sent[0].To != "to@m.example" || env.mailer.sent[0].Subject != "Hello from ops" {
		t.Errorf("email mismatch: %+v", env.mailer.sent[0])
	}
	evs := env.core.commsEvents(usecases.MessageSentEventType)
	if len(evs) != 1 {
		t.Fatalf("expected one admin.message.sent event, got %d", len(evs))
	}
	if evs[0].Payload["skipped"] != false {
		t.Errorf("a successful send must audit skipped=false, got %v", evs[0].Payload["skipped"])
	}
}

// Dry-run resolves recipient + subject and mutates nothing (no email, no event).
func TestMessage_DryRunMutatesNothing(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-d", "Dry Co", map[string]interface{}{"email": "d@x.example"})
	r := env.router("system")
	token := commsAdminToken(t)

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/messages?dry_run=true", token, usecases.SendMessageRequest{
		TenantID: "t-d", Template: "at_risk_outreach",
	})
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var res usecases.SendMessageResult
	mustJSON(t, w, &res)
	if res.Sent || res.Recipient != "d@x.example" {
		t.Fatalf("dry-run should not send but should resolve recipient, got %+v", res)
	}
	if len(env.mailer.sent) != 0 {
		t.Errorf("dry-run must not email, got %d", len(env.mailer.sent))
	}
	if len(env.core.commsEvents(usecases.MessageSentEventType)) != 0 {
		t.Errorf("dry-run must not audit a send")
	}
}

// ============================================================================
// Support notes
// ============================================================================

func TestNotes_AddAndList(t *testing.T) {
	env := newCommsTestEnv(t)
	seedTenant(t, env.repo, "t-n", "Note Co", map[string]interface{}{"email": "n@x"})
	r := env.router("system")
	token := commsAdminToken(t)

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-n/notes", token, map[string]string{"body": "called customer, sync fixed"})
	if w.Code != http.StatusOK {
		t.Fatalf("add note: %d (%s)", w.Code, w.Body.String())
	}
	if got := env.core.commsEvents(usecases.NoteCreatedEventType); len(got) != 1 {
		t.Fatalf("expected one admin.note.created event, got %d", len(got))
	}

	w = doReq(t, r, http.MethodGet, "/api/v1/admin/tenants/t-n/notes", token, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("list notes: %d (%s)", w.Code, w.Body.String())
	}
	var listed struct {
		Notes []usecases.NoteView `json:"notes"`
		Count int                 `json:"count"`
	}
	mustJSON(t, w, &listed)
	if listed.Count != 1 || listed.Notes[0].Body != "called customer, sync fixed" {
		t.Fatalf("expected the note back, got %+v", listed)
	}
}

// Admin endpoints reject a non-admin token (proves AdminAuthMiddleware reuse).
func TestComms_403WithoutAdmin(t *testing.T) {
	env := newCommsTestEnv(t)
	r := env.router("system")
	claims := &AdminClaims{
		UserID: "u", Role: entities.RoleDeveloper,
		StandardClaims: jwt.StandardClaims{ExpiresAt: time.Now().Add(time.Hour).Unix()},
	}
	devTok, _ := claims.signFor(t) //nolint:errcheck // helper fatals on error
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/notices", devTok, usecases.CreateNoticeRequest{
		Audience: usecases.NoticeAudience{TenantID: "t-1"}, Title: "x", Body: "y",
	})
	if w.Code != http.StatusForbidden {
		t.Fatalf("non-admin should be 403, got %d (%s)", w.Code, w.Body.String())
	}
}

// --- tiny helpers ---

func (c *AdminClaims) signFor(t *testing.T) (string, error) {
	t.Helper()
	return jwt.NewWithClaims(jwt.SigningMethodHS256, c).SignedString([]byte(commsTestJWTSecret))
}

func mustJSON(t *testing.T, w *httptest.ResponseRecorder, v any) {
	t.Helper()
	if err := json.Unmarshal(w.Body.Bytes(), v); err != nil {
		t.Fatalf("unmarshal %s: %v", w.Body.String(), err)
	}
}

func indieMeta() map[string]interface{} {
	return map[string]interface{}{
		"email":        "indie@x.example",
		"subscription": map[string]interface{}{"tier": "indie", "status": "active"},
	}
}

func studioMeta() map[string]interface{} {
	return map[string]interface{}{
		"email":        "studio@x.example",
		"subscription": map[string]interface{}{"tier": "studio", "status": "active"},
	}
}

func commsItoa(n int) string {
	if n == 0 {
		return "0"
	}
	var b [20]byte
	i := len(b)
	for n > 0 {
		i--
		b[i] = byte('0' + n%10)
		n /= 10
	}
	return string(b[i:])
}

func hasPrefix(s, p string) bool {
	return len(s) >= len(p) && s[:len(p)] == p
}
