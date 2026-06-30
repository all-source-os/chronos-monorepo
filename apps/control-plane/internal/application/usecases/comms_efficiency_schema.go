package usecases

import (
	"crypto/sha256"
	"encoding/binary"
	"strings"
)

// Proactive-comms efficiency engine — event schema, correlation tags, goal-event
// map, and the deterministic holdout splitter (prompt 050).
//
// THE DOGFOOD: efficiency is a temporal join over the tenant's OWN Core event
// stream — email engagement events ⋈ goal events — computed by a control-plane
// reconciler (CommsEfficiencyUseCase). There is NO parallel analytics stack.
//
// WHERE THINGS LIVE (read CLAUDE.md isolation boundary):
//   - Sends (admin.message.sent), holdouts (comms.holdout), and engagement
//     (email.delivered/opened/clicked/bounced/complained/unsubscribed) are stored
//     under the admin-comms OPERATOR tenant (CommsAuditTenant), each carrying the
//     CUSTOMER tenant_id as a payload tag. This is the operator-level, cross-tenant
//     analytic substrate — it is NOT per-tenant read-model compute in Core, and it
//     keeps customer streams uncluttered by marketing telemetry.
//   - Goal events (subscription.activated, the customer's own product events, …)
//     live in each CUSTOMER tenant's stream. The reconciler joins the two by
//     tenant_id + the attribution window.

// Comms engagement + holdout Core event types. The send event (admin.message.sent)
// is defined in comms_audit.go (MessageSentEventType) — every send AND every
// engagement/holdout event carries the SAME correlation tags so the join exists.
const (
	// EmailDeliveredEventType — ESP confirmed delivery to the recipient MTA.
	EmailDeliveredEventType = "email.delivered"
	// EmailOpenedEventType — recipient opened (UNRELIABLE: Apple MPP pre-fetches
	// images → inflated/instant opens; the engine subordinates this signal).
	EmailOpenedEventType = "email.opened"
	// EmailClickedEventType — recipient clicked a tracked link (LEAD signal).
	EmailClickedEventType = "email.clicked"
	// EmailBouncedEventType — hard/soft bounce (deliverability cost).
	EmailBouncedEventType = "email.bounced"
	// EmailComplainedEventType — spam complaint (reputation cost).
	EmailComplainedEventType = "email.complained"
	// EmailUnsubscribedEventType — list-unsubscribe (audience cost).
	EmailUnsubscribedEventType = "email.unsubscribed"
	// HoldoutEventType — a deterministically held-out would-send. Suppresses the
	// actual send but records the would-be recipient with the same tags, so
	// conversion(sent) vs conversion(holdout) — i.e. causal lift — is measurable.
	HoldoutEventType = "comms.holdout"
)

// Goal Core event types the reconciler joins against in the CUSTOMER tenant
// stream. subscription.activated / subscription.upgraded are emitted by the
// subscription apply path (update_subscription_metadata.go) so the HERO trial→paid
// funnel is real today. x402.payment.settled already exists. The empty sentinel
// (GoalAnyEvent) means "any product event in the tenant's own stream" — used for
// activation / reactivation ("did the nudge get them ingesting again?").
const (
	// GoalSubscriptionActivated marks the first transition into a PAID tier
	// (trial/free → indie+). THE hero conversion. Emitted server-side, never client.
	GoalSubscriptionActivated = "subscription.activated"
	// GoalSubscriptionUpgraded marks a paid→higher-paid tier change (expansion).
	GoalSubscriptionUpgraded = "subscription.upgraded"
	// GoalX402Settled is a settled x402 pay-per-call event (continued agent usage).
	GoalX402Settled = "x402.payment.settled"
	// GoalAnyEvent ("") means any event in the customer's own stream — activation
	// ("first event") and reactivation ("an event after going quiet").
	GoalAnyEvent = ""
)

// Correlation tag KEYS. EVERY send, holdout, and engagement event payload carries
// these so the reconciler can group by campaign/stage/variant/tier and join to
// goals by tenant. WITHOUT shared tags there is no join.
const (
	TagTenantID   = "tenant_id"
	TagCampaignID = "campaign_id"
	TagMessageID  = "message_id" // ESP message id; the engagement join key
	TagTrailStage = "trail_stage"
	TagVariant    = "variant"
	TagCohort     = "cohort"
	TagTier       = "tier"     // tenant billing tier at send time (segmentation; NO free)
	TagSendTS     = "send_ts"  // RFC3339 send/holdout timestamp (window anchor)
	TagHoldout    = "holdout"  // bool
	TagEventTS    = "event_ts" // engagement-only: ESP event timestamp
	TagLink       = "link"     // engagement-only (clicks): the clicked URL
)

// CommsTags is the shared correlation envelope stamped on every send, holdout, and
// engagement event. It is GENERAL — it describes ANY proactive-comms send, not just
// the lifecycle trail.
type CommsTags struct {
	TenantID   string `json:"tenant_id"`
	CampaignID string `json:"campaign_id"`
	MessageID  string `json:"message_id,omitempty"`
	TrailStage string `json:"trail_stage,omitempty"`
	Variant    string `json:"variant,omitempty"`
	Cohort     string `json:"cohort,omitempty"`
	Tier       string `json:"tier,omitempty"`
	SendTS     string `json:"send_ts,omitempty"`
	Holdout    bool   `json:"holdout"`
}

// ApplyTo stamps the correlation tags onto an event payload map (merging, never
// clobbering caller keys it doesn't own).
func (t CommsTags) ApplyTo(payload map[string]any) {
	payload[TagTenantID] = t.TenantID
	payload[TagCampaignID] = t.CampaignID
	if t.MessageID != "" {
		payload[TagMessageID] = t.MessageID
	}
	if t.TrailStage != "" {
		payload[TagTrailStage] = t.TrailStage
	}
	if t.Variant != "" {
		payload[TagVariant] = t.Variant
	}
	if t.Cohort != "" {
		payload[TagCohort] = t.Cohort
	}
	if t.Tier != "" {
		payload[TagTier] = t.Tier
	}
	if t.SendTS != "" {
		payload[TagSendTS] = t.SendTS
	}
	payload[TagHoldout] = t.Holdout
}

// commsTagsFromPayload reconstructs the correlation tags from an event payload
// (tolerant of JSON float/bool). Used by the reconciler to group + join.
func commsTagsFromPayload(p map[string]any) CommsTags {
	return CommsTags{
		TenantID:   stringField(p, TagTenantID),
		CampaignID: stringField(p, TagCampaignID),
		MessageID:  stringField(p, TagMessageID),
		TrailStage: stringField(p, TagTrailStage),
		Variant:    stringField(p, TagVariant),
		Cohort:     stringField(p, TagCohort),
		Tier:       stringField(p, TagTier),
		SendTS:     stringField(p, TagSendTS),
		Holdout:    boolField(p, TagHoldout),
	}
}

// commsCorrelationKey is the Core config key holding a send's correlation tags,
// keyed by the ESP message id. The send path writes it; the engagement webhook
// reads it to resolve message_id → (tenant, campaign, variant, …). Mirrors the
// grant-config / wallet-lookup Core-config patterns (no new store).
func commsCorrelationKey(messageID string) string {
	return "comms:correlation:" + messageID
}

// commsEngagementEntityID is the idempotency key for an engagement event:
// (message_id, type). ExpectedVersion=0 on this entity makes a replayed ESP
// webhook a no-op (VersionConflict → duplicate), and collapses repeat opens/clicks
// of one message to a single funnel event — exactly the per-recipient binary the
// funnel wants.
func commsEngagementEntityID(messageID, eventType string) string {
	slug := strings.TrimPrefix(eventType, "email.")
	return "comms:engage:" + messageID + ":" + slug
}

// ----------------------------------------------------------------------------
// Goal-event map (seeded from docs/marketing/lifecycle-email-trail.md §3, prompt 022)
// ----------------------------------------------------------------------------

// GoalState marks whether the goal signal fires today or still needs a new signal.
// Surfaced honestly in the panel so an operator never trusts a dead metric.
type GoalState string

const (
	// GoalReal — the goal Core event exists and fires today.
	GoalReal GoalState = "real"
	// GoalNeedsSignal — no event fires this goal yet (named in the trail doc §3).
	GoalNeedsSignal GoalState = "needs_signal"
)

// GoalSpec maps a campaign/trail-stage to its goal Core event + attribution window.
type GoalSpec struct {
	// GoalEvent is the Core event_type that signals success in the customer's own
	// stream. GoalAnyEvent ("") means "any product event" (activation/reactivation).
	GoalEvent string
	// WindowDays is the attribution window N: a goal only credits a send if it lands
	// within [send_ts, send_ts+WindowDays].
	WindowDays int
	// State flags real-today vs needs-new-signal.
	State GoalState
	// Hero marks the trial→paid funnel (the headline metric on the panel).
	Hero bool
	// Note is the honest caveat shown when the goal needs a new signal.
	Note string
}

// DefaultGoalAttributionWindowDays is the window used when a stage is not in the
// map. 7 days is the trail's default lifecycle/marketing cadence (§2 weekly cap).
const DefaultGoalAttributionWindowDays = 7

// DefaultGoalMap is the seed goal-event map, lifted from the trail's trigger→signal
// table (lifecycle-email-trail.md §3). Keyed by trail_stage. REUSED, not reinvented.
//
// REAL today: signup/activation-first-event/dormancy use "any event"; paid_welcome
// uses subscription.activated (emitted by the subscription apply path); quota
// expansion uses subscription.upgraded; x402 uses x402.payment.settled.
// NEEDS NEW SIGNAL (flagged): activation_recall (A2), mcp_intro (A3), depth (E5).
var DefaultGoalMap = map[string]GoalSpec{
	// Signup → ingest first event (S1). REAL — any event in the tenant's stream.
	"signup": {GoalEvent: GoalAnyEvent, WindowDays: 3, State: GoalReal},
	// Activation: first event (A1). REAL.
	"activation_first_event": {GoalEvent: GoalAnyEvent, WindowDays: 2, State: GoalReal},
	// Activation: first recall/query (A2). queries are READS, not events → needs signal.
	"activation_recall": {GoalEvent: GoalAnyEvent, WindowDays: 7, State: GoalNeedsSignal,
		Note: "A2: queries_used is a read meter, not a Core event — add a first-recall milestone event (mirror check_usage_warnings)."},
	// Activation: first MCP read (A3). No per-tenant first-MCP-read signal → needs signal.
	"mcp_intro": {GoalEvent: GoalAnyEvent, WindowDays: 14, State: GoalNeedsSignal,
		Note: "A3: no first-MCP-read signal exists — add an MCP-usage marker or approximate from the MCP query path."},
	// Paid welcome (P-Indie/Studio/Scale). HERO: trial→paid. REAL via subscription.activated.
	"paid_welcome": {GoalEvent: GoalSubscriptionActivated, WindowDays: 14, State: GoalReal, Hero: true},
	// Expansion: events at 80/100% (E1/E2) → upgrade. REAL via subscription.upgraded.
	"expansion_quota": {GoalEvent: GoalSubscriptionUpgraded, WindowDays: 7, State: GoalReal},
	// Expansion: x402 allowance (E3) → continued/settled x402 usage. REAL.
	"expansion_x402": {GoalEvent: GoalX402Settled, WindowDays: 14, State: GoalReal},
	// Expansion: unused depth / feature-gap first-use (E5). Needs a feature-adoption signal.
	"depth": {GoalEvent: GoalAnyEvent, WindowDays: 14, State: GoalNeedsSignal,
		Note: "E5: first-use of a specific priced feature has no event yet — add a feature-adoption marker."},
	// Dormancy: stream went quiet / still here? (D1/D2) → any new event. REAL.
	"dormancy": {GoalEvent: GoalAnyEvent, WindowDays: 7, State: GoalReal},
	// Dormancy: paid but dormant pre-renewal (D3) → re-activation (any event). REAL.
	"dormancy_paid": {GoalEvent: GoalAnyEvent, WindowDays: 14, State: GoalReal},
}

// goalSpecFor resolves the goal spec for a trail stage, falling back to a generic
// "any event" goal at the default window for an unmapped/empty stage (so a generic
// proactive-comms send is still measured for reactivation).
func goalSpecFor(stage string) GoalSpec {
	if spec, ok := DefaultGoalMap[stage]; ok {
		return spec
	}
	return GoalSpec{GoalEvent: GoalAnyEvent, WindowDays: DefaultGoalAttributionWindowDays, State: GoalReal}
}

// ----------------------------------------------------------------------------
// Deterministic holdout splitter (causal lift)
// ----------------------------------------------------------------------------

// HoldoutAssignment deterministically decides whether (tenant, campaign) is in the
// holdout for a campaign whose holdout percentage is pct (0–100). It hashes
// tenant_id+campaign_id so the SAME pair always lands the SAME side of the split —
// stable and reproducible across reconciler runs (the property lift depends on).
//
// pct ≤ 0 → never held out; pct ≥ 100 → always. The bucket is the first 8 bytes of
// SHA-256(tenant|campaign) mod 100; held out iff bucket < pct.
func HoldoutAssignment(tenantID, campaignID string, pct int) bool {
	if pct <= 0 {
		return false
	}
	if pct >= 100 {
		return true
	}
	sum := sha256.Sum256([]byte(tenantID + "|" + campaignID))
	bucket := binary.BigEndian.Uint64(sum[:8]) % 100
	return bucket < uint64(pct)
}
