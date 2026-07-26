package usecases

import (
	"context"
	"math"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// T0 is the fixed send anchor for the attribution fixtures.
var effT0 = time.Date(2026, 6, 1, 12, 0, 0, 0, time.UTC)

func newEffReco(repo *persistence.MemoryTenantRepository, core *effCore) *CommsEfficiencyUseCase {
	return NewCommsEfficiencyUseCase(repo, persistence.NewMemoryAuditRepository(), core).
		WithClock(func() time.Time { return effT0.Add(30 * 24 * time.Hour) })
}

// paidTags builds the correlation envelope for a paid-welcome send (goal =
// subscription.activated, 14-day window) in a per-scenario variant group.
func paidTags(tenant, variant string, sendTS time.Time) CommsTags {
	return CommsTags{
		TenantID: tenant, CampaignID: "paid", TrailStage: "paid_welcome",
		Variant: variant, Tier: "trial", SendTS: rfc(sendTS),
	}
}

func saveBareTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id string) {
	t.Helper()
	if err := repo.Save(&entities.Tenant{ID: id, Name: id, Status: entities.TenantStatusActive}); err != nil {
		t.Fatalf("save: %v", err)
	}
}

func findGroup(proj *EfficiencyProjection, variant string) *EfficiencyGroup {
	for i := range proj.Groups {
		if proj.Groups[i].Variant == variant {
			return &proj.Groups[i]
		}
	}
	return nil
}

// TestAttribution_EdgeCases proves the four deliberate edge-case decisions, each
// isolated in its own variant group: goal-before-send (not credited), in-window
// (credited), out-of-window (miss), multi-touch (last-touch), churned tenant.
func TestAttribution_EdgeCases(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newEffCore()

	// pre: send at T0, goal 1h BEFORE the send → must NOT be credited.
	saveBareTenant(t, repo, "t_pre")
	core.sendEvent(paidTags("t_pre", "pre", effT0))
	core.addEvent("t_pre", GoalSubscriptionActivated, "s", effT0.Add(-1*time.Hour), nil)

	// in: send at T0, goal +2d (within 14d) → credited.
	saveBareTenant(t, repo, "t_in")
	core.sendEvent(paidTags("t_in", "in", effT0))
	core.addEvent("t_in", GoalSubscriptionActivated, "s", effT0.Add(2*24*time.Hour), nil)

	// out: send at T0, goal +20d (beyond 14d) → miss.
	saveBareTenant(t, repo, "t_out")
	core.sendEvent(paidTags("t_out", "out", effT0))
	core.addEvent("t_out", GoalSubscriptionActivated, "s", effT0.Add(20*24*time.Hour), nil)

	// multi: TWO sends (T0, T0+5d), goal at T0+6d → LAST-touch credits the +5d send,
	// not the +0 send. converted == 1 over 2 sends.
	saveBareTenant(t, repo, "t_multi")
	core.sendEvent(paidTags("t_multi", "multi", effT0))
	core.sendEvent(paidTags("t_multi", "multi", effT0.Add(5*24*time.Hour)))
	core.addEvent("t_multi", GoalSubscriptionActivated, "s", effT0.Add(6*24*time.Hour), nil)

	// churn: send at T0, but the tenant is DELETED (not in the repo) → counted as
	// churned, never as a conversion; the reconcile must not error.
	core.sendEvent(paidTags("t_churn", "churn", effT0))
	core.addEvent("t_churn", GoalSubscriptionActivated, "s", effT0.Add(1*24*time.Hour), nil)

	proj, err := newEffReco(repo, core).Compute(context.Background())
	if err != nil {
		t.Fatalf("Compute: %v", err)
	}

	check := func(variant string, wantSent, wantConverted, wantChurned int) {
		g := findGroup(proj, variant)
		if g == nil {
			t.Fatalf("group %q missing", variant)
		}
		if g.Sent != wantSent || g.Converted != wantConverted || g.Churned != wantChurned {
			t.Errorf("group %q: sent=%d converted=%d churned=%d; want sent=%d converted=%d churned=%d",
				variant, g.Sent, g.Converted, g.Churned, wantSent, wantConverted, wantChurned)
		}
	}
	check("pre", 1, 0, 0)   // goal before send → not credited
	check("in", 1, 1, 0)    // goal in window → credited
	check("out", 1, 0, 0)   // goal out of window → miss
	check("multi", 2, 1, 0) // last-touch: only the later send converts
	check("churn", 1, 0, 1) // deleted tenant → churned, not converted
}

// TestLiftMath proves lift = conversion(sent) − conversion(holdout) on a fixture.
func TestLiftMath(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newEffCore()

	// 4 SENT tenants, 2 convert in-window → convert_sent = 0.50.
	for i, conv := range []bool{true, true, false, false} {
		id := "sent_" + string(rune('a'+i))
		saveBareTenant(t, repo, id)
		core.sendEvent(paidTags(id, "exp", effT0))
		if conv {
			core.addEvent(id, GoalSubscriptionActivated, "s", effT0.Add(24*time.Hour), nil)
		}
	}
	// 4 HELD-OUT tenants, 1 converts organically → convert_holdout = 0.25.
	for i, conv := range []bool{true, false, false, false} {
		id := "hold_" + string(rune('a'+i))
		saveBareTenant(t, repo, id)
		core.holdoutEvent(paidTags(id, "exp", effT0))
		if conv {
			core.addEvent(id, GoalSubscriptionActivated, "s", effT0.Add(24*time.Hour), nil)
		}
	}

	proj, err := newEffReco(repo, core).Compute(context.Background())
	if err != nil {
		t.Fatalf("Compute: %v", err)
	}
	g := findGroup(proj, "exp")
	if g == nil {
		t.Fatal("group exp missing")
	}
	if g.Sent != 4 || g.HeldOut != 4 || g.Converted != 2 || g.HoldoutConverted != 1 {
		t.Fatalf("unexpected counts: %+v", g)
	}
	approx := func(got, want float64) bool { return math.Abs(got-want) < 1e-9 }
	if !approx(g.ConvertSent, 0.5) || !approx(g.ConvertHoldout, 0.25) || !approx(g.Lift, 0.25) {
		t.Errorf("lift math: convert_sent=%.3f convert_holdout=%.3f lift=%.3f; want 0.5/0.25/0.25",
			g.ConvertSent, g.ConvertHoldout, g.Lift)
	}
}

// TestFunnelAndHero proves engagement roll-up (delivered/opened/clicked) + that the
// trial→paid hero aggregates the subscription.activated (paid-welcome) groups.
func TestFunnelAndHero(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newEffCore()

	// 2 sent tenants; both delivered + opened, 1 clicked, 1 converts.
	for i, conv := range []bool{true, false} {
		id := "f_" + string(rune('a'+i))
		saveBareTenant(t, repo, id)
		tags := paidTags(id, "A", effT0)
		tags.MessageID = "msg-" + id
		core.sendEvent(tags)
		core.engageEvent(EmailDeliveredEventType, tags)
		core.engageEvent(EmailOpenedEventType, tags)
		if i == 0 {
			core.engageEvent(EmailClickedEventType, tags)
		}
		if conv {
			core.addEvent(id, GoalSubscriptionActivated, "s", effT0.Add(24*time.Hour), nil)
		}
	}

	proj, err := newEffReco(repo, core).Compute(context.Background())
	if err != nil {
		t.Fatalf("Compute: %v", err)
	}
	g := findGroup(proj, "A")
	if g == nil {
		t.Fatal("group A missing")
	}
	if g.Delivered != 2 || g.Opened != 2 || g.Clicked != 1 {
		t.Errorf("funnel counts: delivered=%d opened=%d clicked=%d; want 2/2/1", g.Delivered, g.Opened, g.Clicked)
	}
	// conversion = converted/delivered = 1/2; click_rate = 1/2.
	if math.Abs(g.ConversionRate-0.5) > 1e-9 || math.Abs(g.ClickRate-0.5) > 1e-9 {
		t.Errorf("rates: conversion=%.3f click=%.3f; want 0.5/0.5", g.ConversionRate, g.ClickRate)
	}
	// HERO: trial→paid, keyed on subscription.activated.
	if proj.Hero.GoalEvent != GoalSubscriptionActivated {
		t.Errorf("hero goal must be subscription.activated, got %q", proj.Hero.GoalEvent)
	}
	if proj.Hero.Sent != 2 || proj.Hero.Converted != 1 {
		t.Errorf("hero aggregate: sent=%d converted=%d; want 2/1", proj.Hero.Sent, proj.Hero.Converted)
	}
}

// TestNoFreeSegment asserts the engine never invents a free segment: tiers are read
// straight from the send tags, and the hero is keyed on the paid-conversion event.
func TestNoFreeSegment(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newEffCore()
	saveBareTenant(t, repo, "t1")
	core.sendEvent(paidTags("t1", "A", effT0)) // tier=trial
	proj, err := newEffReco(repo, core).Compute(context.Background())
	if err != nil {
		t.Fatalf("Compute: %v", err)
	}
	for _, g := range proj.Groups {
		if g.Tier == string(entities.TierFree) || g.Tier == "free" {
			t.Errorf("a free segment leaked into the projection: %+v", g)
		}
	}
}

// TestEngagementIdempotency proves a replayed engagement webhook produces exactly
// one engagement Core event (ExpectedVersion first-ingest dedupe).
func TestEngagementIdempotency(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveBareTenant(t, repo, "t1")
	core := newEffCore()
	uc := newCommsUC(repo, core, &fakeMailer{})

	// Seed the correlation so the engagement resolves to real tags.
	//nolint:errcheck // seeding correlation; the assertions below cover the outcome
	_ = uc.recorder.setCorrelation(context.Background(), CommsTags{
		TenantID: "t1", CampaignID: "paid", TrailStage: "paid_welcome", Variant: "A", Tier: "trial", MessageID: "esp-1",
	})

	first, err := uc.RecordEngagement(context.Background(), "esp-1", "email.clicked", rfc(effT0), "https://x")
	if err != nil || first != EngagementIngested {
		t.Fatalf("first engagement: status=%q err=%v (want ingested)", first, err)
	}
	second, err := uc.RecordEngagement(context.Background(), "esp-1", "email.clicked", rfc(effT0), "https://x")
	if err != nil || second != EngagementDuplicate {
		t.Fatalf("replayed engagement: status=%q err=%v (want duplicate)", second, err)
	}
	if got := countCoreEvents(core, EmailClickedEventType); got != 1 {
		t.Fatalf("replayed webhook must produce exactly ONE engagement event, got %d", got)
	}

	// A different engagement TYPE for the same message is a distinct funnel event.
	if s, _ := uc.RecordEngagement( //nolint:errcheck // status is what this asserts on
		context.Background(), "esp-1", "email.opened", rfc(effT0), ""); s != EngagementIngested {
		t.Errorf("a different engagement type must ingest, got %q", s)
	}
	// An unknown ESP type is ignored, not mis-ingested.
	if s, _ := uc.RecordEngagement( //nolint:errcheck // status is what this asserts on
		context.Background(), "esp-1", "email.delivery_delayed", rfc(effT0), ""); s != EngagementIgnored {
		t.Errorf("unknown ESP type must be ignored, got %q", s)
	}
}

// TestEngagementResolvesTagsForReconciler ties ingress → reconcile: an engagement
// event recorded via the webhook path is grouped correctly by the reconciler.
func TestEngagementResolvesTagsForReconciler(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	saveBareTenant(t, repo, "t1")
	core := newEffCore()
	uc := newCommsUC(repo, core, &fakeMailer{})
	//nolint:errcheck // seeding correlation; the assertions below cover the outcome
	_ = uc.recorder.setCorrelation(context.Background(), CommsTags{
		TenantID: "t1", CampaignID: "paid", TrailStage: "paid_welcome", Variant: "A", Tier: "trial",
		SendTS: rfc(effT0), MessageID: "esp-1",
	})
	core.sendEvent(CommsTags{TenantID: "t1", CampaignID: "paid", TrailStage: "paid_welcome", Variant: "A", Tier: "trial", SendTS: rfc(effT0), MessageID: "esp-1"})
	if _, err := uc.RecordEngagement(context.Background(), "esp-1", "email.delivered", rfc(effT0), ""); err != nil {
		t.Fatalf("record delivered: %v", err)
	}

	proj, err := newEffReco(repo, core).Compute(context.Background())
	if err != nil {
		t.Fatalf("Compute: %v", err)
	}
	g := findGroup(proj, "A")
	if g == nil || g.Delivered != 1 {
		t.Fatalf("engagement did not roll up into the reconciler group: %+v", g)
	}
}
