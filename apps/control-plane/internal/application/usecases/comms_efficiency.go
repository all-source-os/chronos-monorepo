package usecases

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"sort"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CommsEfficiencyUseCase is the proactive-comms attribution reconciler (prompt
// 050). It mirrors the billing sync_* reconcilers: scheduled, paged Core queries,
// write-back of a derived projection the API reads. It is an OPERATOR-level,
// cross-tenant analytic (control-plane's job, like the billing reconcilers) — NOT
// per-tenant read-model compute in Core's projection engine (CLAUDE.md boundary).
//
// THE JOIN (the whole point — dogfood Core): efficiency = email engagement events
// (under the admin-comms operator tenant) ⋈ goal events (in each CUSTOMER tenant's
// own stream), computed here over a configurable attribution window. No external
// analytics stack.
//
// Per campaign/stage/variant/tier it computes: delivered, open-rate, click-rate,
// conversion (goal-in-window / delivered), time-to-goal (median), unsub/complaint
// rate, and causal LIFT = conversion(sent) − conversion(holdout).
type CommsEfficiencyUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
	recorder   commsRecorder
	goalMap    map[string]GoalSpec
	now        func() time.Time
}

// NewCommsEfficiencyUseCase wires the reconciler. coreClient may be nil in tests
// (Compute then returns an empty projection). The goal map defaults to the seed
// map lifted from the lifecycle trail (DefaultGoalMap).
func NewCommsEfficiencyUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *CommsEfficiencyUseCase {
	return &CommsEfficiencyUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
		recorder:   newCommsRecorder(coreClient),
		goalMap:    DefaultGoalMap,
		now:        time.Now,
	}
}

// WithClock overrides the clock (tests use deterministic time). Returns the
// receiver for chaining.
func (uc *CommsEfficiencyUseCase) WithClock(now func() time.Time) *CommsEfficiencyUseCase {
	uc.now = now
	return uc
}

// WithGoalMap overrides the goal-event map (tests + custom campaigns).
func (uc *CommsEfficiencyUseCase) WithGoalMap(m map[string]GoalSpec) *CommsEfficiencyUseCase {
	uc.goalMap = m
	return uc
}

// commsEfficiencyProjectionKey is the Core config key the reconciler writes the
// computed projection to and the API reads (operator-side projection, no new DB).
const commsEfficiencyProjectionKey = "comms:efficiency:projection"

// commsEfficiencyEventLimit bounds a single comms-event page (admin-comms is a
// low-volume operator tenant; the reconciler pages until a short page returns).
const commsEfficiencyEventLimit = 1000

// ----------------------------------------------------------------------------
// Projection (API response shape + write-back shape)
// ----------------------------------------------------------------------------

// EfficiencyProjection is the operator-side aggregate the panel renders. Every
// figure traces to Core events joined here — no parallel analytics stack.
type EfficiencyProjection struct {
	GeneratedAt string            `json:"generated_at"`
	Hero        TrialToPaidHero   `json:"hero"`
	Groups      []EfficiencyGroup `json:"groups"`
	Notes       []string          `json:"notes"`
	GoalLegend  []GoalLegendEntry `json:"goal_legend"`
}

// TrialToPaidHero is the headline funnel: trial → paid (subscription.activated
// within the window), aggregated across the paid-welcome campaigns. THE number
// that funds the company — first-class, not an afterthought.
type TrialToPaidHero struct {
	GoalEvent        string  `json:"goal_event"`
	Sent             int     `json:"sent"`
	HeldOut          int     `json:"held_out"`
	Delivered        int     `json:"delivered"`
	Clicked          int     `json:"clicked"`
	Converted        int     `json:"converted"`
	HoldoutConverted int     `json:"holdout_converted"`
	ConversionRate   float64 `json:"conversion_rate"`         // converted / sent (intent-to-treat)
	HoldoutRate      float64 `json:"holdout_conversion_rate"` // holdout_converted / held_out
	Lift             float64 `json:"lift"`                    // conversion_rate − holdout_rate (causal)
	TimeToGoalMedSec int64   `json:"time_to_goal_median_sec"`
	HasHoldout       bool    `json:"has_holdout"`
}

// EfficiencyGroup is one campaign/stage/variant/tier funnel row.
type EfficiencyGroup struct {
	Campaign   string    `json:"campaign"`
	Stage      string    `json:"stage"`
	Variant    string    `json:"variant"`
	Tier       string    `json:"tier"`
	GoalEvent  string    `json:"goal_event"`
	GoalState  GoalState `json:"goal_state"`
	GoalNote   string    `json:"goal_note,omitempty"`
	WindowDays int       `json:"window_days"`

	// Funnel counts (clicks + conversion LEAD; opens are subordinated in the UI).
	Sent       int `json:"sent"`
	HeldOut    int `json:"held_out"`
	Delivered  int `json:"delivered"`
	Opened     int `json:"opened"`
	Clicked    int `json:"clicked"`
	Bounced    int `json:"bounced"`
	Unsub      int `json:"unsubscribed"`
	Complained int `json:"complained"`
	Churned    int `json:"churned"` // tenants whose stream was unreadable (excluded from conversion)

	OpenRate            float64 `json:"open_rate"`  // UNRELIABLE (MPP) — subordinated in UI
	ClickRate           float64 `json:"click_rate"` // LEAD signal
	Converted           int     `json:"converted"`  // sent recipients with goal in window (last-touch)
	HoldoutConverted    int     `json:"holdout_converted"`
	ConversionRate      float64 `json:"conversion_rate"` // converted / delivered (funnel)
	ConvertSent         float64 `json:"convert_sent"`    // converted / sent (intent-to-treat)
	ConvertHoldout      float64 `json:"convert_holdout"` // holdout_converted / held_out
	Lift                float64 `json:"lift"`            // convert_sent − convert_holdout (causal)
	TimeToGoalMedianSec int64   `json:"time_to_goal_median_sec"`
	UnsubRate           float64 `json:"unsub_rate"`
	ComplaintRate       float64 `json:"complaint_rate"`
}

// GoalLegendEntry surfaces, per stage, whether the goal signal is real today or
// still needs a new signal — so the operator never trusts a dead metric.
type GoalLegendEntry struct {
	Stage     string    `json:"stage"`
	GoalEvent string    `json:"goal_event"`
	State     GoalState `json:"state"`
	Note      string    `json:"note,omitempty"`
}

// ----------------------------------------------------------------------------
// Compute (the reconcile/join)
// ----------------------------------------------------------------------------

// groupKey identifies one funnel row.
type groupKey struct{ Campaign, Stage, Variant, Tier string }

// touch is a send or a holdout — both are "would-send" decisions the goal is
// attributed to (last-touch within the window).
type touch struct {
	tenant    string
	goalEvent string
	ts        time.Time
	window    time.Duration
	holdout   bool
	key       groupKey
}

// groupAcc accumulates one funnel row before it is finalized into an EfficiencyGroup.
type groupAcc struct {
	key              groupKey
	spec             GoalSpec
	sent             int
	heldOut          int
	delivered        map[string]struct{}
	opened           map[string]struct{}
	clicked          map[string]struct{}
	bounced          map[string]struct{}
	unsub            map[string]struct{}
	complaint        map[string]struct{}
	converted        int
	holdoutConverted int
	churned          int
	ttg              []time.Duration
}

func newGroupAcc(key groupKey, spec GoalSpec) *groupAcc {
	return &groupAcc{
		key: key, spec: spec,
		delivered: map[string]struct{}{}, opened: map[string]struct{}{},
		clicked: map[string]struct{}{}, bounced: map[string]struct{}{},
		unsub: map[string]struct{}{}, complaint: map[string]struct{}{},
	}
}

// Compute reads the comms instrumentation events + joins them to goal events,
// returning the live projection. This is the pure read/compute half (no write-back)
// the API can call directly so the panel is always fresh.
func (uc *CommsEfficiencyUseCase) Compute(ctx context.Context) (*EfficiencyProjection, error) {
	proj := &EfficiencyProjection{
		GeneratedAt: uc.now().UTC().Format(time.RFC3339),
		Groups:      []EfficiencyGroup{},
		Notes:       efficiencyNotes(),
		GoalLegend:  uc.goalLegend(),
	}
	if uc.coreClient == nil {
		return proj, nil
	}

	groups := map[groupKey]*groupAcc{}
	getGroup := func(t CommsTags) *groupAcc {
		k := groupKey{Campaign: t.CampaignID, Stage: t.TrailStage, Variant: t.Variant, Tier: t.Tier}
		g := groups[k]
		if g == nil {
			g = newGroupAcc(k, uc.specFor(t.TrailStage))
			groups[k] = g
		}
		return g
	}

	// --- Sends + holdouts → group counts + attribution touches ---
	var touches []touch
	sends, err := uc.pageComms(ctx, MessageSentEventType)
	if err != nil {
		return nil, fmt.Errorf("comms-efficiency: read sends: %w", err)
	}
	for _, e := range sends {
		if boolField(e.Payload, "skipped") {
			continue // opt-out / rate-limit skips are not sends
		}
		tags := commsTagsFromPayload(e.Payload)
		g := getGroup(tags)
		g.sent++
		if ts, ok := parseEventTime(tags.SendTS, e.Timestamp); ok {
			touches = append(touches, touch{
				tenant: tags.TenantID, goalEvent: g.spec.GoalEvent, ts: ts,
				window: time.Duration(g.spec.WindowDays) * 24 * time.Hour, holdout: false, key: g.key,
			})
		}
	}
	holdouts, err := uc.pageComms(ctx, HoldoutEventType)
	if err != nil {
		return nil, fmt.Errorf("comms-efficiency: read holdouts: %w", err)
	}
	for _, e := range holdouts {
		tags := commsTagsFromPayload(e.Payload)
		g := getGroup(tags)
		g.heldOut++
		if ts, ok := parseEventTime(tags.SendTS, e.Timestamp); ok {
			touches = append(touches, touch{
				tenant: tags.TenantID, goalEvent: g.spec.GoalEvent, ts: ts,
				window: time.Duration(g.spec.WindowDays) * 24 * time.Hour, holdout: true, key: g.key,
			})
		}
	}

	// --- Engagement → distinct-message-id sets per group ---
	engagementSets := []struct {
		typ string
		sel func(*groupAcc) map[string]struct{}
	}{
		{EmailDeliveredEventType, func(g *groupAcc) map[string]struct{} { return g.delivered }},
		{EmailOpenedEventType, func(g *groupAcc) map[string]struct{} { return g.opened }},
		{EmailClickedEventType, func(g *groupAcc) map[string]struct{} { return g.clicked }},
		{EmailBouncedEventType, func(g *groupAcc) map[string]struct{} { return g.bounced }},
		{EmailUnsubscribedEventType, func(g *groupAcc) map[string]struct{} { return g.unsub }},
		{EmailComplainedEventType, func(g *groupAcc) map[string]struct{} { return g.complaint }},
	}
	for _, es := range engagementSets {
		events, err := uc.pageComms(ctx, es.typ)
		if err != nil {
			return nil, fmt.Errorf("comms-efficiency: read %s: %w", es.typ, err)
		}
		for _, e := range events {
			tags := commsTagsFromPayload(e.Payload)
			if tags.MessageID == "" {
				continue
			}
			es.sel(getGroup(tags))[tags.MessageID] = struct{}{}
		}
	}

	// --- Attribution: last-touch within the window, per (tenant, goalEvent) ---
	uc.attribute(ctx, touches, groups)

	// --- Finalize groups + hero ---
	for _, g := range groups {
		proj.Groups = append(proj.Groups, finalizeGroup(g))
	}
	sort.Slice(proj.Groups, func(i, j int) bool {
		a, b := proj.Groups[i], proj.Groups[j]
		if a.Campaign != b.Campaign {
			return a.Campaign < b.Campaign
		}
		if a.Stage != b.Stage {
			return a.Stage < b.Stage
		}
		if a.Variant != b.Variant {
			return a.Variant < b.Variant
		}
		return a.Tier < b.Tier
	})
	proj.Hero = heroFrom(proj.Groups)
	return proj, nil
}

// attribute resolves the last-touch goal credit for every touch and folds the
// result into its group. touches are grouped by (tenant, goalEvent); within each
// group they are sorted ascending so a later touch "owns" goals at/after its time.
func (uc *CommsEfficiencyUseCase) attribute(ctx context.Context, touches []touch, groups map[groupKey]*groupAcc) {
	type tgKey struct{ tenant, goal string }
	byTenantGoal := map[tgKey][]touch{}
	for _, t := range touches {
		k := tgKey{t.tenant, t.goalEvent}
		byTenantGoal[k] = append(byTenantGoal[k], t)
	}

	// A tenant that no longer exists churned mid-window — its goal stream is
	// unreadable. Resolve once per tenant so the whole reconcile degrades (never
	// errors) on a deleted tenant.
	churnedTenant := map[string]bool{}
	isChurned := func(tenant string) bool {
		if v, ok := churnedTenant[tenant]; ok {
			return v
		}
		_, err := uc.tenantRepo.FindByID(tenant)
		churned := err != nil
		churnedTenant[tenant] = churned
		return churned
	}

	for k, ts := range byTenantGoal {
		sort.Slice(ts, func(i, j int) bool { return ts[i].ts.Before(ts[j].ts) })
		for i := range ts {
			t := ts[i]
			g := groups[t.key]
			if isChurned(k.tenant) {
				g.churned++
				continue
			}
			windowEnd := t.ts.Add(t.window)
			// Exclusive upper bound: a later touch owns goals at/after its timestamp
			// (last-touch attribution). The final touch keeps the full window.
			upper := windowEnd
			last := true
			if i+1 < len(ts) {
				if next := ts[i+1].ts; next.Before(upper) {
					upper = next
					last = false
				}
			}
			earliest, found, ok := uc.earliestGoal(ctx, k.tenant, t.goalEvent, t.ts, windowEnd)
			if !ok {
				// Stream unreadable despite the tenant existing — treat as churned.
				g.churned++
				continue
			}
			if !found {
				continue // miss (no goal in window)
			}
			credited := earliest.Before(upper) || (last && !earliest.After(windowEnd))
			if !credited {
				continue // a later touch (last-touch) owns this goal
			}
			if t.holdout {
				g.holdoutConverted++
			} else {
				g.converted++
				g.ttg = append(g.ttg, earliest.Sub(t.ts))
			}
		}
	}
}

// earliestGoal returns the earliest goal event timestamp in [from, to] for the
// tenant's own stream. goalEvent == GoalAnyEvent ("") means any product event
// (activation/reactivation). ok=false signals an unreadable stream (churned).
func (uc *CommsEfficiencyUseCase) earliestGoal(ctx context.Context, tenant, goalEvent string, from, to time.Time) (ts time.Time, found, ok bool) {
	req := clients.QueryEventsRequest{
		TenantID: tenant,
		Since:    from.UTC().Format(time.RFC3339),
		Until:    to.UTC().Format(time.RFC3339),
		Order:    "asc",
		Limit:    1,
	}
	if goalEvent != GoalAnyEvent {
		req.EventType = goalEvent
	}
	resp, err := uc.coreClient.QueryEvents(ctx, req)
	if err != nil {
		return time.Time{}, false, false
	}
	if resp == nil || len(resp.Events) == 0 {
		return time.Time{}, false, true
	}
	gt, parseOK := parseEventTime("", resp.Events[0].Timestamp)
	if !parseOK {
		return time.Time{}, false, true
	}
	// Guard against a Since boundary that returns an event before `from` (a goal
	// strictly before the send must NOT credit it).
	if gt.Before(from) {
		return time.Time{}, false, true
	}
	return gt, true, true
}

// pageComms reads every event of a type from the admin-comms tenant, paging until
// a short page returns.
func (uc *CommsEfficiencyUseCase) pageComms(ctx context.Context, eventType string) ([]clients.EventEntry, error) {
	var out []clients.EventEntry
	offset := 0
	for {
		events, err := uc.recorder.queryComms(ctx, clients.QueryEventsRequest{
			EventType: eventType,
			Limit:     commsEfficiencyEventLimit,
			Offset:    offset,
		})
		if err != nil {
			return nil, err
		}
		out = append(out, events...)
		if len(events) < commsEfficiencyEventLimit {
			break
		}
		offset += commsEfficiencyEventLimit
	}
	return out, nil
}

func (uc *CommsEfficiencyUseCase) specFor(stage string) GoalSpec {
	if uc.goalMap != nil {
		if spec, ok := uc.goalMap[stage]; ok {
			return spec
		}
	}
	return goalSpecFor(stage)
}

func (uc *CommsEfficiencyUseCase) goalLegend() []GoalLegendEntry {
	out := make([]GoalLegendEntry, 0, len(uc.goalMap))
	for stage, spec := range uc.goalMap {
		out = append(out, GoalLegendEntry{Stage: stage, GoalEvent: spec.GoalEvent, State: spec.State, Note: spec.Note})
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Stage < out[j].Stage })
	return out
}

// ----------------------------------------------------------------------------
// Finalize + hero + helpers
// ----------------------------------------------------------------------------

func finalizeGroup(g *groupAcc) EfficiencyGroup {
	delivered := len(g.delivered)
	out := EfficiencyGroup{
		Campaign: g.key.Campaign, Stage: g.key.Stage, Variant: g.key.Variant, Tier: g.key.Tier,
		GoalEvent: g.spec.GoalEvent, GoalState: g.spec.State, GoalNote: g.spec.Note, WindowDays: g.spec.WindowDays,
		Sent: g.sent, HeldOut: g.heldOut, Delivered: delivered,
		Opened: len(g.opened), Clicked: len(g.clicked), Bounced: len(g.bounced),
		Unsub: len(g.unsub), Complained: len(g.complaint), Churned: g.churned,
		Converted: g.converted, HoldoutConverted: g.holdoutConverted,
		TimeToGoalMedianSec: int64(medianDuration(g.ttg).Seconds()),
	}
	if delivered > 0 {
		out.OpenRate = ratio(out.Opened, delivered)
		out.ClickRate = ratio(out.Clicked, delivered)
		out.ConversionRate = ratio(g.converted, delivered)
		out.UnsubRate = ratio(out.Unsub, delivered)
		out.ComplaintRate = ratio(out.Complained, delivered)
	}
	if g.sent > 0 {
		out.ConvertSent = ratio(g.converted, g.sent)
	}
	if g.heldOut > 0 {
		out.ConvertHoldout = ratio(g.holdoutConverted, g.heldOut)
	}
	out.Lift = out.ConvertSent - out.ConvertHoldout
	return out
}

// heroFrom aggregates the trial→paid funnel across every paid-welcome (hero) group.
func heroFrom(groups []EfficiencyGroup) TrialToPaidHero {
	h := TrialToPaidHero{GoalEvent: GoalSubscriptionActivated}
	var ttgTotalSec, ttgN int64
	for _, g := range groups {
		if g.GoalEvent != GoalSubscriptionActivated {
			continue
		}
		h.Sent += g.Sent
		h.HeldOut += g.HeldOut
		h.Delivered += g.Delivered
		h.Clicked += g.Clicked
		h.Converted += g.Converted
		h.HoldoutConverted += g.HoldoutConverted
		if g.TimeToGoalMedianSec > 0 && g.Converted > 0 {
			ttgTotalSec += g.TimeToGoalMedianSec * int64(g.Converted)
			ttgN += int64(g.Converted)
		}
	}
	if h.Sent > 0 {
		h.ConversionRate = ratio(h.Converted, h.Sent)
	}
	if h.HeldOut > 0 {
		h.HoldoutRate = ratio(h.HoldoutConverted, h.HeldOut)
		h.HasHoldout = true
	}
	h.Lift = h.ConversionRate - h.HoldoutRate
	if ttgN > 0 {
		h.TimeToGoalMedSec = ttgTotalSec / ttgN
	}
	return h
}

func efficiencyNotes() []string {
	return []string{
		"Clicks and goal conversion LEAD. Opens are UNRELIABLE — Apple Mail Privacy Protection pre-fetches images, inflating opens — so open-rate is shown but never optimized on.",
		"Lift = conversion(sent) − conversion(holdout) on the intent-to-treat basis (goal / cohort size); it is the only causal number here. Conversion-rate uses goal / delivered (the delivery funnel).",
		"Trial→paid (subscription.activated within the window) is the hero metric. The free tier is retired — there is no free segment.",
		"Churned tenants (deleted mid-window) are counted in the delivery funnel but excluded from conversion/lift — their goal stream can't be read, so conversion is a floor.",
	}
}

func ratio(num, den int) float64 {
	if den <= 0 {
		return 0
	}
	return float64(num) / float64(den)
}

func medianDuration(d []time.Duration) time.Duration {
	if len(d) == 0 {
		return 0
	}
	s := append([]time.Duration(nil), d...)
	sort.Slice(s, func(i, j int) bool { return s[i] < s[j] })
	n := len(s)
	if n%2 == 1 {
		return s[n/2]
	}
	return (s[n/2-1] + s[n/2]) / 2
}

// parseEventTime parses an RFC3339(Nano) timestamp, preferring a payload-supplied
// value and falling back to Core's server-assigned timestamp.
func parseEventTime(preferred, fallback string) (time.Time, bool) {
	for _, s := range []string{preferred, fallback} {
		if s == "" {
			continue
		}
		if t, err := time.Parse(time.RFC3339Nano, s); err == nil {
			return t.UTC(), true
		}
		if t, err := time.Parse(time.RFC3339, s); err == nil {
			return t.UTC(), true
		}
	}
	return time.Time{}, false
}

// ----------------------------------------------------------------------------
// ExecuteAll: the scheduled half — compute + persist the projection + audit.
// ----------------------------------------------------------------------------

// ExecuteAll computes the projection and writes it back to Core config (the
// operator-side projection the API reads) plus a durable snapshot event + audit.
// Mirrors the billing sync_* reconcilers' compute-then-write-back shape.
func (uc *CommsEfficiencyUseCase) ExecuteAll(ctx context.Context) (*EfficiencyProjection, error) {
	proj, err := uc.Compute(ctx)
	if err != nil {
		return nil, err
	}
	if uc.coreClient != nil {
		if b, mErr := json.Marshal(proj); mErr == nil {
			if sErr := uc.coreClient.SetConfig(ctx, clients.SetConfigRequest{
				Key:       commsEfficiencyProjectionKey,
				Value:     string(b),
				ChangedBy: "comms-efficiency-reconciler",
			}); sErr != nil {
				log.Printf("CommsEfficiency: persist projection failed: %v", sErr)
			}
		}
		// Durable snapshot event for history (under admin-comms).
		_, _ = uc.recorder.record(ctx, "comms.efficiency.snapshot", "efficiency:latest", map[string]any{ //nolint:errcheck // history best-effort
			"generated_at": proj.GeneratedAt,
			"groups":       len(proj.Groups),
			"hero_sent":    proj.Hero.Sent,
			"hero_lift":    proj.Hero.Lift,
		})
	}
	if uc.auditRepo != nil {
		auditEvent, _ := entities.NewAuditEvent("comms.efficiency.reconciled", "execute", "SCHEDULER", "/comms/efficiency") //nolint:errcheck
		auditEvent.AddMetadata("groups", fmt.Sprintf("%d", len(proj.Groups)))
		auditEvent.AddMetadata("hero_converted", fmt.Sprintf("%d", proj.Hero.Converted))
		_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
	}
	return proj, nil
}

// Latest returns the projection the API serves: the cached projection the
// reconciler last wrote, falling back to a live Compute when none is cached yet
// (so the panel is never empty before the first scheduled run).
func (uc *CommsEfficiencyUseCase) Latest(ctx context.Context) (*EfficiencyProjection, error) {
	if uc.coreClient != nil {
		if entry, err := uc.coreClient.GetConfig(ctx, commsEfficiencyProjectionKey); err == nil && entry != nil {
			if s, ok := entry.Value.(string); ok && s != "" {
				var proj EfficiencyProjection
				if json.Unmarshal([]byte(s), &proj) == nil {
					return &proj, nil
				}
			}
		}
	}
	return uc.Compute(ctx)
}
