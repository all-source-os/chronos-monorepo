package usecases

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
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

// ProcessLemonSqueezyWebhookUseCase processes incoming LemonSqueezy webhook events.
type ProcessLemonSqueezyWebhookUseCase struct {
	tenantRepo  repositories.TenantRepository
	auditRepo   repositories.AuditRepository
	updateSubUC *UpdateSubscriptionMetadataUseCase
	suspendUC   *SuspendTenantUseCase
}

// NewProcessLemonSqueezyWebhookUseCase creates a new ProcessLemonSqueezyWebhookUseCase.
func NewProcessLemonSqueezyWebhookUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	updateSubUC *UpdateSubscriptionMetadataUseCase,
	suspendUC *SuspendTenantUseCase,
) *ProcessLemonSqueezyWebhookUseCase {
	return &ProcessLemonSqueezyWebhookUseCase{
		tenantRepo:  tenantRepo,
		auditRepo:   auditRepo,
		updateSubUC: updateSubUC,
		suspendUC:   suspendUC,
	}
}

// Execute processes a LemonSqueezy webhook event.
func (uc *ProcessLemonSqueezyWebhookUseCase) Execute(ctx context.Context, event LemonSqueezyWebhookEvent) error {
	tenantID := extractTenantIDFromWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in custom_data")
	}

	var err error
	switch event.EventName {
	case "subscription_created":
		err = uc.handleSubscriptionCreated(tenantID, event)
	case "subscription_updated":
		err = uc.handleSubscriptionUpdated(tenantID, event)
	case "subscription_cancelled":
		err = uc.handleSubscriptionCancelled(ctx, tenantID, event)
	case "subscription_expired":
		err = uc.handleSubscriptionExpired(ctx, tenantID, event)
	case "subscription_payment_failed":
		err = uc.handlePaymentFailed(tenantID, event)
	default:
		// Unknown event — log and ignore
		uc.logAudit("webhook.unknown", "receive", tenantID, event.EventName)
		return nil
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
	tier := resolveTierFromVariantName(attrs.VariantName)

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID: event.Data.ID,
			CustomerID:     fmt.Sprintf("%d", attrs.CustomerID),
			Status:         attrs.Status,
			Tier:           tier,
		},
		// Quotas will be auto-applied by UpdateSubscriptionMetadataUseCase based on tier
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionUpdated(tenantID string, event LemonSqueezyWebhookEvent) error {
	attrs := event.Data.Attributes
	tier := resolveTierFromVariantName(attrs.VariantName)

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID: event.Data.ID,
			CustomerID:     fmt.Sprintf("%d", attrs.CustomerID),
			Status:         attrs.Status,
			Tier:           tier,
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessLemonSqueezyWebhookUseCase) handleSubscriptionCancelled(ctx context.Context, tenantID string, event LemonSqueezyWebhookEvent) error {
	// Update subscription status to cancelled
	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID: event.Data.ID,
			CustomerID:     fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:         "cancelled",
			Tier:           resolveTierFromVariantName(event.Data.Attributes.VariantName),
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
			SubscriptionID: event.Data.ID,
			CustomerID:     fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:         "expired",
			Tier:           resolveTierFromVariantName(event.Data.Attributes.VariantName),
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
			SubscriptionID: event.Data.ID,
			CustomerID:     fmt.Sprintf("%d", event.Data.Attributes.CustomerID),
			Status:         "past_due",
			Tier:           resolveTierFromVariantName(event.Data.Attributes.VariantName),
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
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

// resolveTierFromVariantName maps a LemonSqueezy variant name to a tier.
// Falls back to "free" for unknown variants.
func resolveTierFromVariantName(variantName string) string {
	switch variantName {
	case "Starter", "starter":
		return "starter"
	case "Pro", "pro":
		return "pro"
	case "Enterprise", "enterprise":
		return "enterprise"
	default:
		return "free"
	}
}
