package usecases

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// StripeWebhookEvent represents a parsed Stripe webhook payload.
type StripeWebhookEvent struct {
	ID   string                 `json:"id"`
	Type string                 `json:"type"`
	Data StripeWebhookEventData `json:"data"`
}

// StripeWebhookEventData holds the data object from a Stripe webhook.
type StripeWebhookEventData struct {
	Object map[string]interface{} `json:"object"`
}

// ProcessStripeWebhookUseCase processes incoming Stripe webhook events.
type ProcessStripeWebhookUseCase struct {
	tenantRepo  repositories.TenantRepository
	auditRepo   repositories.AuditRepository
	updateSubUC *UpdateSubscriptionMetadataUseCase
	suspendUC   *SuspendTenantUseCase
}

// NewProcessStripeWebhookUseCase creates a new ProcessStripeWebhookUseCase.
func NewProcessStripeWebhookUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	updateSubUC *UpdateSubscriptionMetadataUseCase,
	suspendUC *SuspendTenantUseCase,
) *ProcessStripeWebhookUseCase {
	return &ProcessStripeWebhookUseCase{
		tenantRepo:  tenantRepo,
		auditRepo:   auditRepo,
		updateSubUC: updateSubUC,
		suspendUC:   suspendUC,
	}
}

// Execute processes a Stripe webhook event.
func (uc *ProcessStripeWebhookUseCase) Execute(ctx context.Context, event StripeWebhookEvent) error {
	var err error
	switch event.Type {
	case "customer.subscription.created":
		err = uc.handleSubscriptionCreated(event)
	case "customer.subscription.updated":
		err = uc.handleSubscriptionUpdated(event)
	case "customer.subscription.deleted":
		err = uc.handleSubscriptionDeleted(ctx, event)
	case "invoice.payment_failed":
		err = uc.handlePaymentFailed(event)
	default:
		// Unknown event — log and ignore
		uc.logAudit("webhook.unknown", "receive", "", event.Type)
		return nil
	}

	tenantID := extractTenantIDFromStripeWebhook(event)
	if err != nil {
		uc.logAudit("webhook.error", "process", tenantID, fmt.Sprintf("%s: %v", event.Type, err))
		return err
	}

	uc.logAudit("webhook."+event.Type, "process", tenantID, event.Type)
	return nil
}

func (uc *ProcessStripeWebhookUseCase) handleSubscriptionCreated(event StripeWebhookEvent) error {
	tenantID := extractTenantIDFromStripeWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in subscription metadata")
	}

	obj := event.Data.Object
	tier := resolveStripeSubscriptionTier(obj)

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  strVal(obj, "id"),
			CustomerID:      strVal(obj, "customer"),
			Status:          strVal(obj, "status"),
			Tier:            tier,
			PaymentProvider: "stripe",
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessStripeWebhookUseCase) handleSubscriptionUpdated(event StripeWebhookEvent) error {
	tenantID := extractTenantIDFromStripeWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in subscription metadata")
	}

	obj := event.Data.Object
	tier := resolveStripeSubscriptionTier(obj)
	status := strVal(obj, "status")

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  strVal(obj, "id"),
			CustomerID:      strVal(obj, "customer"),
			Status:          status,
			Tier:            tier,
			PaymentProvider: "stripe",
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessStripeWebhookUseCase) handleSubscriptionDeleted(ctx context.Context, event StripeWebhookEvent) error {
	tenantID := extractTenantIDFromStripeWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in subscription metadata")
	}

	obj := event.Data.Object
	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  strVal(obj, "id"),
			CustomerID:      strVal(obj, "customer"),
			Status:          "canceled",
			Tier:            resolveStripeSubscriptionTier(obj),
			PaymentProvider: "stripe",
		},
	}

	if _, err := uc.updateSubUC.Execute(tenantID, billing); err != nil {
		return fmt.Errorf("update subscription metadata: %w", err)
	}

	_, err := uc.suspendUC.Execute(ctx, tenantID, entities.RoleAdmin)
	if err != nil {
		return fmt.Errorf("suspend tenant: %w", err)
	}
	return nil
}

func (uc *ProcessStripeWebhookUseCase) handlePaymentFailed(event StripeWebhookEvent) error {
	// invoice.payment_failed — extract subscription from invoice object
	obj := event.Data.Object
	subID := strVal(obj, "subscription")
	customerID := strVal(obj, "customer")

	// Extract tenant_id from invoice metadata or subscription metadata
	tenantID := extractTenantIDFromStripeWebhook(event)
	if tenantID == "" {
		return fmt.Errorf("webhook missing tenant_id in invoice metadata")
	}

	billing := &entities.TenantBillingMetadata{
		Subscription: &entities.SubscriptionMetadata{
			SubscriptionID:  subID,
			CustomerID:      customerID,
			Status:          "past_due",
			PaymentProvider: "stripe",
		},
	}

	_, err := uc.updateSubUC.Execute(tenantID, billing)
	return err
}

func (uc *ProcessStripeWebhookUseCase) logAudit(eventType, action, tenantID, detail string) {
	auditEvent, _ := entities.NewAuditEvent(eventType, action, "POST", "/webhooks/stripe") //nolint:errcheck
	auditEvent.WithResource("webhook", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("detail", detail)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
}

// extractTenantIDFromStripeWebhook extracts tenant_id from subscription/invoice metadata.
func extractTenantIDFromStripeWebhook(event StripeWebhookEvent) string {
	obj := event.Data.Object
	// Try metadata on the object directly (subscription events)
	if metadata, ok := obj["metadata"].(map[string]interface{}); ok {
		if tid, ok := metadata["tenant_id"].(string); ok && tid != "" {
			return tid
		}
	}
	// Try subscription_details.metadata (invoice events)
	if subDetails, ok := obj["subscription_details"].(map[string]interface{}); ok {
		if metadata, ok := subDetails["metadata"].(map[string]interface{}); ok {
			if tid, ok := metadata["tenant_id"].(string); ok && tid != "" {
				return tid
			}
		}
	}
	return ""
}

// resolveStripeSubscriptionTier extracts tier from subscription metadata.
// Falls back to "free" if not set.
func resolveStripeSubscriptionTier(obj map[string]interface{}) string {
	if metadata, ok := obj["metadata"].(map[string]interface{}); ok {
		if tier, ok := metadata["tier"].(string); ok && tier != "" {
			return tier
		}
	}
	return defaultPlan
}

// strVal safely extracts a string value from a map.
func strVal(m map[string]interface{}, key string) string {
	if v, ok := m[key].(string); ok {
		return v
	}
	return ""
}
