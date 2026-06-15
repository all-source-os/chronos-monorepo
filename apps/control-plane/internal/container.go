// Package internal contains the dependency injection container and internal wiring.
package internal

import (
	"context"
	"encoding/json"
	"os"
	"strings"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/application/usecases/billing"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider/nylas"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
	httphandlers "github.com/allsource/control-plane/internal/interfaces/http"
)

// buildVariantTierMap reads LEMON_SQUEEZY_VARIANT_MAP and builds a reverse map
// (variantID → tier name). This is the SOLE authority for webhook tier
// resolution (resolveTier) and reconciliation — keyed by variant ID only.
//
// The forward map is keyed "<tier>:<period>" (011) — e.g.
// {"studio:monthly":"12345","studio:annual":"67890"}. The webhook only cares
// about the tier, so the ":<period>" suffix is stripped when reversing so a
// variant ID resolves to the bare tier ("studio"), not "studio:monthly" (which
// is not a valid tier and would fall through to the free quota).
func buildVariantTierMap() usecases.VariantTierMap {
	raw := os.Getenv("LEMON_SQUEEZY_VARIANT_MAP")
	if raw == "" {
		return nil
	}

	var forwardMap map[string]string
	if err := json.Unmarshal([]byte(raw), &forwardMap); err != nil {
		return nil
	}

	reverseMap := make(usecases.VariantTierMap)
	for key, variantID := range forwardMap {
		tier := key
		if i := strings.IndexByte(key, ':'); i >= 0 {
			tier = key[:i] // "studio:monthly" → "studio"
		}
		reverseMap[variantID] = tier // variant ID → bare tier (only authority)
	}
	return reverseMap
}

// Container holds all application dependencies
type Container struct {
	// Repositories
	TenantRepo    repositories.TenantRepository
	PolicyRepo    repositories.PolicyRepository
	AuditRepo     repositories.AuditRepository
	OperationRepo repositories.OperationRepository
	ConfigRepo    repositories.ConfigRepository
	AlertRuleRepo repositories.AlertRuleRepository
	SLORepo       repositories.SLORepository
	IPRuleRepo    repositories.IPRuleRepository

	// Use Cases — Tenants
	CreateTenantUC       *usecases.CreateTenantUseCase
	GetTenantUC          *usecases.GetTenantUseCase
	ListTenantsUC        *usecases.ListTenantsUseCase
	UpdateTenantUC       *usecases.UpdateTenantUseCase
	UpdateTenantQuotasUC *usecases.UpdateTenantQuotasUseCase
	SuspendTenantUC      *usecases.SuspendTenantUseCase
	ActivateTenantUC     *usecases.ActivateTenantUseCase
	BulkTenantUC         *usecases.BulkTenantUseCase
	DeleteTenantUC       *usecases.DeleteTenantUseCase

	// Use Cases — Policies
	EvaluatePolicyUC *usecases.EvaluatePolicyUseCase
	CreatePolicyUC   *usecases.CreatePolicyUseCase
	GetPolicyUC      *usecases.GetPolicyUseCase
	ListPoliciesUC   *usecases.ListPoliciesUseCase
	UpdatePolicyUC   *usecases.UpdatePolicyUseCase
	DeletePolicyUC   *usecases.DeletePolicyUseCase

	// Use Cases — Operations
	CreateSnapshotUC    *usecases.CreateSnapshotUseCase
	TriggerCompactionUC *usecases.TriggerCompactionUseCase
	StartReplayUC       *usecases.StartReplayUseCase
	GetReplayProgressUC *usecases.GetReplayProgressUseCase
	CancelReplayUC      *usecases.CancelReplayUseCase
	ListOperationsUC    *usecases.ListOperationsUseCase
	GetClusterStatusUC  *usecases.GetClusterStatusUseCase

	// Use Cases — Schema Governance
	RegisterSchemaUC *usecases.RegisterSchemaUseCase
	ListSchemasUC    *usecases.ListSchemasUseCase
	ValidateEventUC  *usecases.ValidateEventUseCase

	// Use Cases — Audit Trail
	QueryAuditUC               *usecases.QueryAuditUseCase
	QueryTokenAuditUC          *usecases.QueryTokenAuditUseCase
	DetectSuspiciousActivityUC *usecases.DetectSuspiciousActivityUseCase

	// Use Cases — Config
	CreateConfigUC *usecases.CreateConfigUseCase
	GetConfigUC    *usecases.GetConfigUseCase
	ListConfigsUC  *usecases.ListConfigsUseCase
	UpdateConfigUC *usecases.UpdateConfigUseCase
	DeleteConfigUC *usecases.DeleteConfigUseCase

	// Use Cases — Alert Rules
	CreateAlertRuleUC *usecases.CreateAlertRuleUseCase
	ListAlertRulesUC  *usecases.ListAlertRulesUseCase
	UpdateAlertRuleUC *usecases.UpdateAlertRuleUseCase
	DeleteAlertRuleUC *usecases.DeleteAlertRuleUseCase

	// Use Cases — IP Rules
	CreateIPRuleUC *usecases.CreateIPRuleUseCase
	ListIPRulesUC  *usecases.ListIPRulesUseCase
	DeleteIPRuleUC *usecases.DeleteIPRuleUseCase

	// Use Cases — SLOs
	CreateSLOUC *usecases.CreateSLOUseCase
	ListSLOsUC  *usecases.ListSLOsUseCase
	DeleteSLOUC *usecases.DeleteSLOUseCase

	// Use Cases — Billing
	CreateCheckoutUC      *usecases.CreateCheckoutUseCase
	GetPortalUC           *usecases.GetPortalUseCase
	GetOverageSummaryUC   *usecases.GetOverageSummaryUseCase
	SetOverageEnabledUC   *usecases.SetOverageEnabledUseCase
	GetProjectedChargesUC *usecases.GetProjectedChargesUseCase

	// Use Cases — Billing (overage)
	CalculateOverageUC   *billing.CalculateOverageUseCase
	ReportUsageUC        *billing.ReportUsageUseCase
	CheckUsageWarningsUC *billing.CheckUsageWarningsUseCase

	// Use Cases — Admin Billing
	AdminListInvoicesUC *billing.AdminListInvoicesUseCase
	AdminRevenueUC      *billing.AdminRevenueUseCase
	AdminRefundUC       *billing.AdminRefundUseCase
	AdminDunningUC      *billing.AdminDunningUseCase

	// Use Cases — Agent Registration
	RegisterAgentUC       *usecases.RegisterAgentUseCase
	RegisterTrialAgentUC  *usecases.RegisterTrialAgentUseCase
	ClaimTrialAgentUC     *usecases.ClaimTrialAgentUseCase
	AgentPaymentHistoryUC *usecases.GetAgentPaymentHistoryUseCase

	// Use Cases — Webhooks
	ProcessLSWebhookUC     *usecases.ProcessLemonSqueezyWebhookUseCase
	UpdateSubscriptionUC   *usecases.UpdateSubscriptionMetadataUseCase
	MigrateEarlyAdoptersUC *usecases.MigrateEarlyAdoptersUseCase

	// Scheduler
	Scheduler *usecases.OperationScheduler

	// x402 auto-pay support
	WalletLookup *CoreWalletLookup
	CDPClient    clients.CDPClient

	// HTTP Handlers
	AlertHandler                 *httphandlers.AlertHandler
	SLOHandler                   *httphandlers.SLOHandler
	AdminTenantHandler           *httphandlers.AdminTenantHandler
	TenantHandler                *httphandlers.TenantHandler
	PolicyHandler                *httphandlers.PolicyHandler
	OperationsHandler            *httphandlers.OperationsHandler
	SchemaHandler                *httphandlers.SchemaHandler
	AuditHandler                 *httphandlers.AuditHandler
	TokenAuditHandler            *httphandlers.TokenAuditHandler
	SuspiciousActivityHandler    *httphandlers.SuspiciousActivityHandler
	ConfigHandler                *httphandlers.ConfigHandler
	IPRuleHandler                *httphandlers.IPRuleHandler
	BillingHandler               *httphandlers.BillingHandler
	AdminBillingHandler          *httphandlers.AdminBillingHandler
	BillingConfigHandler         *httphandlers.BillingConfigHandler
	EarlyAdopterMigrationHandler *httphandlers.EarlyAdopterMigrationHandler
	WebhookHandler               *httphandlers.WebhookHandler
	EmailWebhookHandler          *httphandlers.EmailWebhookHandler
	AgentHandler                 *httphandlers.AgentHandler

	// VerifyBillingConfigUC powers the boot-time billing config self-check.
	VerifyBillingConfigUC *usecases.VerifyBillingConfigUseCase
}

// ContainerConfig holds configuration for dependency injection.
type ContainerConfig struct {
	CoreClient  clients.CoreClient
	LSClient    clients.LemonSqueezyClient
	EmailClient clients.EmailClient
	KeySigner   usecases.KeySignerFunc
	CDPClient   clients.CDPClient
}

// CoreWalletLookup implements x402.WalletLookup using Core's config store.
// The wallet info is written at agent registration time by RegisterAgentUseCase.
type CoreWalletLookup struct {
	coreClient clients.CoreClient
}

// GetAgentWallet retrieves the CDP wallet ID and address for a tenant from Core config.
func (l *CoreWalletLookup) GetAgentWallet(ctx context.Context, tenantID string) (walletID, address string, err error) {
	if l.coreClient == nil {
		return "", "", nil
	}
	key := "agent:" + tenantID + ":cdp_wallet"
	entry, err := l.coreClient.GetConfig(ctx, key)
	if err != nil || entry == nil {
		return "", "", nil //nolint:nilerr // no wallet is not an error
	}
	val, ok := entry.Value.(string)
	if !ok {
		return "", "", nil
	}
	if val == "" {
		return "", "", nil
	}
	// format: walletID|address
	parts := splitTwo(val, '|')
	return parts[0], parts[1], nil
}

func splitTwo(s string, sep byte) [2]string {
	for i := 0; i < len(s); i++ {
		if s[i] == sep {
			return [2]string{s[:i], s[i+1:]}
		}
	}
	return [2]string{s, ""}
}

// NewContainerWithConfig creates and wires up all dependencies using the provided config.
// Tenants, audit, and config are backed by Core via REST when CoreClient is provided.
// Falls back to in-memory repos when CoreClient is nil (for testing).
// Policies and operations remain in-memory (CP-local concerns).
func NewContainerWithConfig(cfg ContainerConfig) *Container {
	// Choose repositories based on whether CoreClient is available
	var tenantRepo repositories.TenantRepository
	var auditRepo repositories.AuditRepository
	var configRepo repositories.ConfigRepository
	if cfg.CoreClient != nil {
		// Production: Core-backed repositories (single source of truth = Core)
		tenantRepo = persistence.NewCoreTenantRepository(cfg.CoreClient)
		auditRepo = persistence.NewCoreAuditRepository(cfg.CoreClient)
		configRepo = persistence.NewCoreConfigRepository(cfg.CoreClient)
	} else {
		// Testing: in-memory repositories
		tenantRepo = persistence.NewMemoryTenantRepository()
		auditRepo = persistence.NewMemoryAuditRepository()
		configRepo = persistence.NewMemoryConfigRepository()
	}

	// In-memory repositories (CP-local concerns)
	policyRepo := persistence.NewMemoryPolicyRepository()
	operationRepo := persistence.NewMemoryOperationRepository()
	alertRuleRepo := persistence.NewMemoryAlertRuleRepository()
	sloRepo := persistence.NewMemorySLORepository()
	ipRuleRepo := persistence.NewMemoryIPRuleRepository()

	// Initialize use cases — Tenants (no coreClient needed, repo delegates directly)
	createTenantUC := usecases.NewCreateTenantUseCase(tenantRepo, auditRepo)
	getTenantUC := usecases.NewGetTenantUseCase(tenantRepo)
	listTenantsUC := usecases.NewListTenantsUseCase(tenantRepo)
	updateTenantUC := usecases.NewUpdateTenantUseCase(tenantRepo, auditRepo)
	suspendTenantUC := usecases.NewSuspendTenantUseCase(tenantRepo, auditRepo)
	activateTenantUC := usecases.NewActivateTenantUseCase(tenantRepo, auditRepo)
	bulkTenantUC := usecases.NewBulkTenantUseCase(tenantRepo, auditRepo)
	deleteTenantUC := usecases.NewDeleteTenantUseCase(tenantRepo, auditRepo)
	updateTenantQuotasUC := usecases.NewUpdateTenantQuotasUseCase(tenantRepo, auditRepo)
	getAdminTenantDetailUC := usecases.NewGetAdminTenantDetailUseCase(tenantRepo, cfg.CoreClient)
	getTenantUsageUC := usecases.NewGetTenantUsageUseCase(tenantRepo, cfg.CoreClient)

	// Initialize use cases — Policies (Layer 2)
	evaluatePolicyUC := usecases.NewEvaluatePolicyUseCase(policyRepo)
	createPolicyUC := usecases.NewCreatePolicyUseCase(policyRepo, auditRepo)
	getPolicyUC := usecases.NewGetPolicyUseCase(policyRepo)
	listPoliciesUC := usecases.NewListPoliciesUseCase(policyRepo)
	updatePolicyUC := usecases.NewUpdatePolicyUseCase(policyRepo, auditRepo)
	deletePolicyUC := usecases.NewDeletePolicyUseCase(policyRepo, auditRepo)

	// Initialize use cases — Operations (Layer 2)
	createSnapshotUC := usecases.NewCreateSnapshotUseCase(operationRepo, auditRepo, cfg.CoreClient)
	triggerCompactionUC := usecases.NewTriggerCompactionUseCase(operationRepo, auditRepo, cfg.CoreClient)
	startReplayUC := usecases.NewStartReplayUseCase(operationRepo, auditRepo, cfg.CoreClient)
	getReplayProgressUC := usecases.NewGetReplayProgressUseCase(operationRepo, cfg.CoreClient)
	cancelReplayUC := usecases.NewCancelReplayUseCase(operationRepo, auditRepo, cfg.CoreClient)
	listOperationsUC := usecases.NewListOperationsUseCase(operationRepo)
	getClusterStatusUC := usecases.NewGetClusterStatusUseCase(cfg.CoreClient)

	// Initialize use cases — Schema Governance (Layer 2)
	registerSchemaUC := usecases.NewRegisterSchemaUseCase(auditRepo, cfg.CoreClient)
	listSchemasUC := usecases.NewListSchemasUseCase(cfg.CoreClient)
	validateEventUC := usecases.NewValidateEventUseCase(cfg.CoreClient)

	// Initialize use cases — Audit Trail (Layer 2)
	queryAuditUC := usecases.NewQueryAuditUseCase(auditRepo)
	queryTokenAuditUC := usecases.NewQueryTokenAuditUseCase(auditRepo)
	detectSuspiciousActivityUC := usecases.NewDetectSuspiciousActivityUseCase(auditRepo)

	// Initialize use cases — Config (Layer 2)
	createConfigUC := usecases.NewCreateConfigUseCase(configRepo, auditRepo)
	getConfigUC := usecases.NewGetConfigUseCase(configRepo)
	listConfigsUC := usecases.NewListConfigsUseCase(configRepo)
	updateConfigUC := usecases.NewUpdateConfigUseCase(configRepo, auditRepo)
	deleteConfigUC := usecases.NewDeleteConfigUseCase(configRepo, auditRepo)

	// Initialize use cases — Alert Rules (Layer 2)
	createAlertRuleUC := usecases.NewCreateAlertRuleUseCase(alertRuleRepo, auditRepo)
	listAlertRulesUC := usecases.NewListAlertRulesUseCase(alertRuleRepo)
	updateAlertRuleUC := usecases.NewUpdateAlertRuleUseCase(alertRuleRepo, auditRepo)
	deleteAlertRuleUC := usecases.NewDeleteAlertRuleUseCase(alertRuleRepo, auditRepo)

	// Initialize use cases — IP Rules (Layer 2)
	createIPRuleUC := usecases.NewCreateIPRuleUseCase(ipRuleRepo, auditRepo)
	listIPRulesUC := usecases.NewListIPRulesUseCase(ipRuleRepo)
	deleteIPRuleUC := usecases.NewDeleteIPRuleUseCase(ipRuleRepo, auditRepo)

	// Initialize use cases — SLOs (Layer 2)
	createSLOUC := usecases.NewCreateSLOUseCase(sloRepo, auditRepo)
	listSLOsUC := usecases.NewListSLOsUseCase(sloRepo, nil) // nil MetricsProvider: defaults to 100% compliance
	deleteSLOUC := usecases.NewDeleteSLOUseCase(sloRepo, auditRepo)

	// Initialize use cases — Billing (Layer 2)
	// Checkout/portal require the LemonSqueezy payment provider.
	var createCheckoutUC *usecases.CreateCheckoutUseCase
	var getPortalUC *usecases.GetPortalUseCase
	if cfg.LSClient != nil {
		createCheckoutUC = usecases.NewCreateCheckoutUseCase(tenantRepo, cfg.LSClient, auditRepo)
		getPortalUC = usecases.NewGetPortalUseCase(tenantRepo, cfg.LSClient)
	}
	getOverageSummaryUC := usecases.NewGetOverageSummaryUseCase(tenantRepo)
	setOverageEnabledUC := usecases.NewSetOverageEnabledUseCase(tenantRepo, auditRepo)
	getProjectedChargesUC := usecases.NewGetProjectedChargesUseCase(tenantRepo)
	// Public pricing catalog, read live from LemonSqueezy (source of truth for
	// charged prices). nil LS client → empty catalog → frontend uses config.
	getCatalogUC := usecases.NewGetCatalogUseCase(cfg.LSClient)

	// Initialize use cases — Agent Registration
	registerAgentUC := usecases.NewRegisterAgentUseCase(createTenantUC, auditRepo, cfg.CoreClient, cfg.KeySigner)
	if cfg.CDPClient != nil {
		registerAgentUC = registerAgentUC.WithCDP(cfg.CDPClient)
	}
	registerTrialAgentUC := usecases.NewRegisterTrialAgentUseCase(createTenantUC, auditRepo, cfg.CoreClient, cfg.KeySigner)
	claimTrialAgentUC := usecases.NewClaimTrialAgentUseCase(tenantRepo, auditRepo, cfg.CoreClient)
	agentPaymentHistoryUC := usecases.NewGetAgentPaymentHistoryUseCase(cfg.CoreClient)

	// Initialize use cases — Webhooks
	updateSubscriptionUC := usecases.NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	migrateEarlyAdoptersUC := usecases.NewMigrateEarlyAdoptersUseCase(tenantRepo, updateSubscriptionUC, auditRepo)
	var changePlanUC *usecases.ChangePlanUseCase
	if cfg.LSClient != nil {
		changePlanUC = usecases.NewChangePlanUseCase(tenantRepo, cfg.LSClient, updateSubscriptionUC)
	}
	// Build reverse variant map (tier→variantID becomes variantName→tier) for webhook tier resolution
	var variantTierMap usecases.VariantTierMap
	if cfg.LSClient != nil {
		// Parse LEMON_SQUEEZY_VARIANT_MAP and build reverse lookup
		variantTierMap = buildVariantTierMap()
	}
	processLSWebhookUC := usecases.NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubscriptionUC, suspendTenantUC, variantTierMap, cfg.CoreClient)

	// Initialize use cases — Billing (overage reporting)
	calculateOverageUC := billing.NewCalculateOverageUseCase(tenantRepo)
	var reportUsageUC *billing.ReportUsageUseCase
	if cfg.LSClient != nil {
		reportUsageUC = billing.NewReportUsageUseCase(tenantRepo, auditRepo, cfg.LSClient, calculateOverageUC)
	}

	// Initialize use cases — Billing (usage warnings)
	var checkUsageWarningsUC *billing.CheckUsageWarningsUseCase
	if cfg.EmailClient != nil {
		checkUsageWarningsUC = billing.NewCheckUsageWarningsUseCase(tenantRepo, auditRepo, cfg.EmailClient)
	}

	// Initialize use cases — Billing (x402 allowance reconciliation, 011).
	// Reads the durable x402 events in Core to keep x402_used truthful so the
	// per-tier allowance depletes and overage kicks in.
	var syncX402UsageUC *billing.SyncX402UsageUseCase
	if cfg.CoreClient != nil {
		syncX402UsageUC = billing.NewSyncX402UsageUseCase(tenantRepo, auditRepo, cfg.CoreClient)
	}

	// Initialize use cases — Billing (subscription reconciliation, t-f23d2a).
	// Self-heals tier from LemonSqueezy when a webhook is missed/rejected.
	var syncSubsUC *usecases.SyncSubscriptionsUseCase
	if cfg.LSClient != nil {
		syncSubsUC = usecases.NewSyncSubscriptionsUseCase(tenantRepo, cfg.LSClient, updateSubscriptionUC, variantTierMap)
	}

	// Initialize use cases — Admin Billing
	var adminListInvoicesUC *billing.AdminListInvoicesUseCase
	var adminRevenueUC *billing.AdminRevenueUseCase
	var adminRefundUC *billing.AdminRefundUseCase
	adminDunningUC := billing.NewAdminDunningUseCase(tenantRepo, auditRepo)
	adminRevenueUC = billing.NewAdminRevenueUseCase(tenantRepo, auditRepo, cfg.LSClient)
	if cfg.LSClient != nil {
		adminListInvoicesUC = billing.NewAdminListInvoicesUseCase(tenantRepo, auditRepo, cfg.LSClient)
		adminRefundUC = billing.NewAdminRefundUseCase(auditRepo, cfg.LSClient)
	}

	// Initialize scheduler
	scheduler := usecases.NewOperationScheduler(operationRepo, auditRepo, cfg.CoreClient)
	if reportUsageUC != nil {
		scheduler.SetReportUsageUseCase(reportUsageUC)
	}
	if checkUsageWarningsUC != nil {
		scheduler.SetCheckUsageWarningsUseCase(checkUsageWarningsUC)
	}
	if syncX402UsageUC != nil {
		scheduler.SetSyncX402UsageUseCase(syncX402UsageUC)
	}
	if syncSubsUC != nil {
		scheduler.SetSyncSubscriptionsUseCase(syncSubsUC)
	}

	// Initialize HTTP handlers (Layer 4)
	alertHandler := httphandlers.NewAlertHandler(createAlertRuleUC, listAlertRulesUC, updateAlertRuleUC, deleteAlertRuleUC)
	sloHandler := httphandlers.NewSLOHandler(createSLOUC, listSLOsUC, deleteSLOUC)
	adminTenantHandler := httphandlers.NewAdminTenantHandler(listTenantsUC, getAdminTenantDetailUC, getTenantUsageUC, updateTenantQuotasUC, suspendTenantUC, activateTenantUC, bulkTenantUC)
	tenantHandler := httphandlers.NewTenantHandler(
		createTenantUC, getTenantUC, listTenantsUC, updateTenantUC,
		suspendTenantUC, activateTenantUC, deleteTenantUC, cfg.CoreClient,
	)
	policyHandler := httphandlers.NewPolicyHandler(
		evaluatePolicyUC, createPolicyUC, getPolicyUC,
		listPoliciesUC, updatePolicyUC, deletePolicyUC,
	)
	operationsHandler := httphandlers.NewOperationsHandler(
		createSnapshotUC, triggerCompactionUC, startReplayUC, getReplayProgressUC,
		cancelReplayUC, listOperationsUC, getClusterStatusUC, cfg.CoreClient,
	)
	schemaHandler := httphandlers.NewSchemaHandler(registerSchemaUC, listSchemasUC, validateEventUC)
	auditHandler := httphandlers.NewAuditHandler(queryAuditUC)
	tokenAuditHandler := httphandlers.NewTokenAuditHandler(queryTokenAuditUC)
	suspiciousActivityHandler := httphandlers.NewSuspiciousActivityHandler(detectSuspiciousActivityUC)
	configHandler := httphandlers.NewConfigHandler(
		createConfigUC, getConfigUC, listConfigsUC, updateConfigUC, deleteConfigUC,
	)
	billingHandler := httphandlers.NewBillingHandler(
		createCheckoutUC, changePlanUC, getPortalUC, getOverageSummaryUC, setOverageEnabledUC, getProjectedChargesUC, getCatalogUC,
	)
	ipRuleHandler := httphandlers.NewIPRuleHandler(createIPRuleUC, listIPRulesUC, deleteIPRuleUC)
	adminBillingHandler := httphandlers.NewAdminBillingHandler(adminListInvoicesUC, adminRevenueUC, adminRefundUC, adminDunningUC)
	verifyBillingConfigUC := usecases.NewVerifyBillingConfigUseCase(cfg.LSClient, os.Getenv("LEMON_SQUEEZY_WEBHOOK_SECRET"))
	billingConfigHandler := httphandlers.NewBillingConfigHandler(verifyBillingConfigUC)
	earlyAdopterMigrationHandler := httphandlers.NewEarlyAdopterMigrationHandler(migrateEarlyAdoptersUC)
	webhookHandler := httphandlers.NewWebhookHandler(processLSWebhookUC)
	// Email inbox connector (P0): enabled only when both Core and a provider are
	// configured. nil provider -> the handler returns 503. See emailprovider/nylas.
	var emailProvider emailprovider.Provider
	if cfg.CoreClient != nil {
		if np, err := nylas.NewFromEnv(); err == nil {
			emailProvider = np
		}
	}
	emailWebhookHandler := httphandlers.NewEmailWebhookHandler(emailProvider, cfg.CoreClient)
	agentHandler := httphandlers.NewAgentHandler(agentPaymentHistoryUC)

	return &Container{
		TenantRepo:                   tenantRepo,
		PolicyRepo:                   policyRepo,
		AuditRepo:                    auditRepo,
		OperationRepo:                operationRepo,
		ConfigRepo:                   configRepo,
		AlertRuleRepo:                alertRuleRepo,
		SLORepo:                      sloRepo,
		IPRuleRepo:                   ipRuleRepo,
		CreateTenantUC:               createTenantUC,
		GetTenantUC:                  getTenantUC,
		ListTenantsUC:                listTenantsUC,
		UpdateTenantUC:               updateTenantUC,
		UpdateTenantQuotasUC:         updateTenantQuotasUC,
		SuspendTenantUC:              suspendTenantUC,
		ActivateTenantUC:             activateTenantUC,
		BulkTenantUC:                 bulkTenantUC,
		DeleteTenantUC:               deleteTenantUC,
		EvaluatePolicyUC:             evaluatePolicyUC,
		CreatePolicyUC:               createPolicyUC,
		GetPolicyUC:                  getPolicyUC,
		ListPoliciesUC:               listPoliciesUC,
		UpdatePolicyUC:               updatePolicyUC,
		DeletePolicyUC:               deletePolicyUC,
		CreateSnapshotUC:             createSnapshotUC,
		TriggerCompactionUC:          triggerCompactionUC,
		StartReplayUC:                startReplayUC,
		GetReplayProgressUC:          getReplayProgressUC,
		CancelReplayUC:               cancelReplayUC,
		ListOperationsUC:             listOperationsUC,
		GetClusterStatusUC:           getClusterStatusUC,
		RegisterSchemaUC:             registerSchemaUC,
		ListSchemasUC:                listSchemasUC,
		ValidateEventUC:              validateEventUC,
		QueryAuditUC:                 queryAuditUC,
		QueryTokenAuditUC:            queryTokenAuditUC,
		DetectSuspiciousActivityUC:   detectSuspiciousActivityUC,
		CreateConfigUC:               createConfigUC,
		GetConfigUC:                  getConfigUC,
		ListConfigsUC:                listConfigsUC,
		UpdateConfigUC:               updateConfigUC,
		DeleteConfigUC:               deleteConfigUC,
		CreateAlertRuleUC:            createAlertRuleUC,
		ListAlertRulesUC:             listAlertRulesUC,
		UpdateAlertRuleUC:            updateAlertRuleUC,
		DeleteAlertRuleUC:            deleteAlertRuleUC,
		CreateIPRuleUC:               createIPRuleUC,
		ListIPRulesUC:                listIPRulesUC,
		DeleteIPRuleUC:               deleteIPRuleUC,
		CreateSLOUC:                  createSLOUC,
		ListSLOsUC:                   listSLOsUC,
		DeleteSLOUC:                  deleteSLOUC,
		CreateCheckoutUC:             createCheckoutUC,
		GetPortalUC:                  getPortalUC,
		GetOverageSummaryUC:          getOverageSummaryUC,
		SetOverageEnabledUC:          setOverageEnabledUC,
		GetProjectedChargesUC:        getProjectedChargesUC,
		CalculateOverageUC:           calculateOverageUC,
		ReportUsageUC:                reportUsageUC,
		CheckUsageWarningsUC:         checkUsageWarningsUC,
		AdminListInvoicesUC:          adminListInvoicesUC,
		AdminRevenueUC:               adminRevenueUC,
		AdminRefundUC:                adminRefundUC,
		AdminDunningUC:               adminDunningUC,
		Scheduler:                    scheduler,
		WalletLookup:                 &CoreWalletLookup{coreClient: cfg.CoreClient},
		CDPClient:                    cfg.CDPClient,
		RegisterAgentUC:              registerAgentUC,
		RegisterTrialAgentUC:         registerTrialAgentUC,
		ClaimTrialAgentUC:            claimTrialAgentUC,
		AgentPaymentHistoryUC:        agentPaymentHistoryUC,
		AlertHandler:                 alertHandler,
		SLOHandler:                   sloHandler,
		AdminTenantHandler:           adminTenantHandler,
		TenantHandler:                tenantHandler,
		PolicyHandler:                policyHandler,
		OperationsHandler:            operationsHandler,
		SchemaHandler:                schemaHandler,
		AuditHandler:                 auditHandler,
		TokenAuditHandler:            tokenAuditHandler,
		SuspiciousActivityHandler:    suspiciousActivityHandler,
		ConfigHandler:                configHandler,
		IPRuleHandler:                ipRuleHandler,
		BillingHandler:               billingHandler,
		AdminBillingHandler:          adminBillingHandler,
		BillingConfigHandler:         billingConfigHandler,
		EarlyAdopterMigrationHandler: earlyAdopterMigrationHandler,
		WebhookHandler:               webhookHandler,
		EmailWebhookHandler:          emailWebhookHandler,
		AgentHandler:                 agentHandler,
		VerifyBillingConfigUC:        verifyBillingConfigUC,
		ProcessLSWebhookUC:           processLSWebhookUC,
		UpdateSubscriptionUC:         updateSubscriptionUC,
		MigrateEarlyAdoptersUC:       migrateEarlyAdoptersUC,
	}
}

// NewContainer creates and wires up all dependencies using in-memory repositories.
// Retained for backward compatibility with existing tests.
func NewContainer() *Container {
	return NewContainerWithConfig(ContainerConfig{})
}
