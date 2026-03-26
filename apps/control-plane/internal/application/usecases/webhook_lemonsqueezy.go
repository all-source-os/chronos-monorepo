package usecases

import (
	"context"
	"fmt"
	"log"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// LemonSqueezyWebhookEvent represents a parsed LemonSqueezy webhook payload.
type LemonSqueezyWebhookEvent struct {
	EventName string                 `json:"event_name"`
	Data      LemonSqueezyEventData  `json:"data"`
	Meta      map[string]interface{} `json:"meta"`
}

// LemonSqueezyEventData holds the data object from a LemonSqueezy webhook.
type LemonSqueezyEventData struct {
	ID         string                        `json:"id"`
	Type       string                        `json:"type"`
	Attributes LemonSqueezySubscriptionAttrs `json:"attributes"`
}

// LemonSqueezySubscriptionAttrs holds subscription attributes from LemonSqueezy.
type LemonSqueezySubscriptionAttrs struct {
	StoreID         int    `json:"store_id"`
	CustomerID      int    `json:"customer_id"`
	ProductID       int    `json:"product_id"`
	VariantID       int    `json:"variant_id"`
	Status          string `json:"status"`
	StatusFormatted string `json:"status_formatted"`
	ProductName     string `json:"product_name"`
	VariantName     string `json:"variant_name"`
	UserName        string `json:"user_name"`
	UserEmail       string `json:"user_email"`
}

// Tier constants for subscription plans.
const (
	tierGrowth     = "growth"
	tierTeam       = "team"
	tierEnterprise = "enterprise"
)

// VariantTierMap maps LemonSqueezy variant names/IDs to subscription tier names.
// Built as a reverse lookup from the LEMON_SQUEEZY_VARIANT_MAP env var.
type VariantTierMap map[string]string

// ProcessLemonSqueezyWebhookUseCase processes incoming LemonSqueezy webhook events.
type ProcessLemonSqueezyWebhookUseCase struct {
	tenantRepo     repositories.TenantRepository
	auditRepo      repositories.AuditRepository
	updateSubUC    *UpdateSubscriptionMetadataUseCase
	suspendUC      *SuspendTenantUseCase
	variantTierMap VariantTierMap
	coreClient     clients.CoreClient
}

// NewProcessLemonSqueezyWebhookUseCase creates a new ProcessLemonSqueezyWebhookUseCase.
func NewProcessLemonSqueezyWebhookUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	updateSubUC *UpdateSubscriptionMetadataUseCase,
	suspendUC *SuspendTenantUseCase,
	variantTierMap VariantTierMap,
	coreClient ...clients.CoreClient,
) *ProcessLemonSqueezyWebhookUseCase {
	uc := &ProcessLemonSqueezyWebhookUseCase{
		tenantRepo:     tenantRepo,
		auditRepo:      auditRepo,
		updateSubUC:    updateSubUC,
		suspendUC:      suspendUC,
		variantTierMap: variantTierMap,
	}
	if len(coreClient) > 0 {
		uc.coreClient = coreClient[0]
	}
	return uc
}

// Execute processes a LemonSqueezy webhook event.
func (uc *ProcessLemonSqueezyWebhookUseCase) Execute(ctx context.Context, event LemonSqueezyWebhookEvent) error {
	tenantID := extractTenantIDFromWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in custom_data")
	}

	attrs := event.Data.Attributes
	tier := uc.resolveTier(attrs.VariantName, attrs.VariantID)

	var err error
	switch event.EventName {
	case "subscription_created":
		err = uc.handleSubscriptionCreated(tenantID, event)
	case "subscription_updated":
		err = uc.handleSubscriptionUpdated(tenantID, event)
	case "subscription_cancelled": //nolint:misspell // LemonSqueezy API event name
		err = uc.handleSubscriptionCanceled(ctx, tenantID, event)
	case "subscription_expired":
		err = uc.handleSubscriptionExpired(ctx, tenantID, event)
	case "subscription_payment_failed":
		err = uc.handlePaymentFailed(tenantID, event)
	default:
		// Unknown event — log and ignore
		uc.logAudit("webhook.unknown", "receive", tenantID, event.EventName)
		return nil
	}

	// Write billing event to Core (non-blocking — errors logged but don't fail the webhook)
	if err == nil {
		billingEventType := "billing." + event.EventName
		uc.writeBillingEvent(ctx, billingEventType, tenantID, attrs, tier, attrs.Status)
	}

	if err != nil {
		uc.logAudit("webhook.error", "process", tenantID, fmt.Sprintf("%s: %v", event.EventName, err))
		return err
	}

	uc.logAudit("webhook."+event.EventName, "process", tenantID, event.EventName)
	return nil
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionCreated(tenantID string, event LemonSqueezyWebhookEvent) error {
	attrs := event.Data.Attributes
	tier := uc.resolveTier(attrs.VariantName, attrs.VariantID)

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  event.Data.ID,
			CustomerID:      fmt.Sprintf("%d", attrs.CustomerID),
			Status:          attrs.Status,
			Tier:            tier,
			PaymentProvider: "lemonsqueezy",
		},
		// Quotas will be auto-applied by UpdateSubscriptionMetadataUseCase based on tier
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionUpdated(tenantID string, event LemonSqueezyWebhookEvent) error {
	attrs := event.Data.Attributes
	tier := uc.resolveTier(attrs.VariantName, attrs.VariantID)

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  event.Data.ID,
			CustomerID:      fmt.Sprintf("%d", attrs.CustomerID),
			Status:          attrs.Status,
			Tier:            tier,
			PaymentProvider: "lemonsqueezy",
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionCanceled(ctx context.Context, tenantID string, event LemonSqueezyWebhookEvent) error {
	// Update subscription status to canceled
	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  event.Data.ID,
			CustomerID:      fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:          "canceled",
			Tier:            uc.resolveTier(event.Data.Attributes.VariantName, event.Data.Attributes.VariantID),
			PaymentProvider: "lemonsqueezy",
		},
	}
	if _, err := uc.updateSubUC.Execute(tenantID, billing); err != nil {
		return fmt.Errorf("update subscription metadata: %w", err)
	}

	// Suspend the tenant
	_, err := uc.suspendUC.Execute(ctx, tenantID, entities.RoleAdmin)
	if err != nil {
		return fmt.Errorf("suspend tenant: %w", err)
	}
	return nil
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionExpired(ctx context.Context, tenantID string, event LemonSqueezyWebhookEvent) error {
	// Update subscription status to expired
	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  event.Data.ID,
			CustomerID:      fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:          "expired",
			Tier:            uc.resolveTier(event.Data.Attributes.VariantName, event.Data.Attributes.VariantID),
			PaymentProvider: "lemonsqueezy",
		},
	}
	if _, err := uc.updateSubUC.Execute(tenantID, billing); err != nil {
		return fmt.Errorf("update subscription metadata: %w", err)
	}

	// Suspend the tenant
	_, err := uc.suspendUC.Execute(ctx, tenantID, entities.RoleAdmin)
	if err != nil {
		return fmt.Errorf("suspend tenant: %w", err)
	}
	return nil
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handlePaymentFailed(tenantID string, event LemonSqueezyWebhookEvent) error {
	// Update subscription status to past_due but don't suspend yet
	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  event.Data.ID,
			CustomerID:      fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:          "past_due",
			Tier:            uc.resolveTier(event.Data.Attributes.VariantName, event.Data.Attributes.VariantID),
			PaymentProvider: "lemonsqueezy",
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

// writeBillingEvent writes a billing event to Core for durability and audit trail.
// Errors are logged but don't block webhook processing.
func (uc *ProcessLemonSqueezyWebhookUseCase) writeBillingEvent(ctx context.Context, eventType, tenantID string, attrs LemonSqueezySubscriptionAttrs, tier, status string) {
	if uc.coreClient == nil {
		return
	}

	payload := map[string]any{
		"subscription_id":  "",
		"customer_id":      fmt.Sprintf("%d", attrs.CustomerID),
		"tier":             tier,
		"status":           status,
		"payment_provider": "lemonsqueezy",
		"variant_id":       attrs.VariantID,
		"variant_name":     attrs.VariantName,
		"product_name":     attrs.ProductName,
	}

	_, err := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: eventType,
		EntityID:  tenantID,
		Payload:   payload,
	})
	if err != nil {
		log.Printf("[billing] failed to write %s event for tenant %s to Core: %v", eventType, tenantID, err)
	}
}

func (uc *ProcessLemonSqueezyWebhookUseCase) logAudit(eventType, action, tenantID, detail string) {
	auditEvent, _ := entities.NewAuditEvent(eventType, action, "POST", "/webhooks/lemonsqueezy") //nolint:errcheck
	auditEvent.WithResource("webhook", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("detail", detail)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
}

// extractTenantIDFromWebhook pulls the tenant_id from the webhook's meta.custom_data.
func extractTenantIDFromWebhook(event LemonSqueezyWebhookEvent) string {
	meta := event.Meta
	if meta == nil {
		return ""
	}
	customData, ok := meta["custom_data"]
	if !ok {
		return ""
	}
	switch cd := customData.(type) {
	case map[string]interface{}:
		if tid, ok := cd["tenant_id"].(string); ok {
			return tid
		}
	case map[string]string:
		return cd["tenant_id"]
	}
	return ""
}

// resolveTier maps a LemonSqueezy variant name or ID to a tier using the variant map.
// Falls back to hardcoded mapping if variant map is not configured, and "free" as last resort.
func (uc *ProcessLemonSqueezyWebhookUseCase) resolveTier(variantName string, variantID int) string {
	// Try variant map first (reverse lookup: check both variant name and ID)
	if uc.variantTierMap != nil {
		if tier, ok := uc.variantTierMap[variantName]; ok {
			return tier
		}
		variantIDStr := fmt.Sprintf("%d", variantID)
		if tier, ok := uc.variantTierMap[variantIDStr]; ok {
			return tier
		}
	}

	// Hardcoded fallback for backwards compatibility
	switch variantName {
	case "Pro", "pro", "Growth", tierGrowth:
		return tierGrowth
	case "Team", tierTeam:
		return tierTeam
	case "Enterprise", tierEnterprise:
		return tierEnterprise
	default:
		return defaultPlan
	}
}
