package usecases

import (
	"fmt"
	"time"
)

// CommsCategory classifies a message for opt-out purposes. Operational messages
// (you're at quota, your sync stalled, your payment failed) can be exempt from a
// tenant's opt-out because they are service-critical; marketing/nudge messages
// always honor opt-out (ADMIN_TENANT_POWER_TOOL §4 Pillar C "per-category, e.g.
// marketing vs operational — operational/critical can be exempt").
type CommsCategory string

const (
	// CategoryOperational — service-critical; bypasses opt-out by default.
	CategoryOperational CommsCategory = "operational"
	// CategoryMarketing — nudges/outreach; always honors opt-out.
	CategoryMarketing CommsCategory = "marketing"
)

// CommsTemplate is one operator template keyed to a health/lifecycle event. It
// renders a subject + plain-text body from the tenant + a small render context,
// and declares its category (opt-out behavior) + cooldown (rate-limit window).
type CommsTemplate struct {
	// Key is the stable template id echoed in admin.message.sent and used as the
	// rate-limit cooldown key (one cooldown per tenant+template).
	Key string
	// Category drives opt-out: operational templates bypass a plain opt-out;
	// marketing templates are skipped for an opted-out tenant.
	Category CommsCategory
	// Cooldown is the minimum spacing between two sends of THIS template to the
	// SAME tenant. A repeat within the window is rate-limited (skipped_rate_limit).
	Cooldown time.Duration
	// Render builds the subject + body. ctx carries optional render values
	// (tenant name, usage, tier, …); a template uses only what it needs.
	Render func(ctx CommsRenderContext) (subject, body string)
}

// CommsRenderContext is the data a template may interpolate. All fields are
// optional; a template guards anything it reads. The values come from the
// tenant record + (where relevant) its health/usage context, never from
// client-trusted input on the message send path.
type CommsRenderContext struct {
	TenantName  string
	Tier        string
	EventsUsed  int64
	EventsQuota int64
	HealthTier  string // signals tier ("at_risk"/"critical"/…) when known
	// Custom lets a one-off operator message ride the same send/audit path with a
	// caller-supplied subject+body (template "custom").
	CustomSubject string
	CustomBody    string
}

// CommsTemplates is the registry of operator templates (§4 Pillar C templates:
// at_risk_outreach, quota_warning, onboarding_nudge, dunning_reminder), plus a
// "custom" pass-through so an operator can send a bespoke subject+body through
// the same opt-out + rate-limit + audit machinery.
var CommsTemplates = map[string]CommsTemplate{
	"at_risk_outreach": {
		Key:      "at_risk_outreach",
		Category: CategoryMarketing,
		Cooldown: 7 * 24 * time.Hour, // at most one outreach/week per tenant
		Render: func(ctx CommsRenderContext) (string, string) {
			name := nameOrThere(ctx.TenantName)
			subject := "Checking in on your AllSource workspace"
			body := fmt.Sprintf(`Hi %s,

We noticed your AllSource workspace could use a hand getting fully set up.
A few minutes now usually unblocks the rest — and we're happy to help.

Reply to this email and we'll get you going, or read the quickstart at:
https://all-source.xyz/docs/quickstart

Best,
The AllSource Chronos Team
`, name)
			return subject, body
		},
	},
	"quota_warning": {
		Key:      "quota_warning",
		Category: CategoryOperational, // service-critical: usage about to be capped
		Cooldown: 24 * time.Hour,      // no more than one quota_warning per day
		Render: func(ctx CommsRenderContext) (string, string) {
			name := nameOrThere(ctx.TenantName)
			pct := 0
			if ctx.EventsQuota > 0 {
				pct = int(ctx.EventsUsed * 100 / ctx.EventsQuota)
			}
			subject := "AllSource Chronos: You're approaching your event quota"
			body := fmt.Sprintf(`Hi %s,

You've used %d%% of your monthly event quota (%d / %d events).

Events ingested beyond your quota may be subject to overage charges or
throttling depending on your plan. To raise your quota, upgrade at:
https://all-source.xyz/billing

Best,
The AllSource Chronos Team
`, name, pct, ctx.EventsUsed, ctx.EventsQuota)
			return subject, body
		},
	},
	"onboarding_nudge": {
		Key:      "onboarding_nudge",
		Category: CategoryMarketing,
		Cooldown: 3 * 24 * time.Hour, // gentle: at most one nudge every 3 days
		Render: func(ctx CommsRenderContext) (string, string) {
			name := nameOrThere(ctx.TenantName)
			subject := "Send your first event to AllSource Chronos"
			body := fmt.Sprintf(`Hi %s,

Your AllSource workspace is ready, but we haven't seen any events yet.
Sending your first event takes one API call:

  curl -X POST https://api.all-source.xyz/api/v1/events \
    -H "Authorization: Bearer <your-api-key>" \
    -d '{"event_type":"hello.world","entity_id":"1","payload":{}}'

Full quickstart: https://all-source.xyz/docs/quickstart

Best,
The AllSource Chronos Team
`, name)
			return subject, body
		},
	},
	"dunning_reminder": {
		Key:      "dunning_reminder",
		Category: CategoryOperational, // service-critical: payment failed
		Cooldown: 24 * time.Hour,
		Render: func(ctx CommsRenderContext) (string, string) {
			name := nameOrThere(ctx.TenantName)
			subject := "Action needed: update your AllSource billing details"
			body := fmt.Sprintf(`Hi %s,

We weren't able to process the payment for your AllSource subscription.
To avoid any interruption, please update your billing details:

https://all-source.xyz/billing

If you've already fixed this, thank you — you can ignore this message.

Best,
The AllSource Chronos Team
`, name)
			return subject, body
		},
	},
	"custom": {
		Key:      "custom",
		Category: CategoryMarketing, // a bespoke operator message honors opt-out by default
		Cooldown: 0,                 // no cooldown — operator-driven, one-off
		Render: func(ctx CommsRenderContext) (string, string) {
			return ctx.CustomSubject, ctx.CustomBody
		},
	},
}

// LookupTemplate returns the template by key (ok=false if unknown).
func LookupTemplate(key string) (CommsTemplate, bool) {
	t, ok := CommsTemplates[key]
	return t, ok
}

func nameOrThere(name string) string {
	if name == "" {
		return "there"
	}
	return name
}
