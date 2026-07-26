package usecases

import (
	"context"
	"fmt"
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// fakeMailer records sends and returns NO message id (SMTP-shaped).
type fakeMailer struct{ sends int }

func (m *fakeMailer) SendEmail(_ context.Context, _ clients.SendEmailRequest) error {
	m.sends++
	return nil
}

// fakeIDMailer also returns an ESP message id (Resend-shaped) so the send path
// can stamp it on the send event + correlation record.
type fakeIDMailer struct {
	sends int
	id    string
}

func (m *fakeIDMailer) SendEmail(_ context.Context, _ clients.SendEmailRequest) error {
	m.sends++
	return nil
}
func (m *fakeIDMailer) SendEmailWithID(_ context.Context, _ clients.SendEmailRequest) (string, error) {
	m.sends++
	return m.id, nil
}

func saveTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id, tier, email string, optOut bool) {
	t.Helper()
	meta := map[string]interface{}{
		"email":        email,
		"subscription": map[string]interface{}{"tier": tier, "status": "active"},
	}
	if optOut {
		meta["comms_opt_out"] = true
	}
	if err := repo.Save(&entities.Tenant{ID: id, Name: id, Status: entities.TenantStatusActive, Metadata: meta}); err != nil {
		t.Fatalf("save tenant: %v", err)
	}
}

func newCommsUC(repo *persistence.MemoryTenantRepository, core clients.CoreClient, mailer clients.EmailClient) *CommsUseCase {
	return NewCommsUseCase(CommsDeps{TenantRepo: repo, CoreClient: core, EmailClient: mailer, JWTSecret: "test-secret"})
}

// --- HoldoutAssignment: deterministic + bounded ---

func TestHoldoutAssignment_Deterministic(t *testing.T) {
	// Same (tenant, campaign) always lands the same side, on every call.
	for i := 0; i < 100; i++ {
		if HoldoutAssignment("tenant-x", "camp-1", 50) != HoldoutAssignment("tenant-x", "camp-1", 50) {
			t.Fatal("holdout assignment is not stable for the same (tenant, campaign)")
		}
	}
	// Boundaries: 0% never holds out, 100% always.
	if HoldoutAssignment("any", "camp", 0) {
		t.Error("pct=0 must never hold out")
	}
	if !HoldoutAssignment("any", "camp", 100) {
		t.Error("pct=100 must always hold out")
	}
}

func TestHoldoutAssignment_RoughlyMatchesPct(t *testing.T) {
	const n, pct = 2000, 20
	held := 0
	for i := 0; i < n; i++ {
		if HoldoutAssignment(fmt.Sprintf("tenant-%d", i), "camp", pct) {
			held++
		}
	}
	frac := float64(held) / float64(n)
	if frac < 0.15 || frac > 0.25 { // ~20% ± a generous band
		t.Errorf("holdout fraction %.3f is far from target %d%%", frac, pct)
	}
}

// --- SendMessage holdout integration: marketing only, opt-out + critical exclusion ---

func TestSendMessage_HoldoutSuppressesMarketingSend(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveTenant(t, repo, "t1", "indie", "t1@x.com", false)
	core := newEffCore()
	mailer := &fakeMailer{}
	uc := newCommsUC(repo, core, mailer)

	// onboarding_nudge is a MARKETING template; pct=100 → certain holdout.
	res, err := uc.SendMessage(context.Background(), SendMessageRequest{
		TenantID: "t1", Template: "onboarding_nudge", Campaign: "lifecycle", HoldoutPct: 100,
	})
	if err != nil {
		t.Fatalf("SendMessage: %v", err)
	}
	if !res.HeldOut || res.Sent || res.SkipReason != SkipHeldOut {
		t.Fatalf("expected held-out (no send), got %+v", res)
	}
	if mailer.sends != 0 {
		t.Errorf("held-out send must NOT call the mailer, got %d sends", mailer.sends)
	}
	// A comms.holdout event must be recorded (and NO admin.message.sent send).
	if got := countCoreEvents(core, HoldoutEventType); got != 1 {
		t.Errorf("expected 1 comms.holdout event, got %d", got)
	}
	if got := countNonSkippedSends(core); got != 0 {
		t.Errorf("a held-out send must record 0 admin.message.sent sends, got %d", got)
	}
}

func TestSendMessage_HoldoutNeverAppliesToCriticalSend(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveTenant(t, repo, "t1", "indie", "t1@x.com", false)
	core := newEffCore()
	mailer := &fakeMailer{}
	uc := newCommsUC(repo, core, mailer)

	// quota_warning is an OPERATIONAL (service-critical) template. Even at pct=100
	// it must NEVER be held out — you don't suppress a critical message for a test.
	res, err := uc.SendMessage(context.Background(), SendMessageRequest{
		TenantID: "t1", Template: "quota_warning", Campaign: "ops", HoldoutPct: 100,
	})
	if err != nil {
		t.Fatalf("SendMessage: %v", err)
	}
	if res.HeldOut {
		t.Fatal("operational/critical send must never be held out")
	}
	if !res.Sent || mailer.sends != 1 {
		t.Errorf("critical send must go out, got sent=%v mailerSends=%d", res.Sent, mailer.sends)
	}
	if countCoreEvents(core, HoldoutEventType) != 0 {
		t.Error("operational send must record no holdout marker")
	}
}

func TestSendMessage_OptOutWinsOverHoldout(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveTenant(t, repo, "t1", "indie", "t1@x.com", true) // opted out
	core := newEffCore()
	mailer := &fakeMailer{}
	uc := newCommsUC(repo, core, mailer)

	res, err := uc.SendMessage(context.Background(), SendMessageRequest{
		TenantID: "t1", Template: "onboarding_nudge", Campaign: "lifecycle", HoldoutPct: 100,
	})
	if err != nil {
		t.Fatalf("SendMessage: %v", err)
	}
	// Opt-out is law: the tenant is suppressed as skipped_opt_out, NOT held out.
	if res.SkipReason != SkipOptOut || res.HeldOut {
		t.Fatalf("opt-out must win over holdout, got %+v", res)
	}
	if mailer.sends != 0 || countCoreEvents(core, HoldoutEventType) != 0 {
		t.Error("opted-out tenant: no send, no holdout marker")
	}
}

func TestSendMessage_CapturesESPMessageIDAndCorrelation(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveTenant(t, repo, "t1", "studio", "t1@x.com", false)
	core := newEffCore()
	mailer := &fakeIDMailer{id: "esp-msg-42"}
	uc := newCommsUC(repo, core, mailer)

	res, err := uc.SendMessage(context.Background(), SendMessageRequest{
		TenantID: "t1", Template: "onboarding_nudge", Campaign: "lifecycle",
		TrailStage: "activation_first_event", Variant: "A",
	})
	if err != nil {
		t.Fatalf("SendMessage: %v", err)
	}
	if !res.Sent || res.MessageID != "esp-msg-42" {
		t.Fatalf("expected sent with ESP id, got %+v", res)
	}
	// Correlation record written so the engagement webhook can resolve the id → tags.
	tags, ok := uc.recorder.getCorrelation(context.Background(), "esp-msg-42")
	if !ok || tags.TenantID != "t1" || tags.CampaignID != "lifecycle" || tags.Variant != "A" || tags.Tier != "studio" {
		t.Fatalf("correlation not written correctly: ok=%v tags=%+v", ok, tags)
	}
}

// --- helpers reaching into the fake core's recorded events ---

func countCoreEvents(c *effCore, eventType string) int {
	c.mu.Lock()
	defer c.mu.Unlock()
	n := 0
	for _, e := range c.events {
		if e.eventType == eventType {
			n++
		}
	}
	return n
}

func countNonSkippedSends(c *effCore) int {
	c.mu.Lock()
	defer c.mu.Unlock()
	n := 0
	for _, e := range c.events {
		if e.eventType == MessageSentEventType {
			if skipped, _ := e.payload["skipped"].(bool); !skipped { //nolint:errcheck // absent 'skipped' correctly reads as false
				n++
			}
		}
	}
	return n
}
