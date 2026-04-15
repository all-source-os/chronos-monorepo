package billing

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Warning thresholds as percentage of quota.
const (
	WarningThreshold80  = 80
	WarningThreshold100 = 100
)

// UsageWarningResult holds the result of checking a single tenant's usage.
type UsageWarningResult struct {
	TenantID     string
	EmailSent    bool
	ThresholdPct int // 80 or 100 (0 if below threshold)
	Skipped      bool
	Error        error
}

// CheckUsageWarningsUseCase checks tenant usage against quota thresholds
// and sends warning emails when thresholds are crossed.
type CheckUsageWarningsUseCase struct {
	tenantRepo  repositories.TenantRepository
	auditRepo   repositories.AuditRepository
	emailClient clients.EmailClient
}

// NewCheckUsageWarningsUseCase creates a new CheckUsageWarningsUseCase.
func NewCheckUsageWarningsUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	emailClient clients.EmailClient,
) *CheckUsageWarningsUseCase {
	return &CheckUsageWarningsUseCase{
		tenantRepo:  tenantRepo,
		auditRepo:   auditRepo,
		emailClient: emailClient,
	}
}

// Execute checks a single tenant's usage and sends warning emails if thresholds are crossed.
func (uc *CheckUsageWarningsUseCase) Execute(ctx context.Context, tenantID string) (*UsageWarningResult, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, fmt.Errorf("find tenant %s: %w", tenantID, err)
	}

	quotas := extractQuotas(tenant.Metadata)
	if quotas.EventsQuota <= 0 {
		return &UsageWarningResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Determine current usage percentage
	usagePct := int(quotas.EventsUsed * 100 / quotas.EventsQuota)

	// Determine which threshold we've crossed
	var thresholdPct int
	switch {
	case usagePct >= WarningThreshold100:
		thresholdPct = WarningThreshold100
	case usagePct >= WarningThreshold80:
		thresholdPct = WarningThreshold80
	default:
		return &UsageWarningResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Check if we already sent a warning for this threshold in this billing period
	warnings := extractWarnings(tenant.Metadata)
	if warnings.LastWarningPct >= thresholdPct {
		return &UsageWarningResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Extract email from tenant metadata (set during onboarding/registration)
	email := extractTenantEmail(tenant.Metadata)
	if email == "" {
		log.Printf("UsageWarnings: no email for tenant %s, skipping", tenantID)
		return &UsageWarningResult{TenantID: tenantID, Skipped: true}, nil
	}

	// Determine tier for upgrade CTA
	tier := extractSubscriptionTier(tenant.Metadata)

	// Send warning email
	subject, body := buildWarningEmail(thresholdPct, quotas, tier, tenant.Name)
	err = uc.emailClient.SendEmail(ctx, clients.SendEmailRequest{
		To:      email,
		Subject: subject,
		Body:    body,
	})
	if err != nil {
		return nil, fmt.Errorf("send warning email for %s: %w", tenantID, err)
	}

	// Update tenant metadata with last warning sent
	warnings.LastWarningPct = thresholdPct
	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}
	tenant.Metadata["usage_warnings"] = &warnings
	if err := uc.tenantRepo.Update(tenant); err != nil {
		log.Printf("UsageWarnings: failed to update warning state for %s: %v", tenantID, err)
	}

	// Audit
	auditEvent, _ := entities.NewAuditEvent("billing.usage_warning.sent", "notify", "SCHEDULER", "/billing/usage-warning") //nolint:errcheck
	auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("threshold_pct", fmt.Sprintf("%d", thresholdPct))
	auditEvent.AddMetadata("events_used", fmt.Sprintf("%d", quotas.EventsUsed))
	auditEvent.AddMetadata("events_quota", fmt.Sprintf("%d", quotas.EventsQuota))
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return &UsageWarningResult{
		TenantID:     tenantID,
		EmailSent:    true,
		ThresholdPct: thresholdPct,
	}, nil
}

// ExecuteAll checks usage for all active tenants.
func (uc *CheckUsageWarningsUseCase) ExecuteAll(ctx context.Context) []UsageWarningResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("UsageWarnings: failed to list active tenants: %v", err)
		return nil
	}

	var results []UsageWarningResult
	for _, t := range tenants {
		result, err := uc.Execute(ctx, t.ID)
		if err != nil {
			results = append(results, UsageWarningResult{TenantID: t.ID, Error: err})
			continue
		}
		results = append(results, *result)
	}
	return results
}

// UsageWarningMetadata tracks which warnings have been sent in the current billing period.
type UsageWarningMetadata struct {
	LastWarningPct int    `json:"last_warning_pct"` // highest threshold sent (80 or 100)
	ResetDate      string `json:"reset_date,omitempty"`
}

func extractWarnings(metadata map[string]interface{}) UsageWarningMetadata {
	if metadata == nil {
		return UsageWarningMetadata{}
	}
	raw, ok := metadata["usage_warnings"]
	if !ok {
		return UsageWarningMetadata{}
	}
	switch v := raw.(type) {
	case *UsageWarningMetadata:
		return *v
	case UsageWarningMetadata:
		return v
	case map[string]interface{}:
		return warningsFromMap(v)
	default:
		return UsageWarningMetadata{}
	}
}

func warningsFromMap(m map[string]interface{}) UsageWarningMetadata {
	w := UsageWarningMetadata{}
	if v, ok := m["last_warning_pct"].(float64); ok {
		w.LastWarningPct = int(v)
	}
	if v, ok := m["reset_date"].(string); ok {
		w.ResetDate = v
	}
	return w
}

func extractTenantEmail(metadata map[string]interface{}) string {
	if metadata == nil {
		return ""
	}
	if email, ok := metadata["email"].(string); ok {
		return email
	}
	return ""
}

const (
	subjectQuotaReached      = "AllSource Chronos: You've reached your event quota"
	subjectQuotaApproaching  = "AllSource Chronos: You're approaching your event quota"
	subjectUsageNotification = "AllSource Chronos: Usage notification"
)

func buildWarningEmail(thresholdPct int, quotas entities.QuotaMetadata, tier, tenantName string) (subject, body string) {
	if tenantName == "" {
		tenantName = "there"
	}

	usagePct := int(quotas.EventsUsed * 100 / quotas.EventsQuota)

	switch thresholdPct {
	case WarningThreshold100:
		subject = subjectQuotaReached
		body = fmt.Sprintf(`Hi %s,

You've reached 100%% of your monthly event quota.

Current usage: %d / %d events (%d%% used)

Events ingested beyond your quota may be subject to overage charges
or throttling depending on your plan settings.

`, tenantName, quotas.EventsUsed, quotas.EventsQuota, usagePct)

	case WarningThreshold80:
		subject = subjectQuotaApproaching
		body = fmt.Sprintf(`Hi %s,

You've used 80%% of your monthly event quota.

Current usage: %d / %d events (%d%% used)

`, tenantName, quotas.EventsUsed, quotas.EventsQuota, usagePct)

	default:
		subject = subjectUsageNotification
		body = fmt.Sprintf("Current usage: %d / %d events\n", quotas.EventsUsed, quotas.EventsQuota)
	}

	// Add upgrade CTA for lower tiers
	switch tier {
	case string(entities.TierFree):
		body += `To increase your quota, upgrade to Pro ($29/mo, 1M events) or Growth ($79/mo billed yearly, 10M events).

Upgrade at: https://all-source.xyz/billing
`
	case string(entities.TierPro):
		body += `To increase your quota, upgrade to Growth ($79/mo billed yearly, 10M events).

Upgrade at: https://all-source.xyz/billing
`
	default:
		body += `Contact support if you need a higher quota: support@all-source.xyz
`
	}

	body += `
Best,
The AllSource Chronos Team
`
	return subject, body
}
