package usecases

import (
	"context"
	"fmt"
	"math"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Payment provider constants.
const (
	providerLemonSqueezy = "lemonsqueezy"
	providerStripe       = "stripe"
)

// --- Billing DTOs (handler-facing) ---

// CheckoutRequest is the input for creating a billing checkout.
type CheckoutRequest struct {
	TenantID      string `json:"tenant_id" binding:"required"`
	Tier          string `json:"tier" binding:"required"`
	BillingPeriod string `json:"billing_period,omitempty"` // "monthly" (default) or "annual"/"yearly"
	Provider      string `json:"provider,omitempty"`       // "lemonsqueezy" (default) or "stripe"
	Email         string `json:"email,omitempty"`
	Name          string `json:"name,omitempty"`
	RedirectURL   string `json:"redirect_url,omitempty"`
}

// CheckoutResult is the output of creating a billing checkout.
type CheckoutResult struct {
	CheckoutID  string `json:"checkout_id"`
	CheckoutURL string `json:"checkout_url"`
	TenantID    string `json:"tenant_id"`
	Tier        string `json:"tier"`
	Provider    string `json:"provider"`
}

// PortalResult is the output of getting the customer portal URL.
type PortalResult struct {
	PortalURL string `json:"portal_url"`
	TenantID  string `json:"tenant_id"`
}

// OverageSummary is the output of getting the overage summary.
type OverageSummary struct {
	TenantID       string  `json:"tenant_id"`
	Enabled        bool    `json:"enabled"`
	EventRate      float64 `json:"event_rate,omitempty"`
	QueryRate      float64 `json:"query_rate,omitempty"`
	EventsOverage  int64   `json:"events_overage"`
	QueriesOverage int64   `json:"queries_overage"`
	EventsQuota    int64   `json:"events_quota"`
	QueriesQuota   int64   `json:"queries_quota"`
	EventsUsed     int64   `json:"events_used"`
	QueriesUsed    int64   `json:"queries_used"`
}

// ProjectedCharges is the output of projected billing.
type ProjectedCharges struct {
	TenantID                string `json:"tenant_id"`
	Tier                    string `json:"tier"`
	EventsQuota             int64  `json:"events_quota"`
	QueriesQuota            int64  `json:"queries_quota"`
	EventsUsed              int64  `json:"events_used"`
	QueriesUsed             int64  `json:"queries_used"`
	ProjectedEvents         int64  `json:"projected_events"`
	ProjectedQueries        int64  `json:"projected_queries"`
	ProjectedOverageEvents  int64  `json:"projected_overage_events"`
	ProjectedOverageQueries int64  `json:"projected_overage_queries"`
	EstimatedChargeCents    int64  `json:"estimated_charge_cents"`
	OverageEnabled          bool   `json:"overage_enabled"`
}

// --- Use Cases ---

// CreateCheckoutUseCase creates a checkout for a tenant using the requested payment provider.
type CreateCheckoutUseCase struct {
	tenantRepo   repositories.TenantRepository
	lsClient     clients.LemonSqueezyClient
	stripeClient clients.StripeClient
	auditRepo    repositories.AuditRepository
}

// NewCreateCheckoutUseCase creates a new CreateCheckoutUseCase.
func NewCreateCheckoutUseCase(
	tenantRepo repositories.TenantRepository,
	lsClient clients.LemonSqueezyClient,
	stripeClient clients.StripeClient,
	auditRepo repositories.AuditRepository,
) *CreateCheckoutUseCase {
	return &CreateCheckoutUseCase{
		tenantRepo:   tenantRepo,
		lsClient:     lsClient,
		stripeClient: stripeClient,
		auditRepo:    auditRepo,
	}
}

// Execute creates a checkout URL for the given tenant and tier.
func (uc *CreateCheckoutUseCase) Execute(ctx context.Context, req CheckoutRequest) (*CheckoutResult, error) {
	// Verify tenant exists
	_, err := uc.tenantRepo.FindByID(req.TenantID)
	if err != nil {
		return nil, err
	}

	provider := req.Provider
	if provider == "" {
		provider = providerLemonSqueezy
	}

	switch provider {
	case "stripe":
		return uc.executeStripe(ctx, req)
	case providerLemonSqueezy:
		return uc.executeLemonSqueezy(ctx, req)
	default:
		return nil, fmt.Errorf("unsupported payment provider: %s", provider)
	}
}

func (uc *CreateCheckoutUseCase) executeLemonSqueezy(ctx context.Context, req CheckoutRequest) (*CheckoutResult, error) {
	if uc.lsClient == nil {
		return nil, fmt.Errorf("lemonsqueezy payment provider is not configured")
	}

	variantID, err := uc.lsClient.LookupVariantID(req.Tier)
	if err != nil {
		return nil, fmt.Errorf("unknown billing tier %q: %w", req.Tier, err)
	}

	checkoutReq := clients.CreateCheckoutRequest{
		VariantID: variantID,
		Email:     req.Email,
		Name:      req.Name,
		CustomData: map[string]string{
			"tenant_id": req.TenantID,
		},
		RedirectURL: req.RedirectURL,
	}

	checkout, err := uc.lsClient.CreateCheckout(ctx, checkoutReq)
	if err != nil {
		return nil, fmt.Errorf("create checkout: %w", err)
	}

	uc.logCheckoutAudit(req.TenantID, providerLemonSqueezy)

	return &CheckoutResult{
		CheckoutID:  checkout.ID,
		CheckoutURL: checkout.URL,
		TenantID:    req.TenantID,
		Tier:        req.Tier,
		Provider:    providerLemonSqueezy,
	}, nil
}

func (uc *CreateCheckoutUseCase) executeStripe(ctx context.Context, req CheckoutRequest) (*CheckoutResult, error) {
	if uc.stripeClient == nil {
		return nil, fmt.Errorf("stripe payment provider is not configured")
	}

	// Resolve tier + billing period to the right Stripe price (monthly vs annual).
	// Canonicalize retired tier ids so an old "pro"/"growth" request still maps
	// to a live 011 price during cutover.
	canonicalTier := entities.MapRetiredTier(req.Tier)
	priceID, err := uc.stripeClient.LookupPriceID(canonicalTier, req.BillingPeriod)
	if err != nil {
		return nil, fmt.Errorf("resolve price for tier %q period %q: %w", req.Tier, req.BillingPeriod, err)
	}

	period := req.BillingPeriod
	if period == "" {
		period = "monthly"
	}

	successURL := req.RedirectURL
	if successURL == "" {
		successURL = "https://all-source.xyz/billing/success"
	}

	checkoutReq := clients.StripeCheckoutRequest{
		PriceID:       priceID,
		CustomerEmail: req.Email,
		Metadata: map[string]string{
			"tenant_id":      req.TenantID,
			"tier":           canonicalTier,
			"billing_period": period,
		},
		SuccessURL: successURL,
		CancelURL:  successURL,
	}

	session, err := uc.stripeClient.CreateCheckoutSession(ctx, checkoutReq)
	if err != nil {
		return nil, fmt.Errorf("create stripe checkout: %w", err)
	}

	uc.logCheckoutAudit(req.TenantID, "stripe")

	return &CheckoutResult{
		CheckoutID:  session.ID,
		CheckoutURL: session.URL,
		TenantID:    req.TenantID,
		Tier:        req.Tier,
		Provider:    "stripe",
	}, nil
}

func (uc *CreateCheckoutUseCase) logCheckoutAudit(tenantID, provider string) {
	auditEvent, _ := entities.NewAuditEvent("billing.checkout.created", "create", "POST", "/billing/checkout") //nolint:errcheck
	auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("provider", provider)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck
}

// GetPortalUseCase retrieves the customer portal URL for a tenant.
type GetPortalUseCase struct {
	tenantRepo   repositories.TenantRepository
	lsClient     clients.LemonSqueezyClient
	stripeClient clients.StripeClient
}

// NewGetPortalUseCase creates a new GetPortalUseCase.
func NewGetPortalUseCase(
	tenantRepo repositories.TenantRepository,
	lsClient clients.LemonSqueezyClient,
	stripeClient clients.StripeClient,
) *GetPortalUseCase {
	return &GetPortalUseCase{
		tenantRepo:   tenantRepo,
		lsClient:     lsClient,
		stripeClient: stripeClient,
	}
}

// Execute retrieves the customer portal URL for the given tenant.
func (uc *GetPortalUseCase) Execute(ctx context.Context, tenantID string) (*PortalResult, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	sub := extractSubscription(tenant.Metadata)
	if sub.SubscriptionID == "" && sub.CustomerID == "" {
		return nil, fmt.Errorf("tenant %s has no active subscription", tenantID)
	}

	var portalURL string
	switch sub.PaymentProvider {
	case "stripe":
		if uc.stripeClient == nil {
			return nil, fmt.Errorf("stripe provider not configured")
		}
		portalURL, err = uc.stripeClient.GetCustomerPortalURL(ctx, sub.CustomerID, "")
	default:
		// LemonSqueezy (or unset provider for backwards compat)
		if uc.lsClient == nil {
			return nil, fmt.Errorf("lemonsqueezy provider not configured")
		}
		portalURL, err = uc.lsClient.GetCustomerPortalURL(ctx, sub.SubscriptionID)
	}
	if err != nil {
		return nil, fmt.Errorf("get portal URL: %w", err)
	}

	return &PortalResult{
		PortalURL: portalURL,
		TenantID:  tenantID,
	}, nil
}

// GetOverageSummaryUseCase retrieves overage data from tenant metadata.
type GetOverageSummaryUseCase struct {
	tenantRepo repositories.TenantRepository
}

// NewGetOverageSummaryUseCase creates a new GetOverageSummaryUseCase.
func NewGetOverageSummaryUseCase(tenantRepo repositories.TenantRepository) *GetOverageSummaryUseCase {
	return &GetOverageSummaryUseCase{tenantRepo: tenantRepo}
}

// Execute retrieves the overage summary for the given tenant.
func (uc *GetOverageSummaryUseCase) Execute(tenantID string) (*OverageSummary, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	overage := extractOverage(tenant.Metadata)
	quotas := extractQuotas(tenant.Metadata)

	return &OverageSummary{
		TenantID:       tenantID,
		Enabled:        overage.Enabled,
		EventRate:      overage.EventRate,
		QueryRate:      overage.QueryRate,
		EventsOverage:  overage.EventsOverage,
		QueriesOverage: overage.QueriesOverage,
		EventsQuota:    quotas.EventsQuota,
		QueriesQuota:   quotas.QueriesQuota,
		EventsUsed:     quotas.EventsUsed,
		QueriesUsed:    quotas.QueriesUsed,
	}, nil
}

// SetOverageEnabledUseCase enables or disables hybrid pricing for a tenant.
type SetOverageEnabledUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
}

// NewSetOverageEnabledUseCase creates a new SetOverageEnabledUseCase.
func NewSetOverageEnabledUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
) *SetOverageEnabledUseCase {
	return &SetOverageEnabledUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
	}
}

// Execute enables or disables overage for the given tenant.
func (uc *SetOverageEnabledUseCase) Execute(tenantID string, enabled bool) (*OverageSummary, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	if tenant.Metadata == nil {
		tenant.Metadata = make(map[string]interface{})
	}

	overage := extractOverage(tenant.Metadata)
	overage.Enabled = enabled
	tenant.Metadata["overage"] = &overage

	if err := uc.tenantRepo.Update(tenant); err != nil {
		return nil, err
	}

	action := "disabled"
	if enabled {
		action = "enabled"
	}
	auditEvent, _ := entities.NewAuditEvent("billing.overage."+action, "update", "POST", "/billing/overage") //nolint:errcheck
	auditEvent.WithResource("tenant", tenantID).WithTenant(tenantID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	quotas := extractQuotas(tenant.Metadata)

	return &OverageSummary{
		TenantID:       tenantID,
		Enabled:        overage.Enabled,
		EventRate:      overage.EventRate,
		QueryRate:      overage.QueryRate,
		EventsOverage:  overage.EventsOverage,
		QueriesOverage: overage.QueriesOverage,
		EventsQuota:    quotas.EventsQuota,
		QueriesQuota:   quotas.QueriesQuota,
		EventsUsed:     quotas.EventsUsed,
		QueriesUsed:    quotas.QueriesUsed,
	}, nil
}

// GetProjectedChargesUseCase calculates projected charges based on current usage rate.
type GetProjectedChargesUseCase struct {
	tenantRepo repositories.TenantRepository
}

// NewGetProjectedChargesUseCase creates a new GetProjectedChargesUseCase.
func NewGetProjectedChargesUseCase(tenantRepo repositories.TenantRepository) *GetProjectedChargesUseCase {
	return &GetProjectedChargesUseCase{tenantRepo: tenantRepo}
}

// Execute projects end-of-period charges for the given tenant.
func (uc *GetProjectedChargesUseCase) Execute(tenantID string) (*ProjectedCharges, error) {
	tenant, err := uc.tenantRepo.FindByID(tenantID)
	if err != nil {
		return nil, err
	}

	sub := extractSubscription(tenant.Metadata)
	quotas := extractQuotas(tenant.Metadata)
	overage := extractOverage(tenant.Metadata)

	// Linear projection: assume usage doubles by end of period
	projectedEvents := quotas.EventsUsed * 2
	projectedQueries := quotas.QueriesUsed * 2

	var projectedOverageEvents, projectedOverageQueries int64
	if projectedEvents > quotas.EventsQuota && quotas.EventsQuota > 0 {
		projectedOverageEvents = projectedEvents - quotas.EventsQuota
	}
	if projectedQueries > quotas.QueriesQuota && quotas.QueriesQuota > 0 {
		projectedOverageQueries = projectedQueries - quotas.QueriesQuota
	}

	var estimatedChargeCents int64
	if overage.Enabled {
		eventCharge := float64(projectedOverageEvents) * overage.EventRate
		queryCharge := float64(projectedOverageQueries) * overage.QueryRate
		estimatedChargeCents = int64(math.Ceil(eventCharge + queryCharge))
	}

	return &ProjectedCharges{
		TenantID:                tenantID,
		Tier:                    sub.Tier,
		EventsQuota:             quotas.EventsQuota,
		QueriesQuota:            quotas.QueriesQuota,
		EventsUsed:              quotas.EventsUsed,
		QueriesUsed:             quotas.QueriesUsed,
		ProjectedEvents:         projectedEvents,
		ProjectedQueries:        projectedQueries,
		ProjectedOverageEvents:  projectedOverageEvents,
		ProjectedOverageQueries: projectedOverageQueries,
		EstimatedChargeCents:    estimatedChargeCents,
		OverageEnabled:          overage.Enabled,
	}, nil
}

// --- Metadata extraction helpers ---

func extractSubscription(metadata map[string]interface{}) entities.SubscriptionMetadata {
	if metadata == nil {
		return entities.SubscriptionMetadata{}
	}
	raw, ok := metadata["subscription"]
	if !ok {
		return entities.SubscriptionMetadata{}
	}
	switch v := raw.(type) {
	case *entities.SubscriptionMetadata:
		return *v
	case entities.SubscriptionMetadata:
		return v
	case map[string]interface{}:
		return subscriptionFromMap(v)
	default:
		return entities.SubscriptionMetadata{}
	}
}

func extractQuotas(metadata map[string]interface{}) entities.QuotaMetadata {
	if metadata == nil {
		return entities.QuotaMetadata{}
	}
	raw, ok := metadata["quotas"]
	if !ok {
		return entities.QuotaMetadata{}
	}
	switch v := raw.(type) {
	case *entities.QuotaMetadata:
		return *v
	case entities.QuotaMetadata:
		return v
	case map[string]interface{}:
		return quotasFromMap(v)
	default:
		return entities.QuotaMetadata{}
	}
}

func extractOverage(metadata map[string]interface{}) entities.OverageMetadata {
	if metadata == nil {
		return entities.OverageMetadata{}
	}
	raw, ok := metadata["overage"]
	if !ok {
		return entities.OverageMetadata{}
	}
	switch v := raw.(type) {
	case *entities.OverageMetadata:
		return *v
	case entities.OverageMetadata:
		return v
	case map[string]interface{}:
		return overageFromMap(v)
	default:
		return entities.OverageMetadata{}
	}
}

func subscriptionFromMap(m map[string]interface{}) entities.SubscriptionMetadata {
	s := entities.SubscriptionMetadata{}
	if v, ok := m["subscription_id"].(string); ok {
		s.SubscriptionID = v
	}
	if v, ok := m["subscription_item_id"].(string); ok {
		s.SubscriptionItemID = v
	}
	if v, ok := m["customer_id"].(string); ok {
		s.CustomerID = v
	}
	if v, ok := m["status"].(string); ok {
		s.Status = v
	}
	if v, ok := m["tier"].(string); ok {
		s.Tier = v
	}
	if v, ok := m["payment_provider"].(string); ok {
		s.PaymentProvider = v
	}
	return s
}

func quotasFromMap(m map[string]interface{}) entities.QuotaMetadata {
	q := entities.QuotaMetadata{}
	if v, ok := m["events_quota"].(float64); ok {
		q.EventsQuota = int64(v)
	}
	if v, ok := m["queries_quota"].(float64); ok {
		q.QueriesQuota = int64(v)
	}
	if v, ok := m["events_used"].(float64); ok {
		q.EventsUsed = int64(v)
	}
	if v, ok := m["queries_used"].(float64); ok {
		q.QueriesUsed = int64(v)
	}
	if v, ok := m["x402_allowance"].(float64); ok {
		q.X402Allowance = int64(v)
	}
	if v, ok := m["x402_used"].(float64); ok {
		q.X402Used = int64(v)
	}
	if v, ok := m["retention_days"].(float64); ok {
		q.RetentionDays = int64(v)
	}
	if v, ok := m["max_streams"].(float64); ok {
		q.MaxStreams = int64(v)
	}
	if v, ok := m["mcp_scope"].(string); ok {
		q.MCPScope = v
	}
	return q
}

func overageFromMap(m map[string]interface{}) entities.OverageMetadata {
	o := entities.OverageMetadata{}
	if v, ok := m["enabled"].(bool); ok {
		o.Enabled = v
	}
	if v, ok := m["event_rate"].(float64); ok {
		o.EventRate = v
	}
	if v, ok := m["query_rate"].(float64); ok {
		o.QueryRate = v
	}
	if v, ok := m["events_overage"].(float64); ok {
		o.EventsOverage = int64(v)
	}
	if v, ok := m["queries_overage"].(float64); ok {
		o.QueriesOverage = int64(v)
	}
	if v, ok := m["last_reported_events"].(float64); ok {
		o.LastReportedEvents = int64(v)
	}
	if v, ok := m["last_reported_queries"].(float64); ok {
		o.LastReportedQueries = int64(v)
	}
	return o
}
