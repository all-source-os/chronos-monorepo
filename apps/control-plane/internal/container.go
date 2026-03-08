// Package internal contains the dependency injection container and internal wiring.
package internal

import (
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/application/usecases/billing"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
	httphandlers "github.com/allsource/control-plane/internal/interfaces/http"
)

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

	// Use Cases — Webhooks
	ProcessLSWebhookUC     *usecases.ProcessLemonSqueezyWebhookUseCase
	ProcessStripeWebhookUC *usecases.ProcessStripeWebhookUseCase
	UpdateSubscriptionUC   *usecases.UpdateSubscriptionMetadataUseCase

	// Scheduler
	Scheduler *usecases.OperationScheduler

	// HTTP Handlers
	AlertHandler              *httphandlers.AlertHandler
	SLOHandler                *httphandlers.SLOHandler
	AdminTenantHandler        *httphandlers.AdminTenantHandler
	TenantHandler             *httphandlers.TenantHandler
	PolicyHandler             *httphandlers.PolicyHandler
	OperationsHandler         *httphandlers.OperationsHandler
	SchemaHandler             *httphandlers.SchemaHandler
	AuditHandler              *httphandlers.AuditHandler
	TokenAuditHandler         *httphandlers.TokenAuditHandler
	SuspiciousActivityHandler *httphandlers.SuspiciousActivityHandler
	ConfigHandler             *httphandlers.ConfigHandler
	IPRuleHandler             *httphandlers.IPRuleHandler
	BillingHandler            *httphandlers.BillingHandler
	AdminBillingHandler       *httphandlers.AdminBillingHandler
	WebhookHandler            *httphandlers.WebhookHandler
}

// ContainerConfig holds configuration for dependency injection.
type ContainerConfig struct {
	CoreClient   clients.CoreClient
	LSClient     clients.LemonSqueezyClient
	StripeClient clients.StripeClient
	EmailClient  clients.EmailClient
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
	// Checkout/portal require at least one payment provider (LS or Stripe)
	var createCheckoutUC *usecases.CreateCheckoutUseCase
	var getPortalUC *usecases.GetPortalUseCase
	if cfg.LSClient != nil || cfg.StripeClient != nil {
		createCheckoutUC = usecases.NewCreateCheckoutUseCase(tenantRepo, cfg.LSClient, cfg.StripeClient, auditRepo)
		getPortalUC = usecases.NewGetPortalUseCase(tenantRepo, cfg.LSClient, cfg.StripeClient)
	}
	getOverageSummaryUC := usecases.NewGetOverageSummaryUseCase(tenantRepo)
	setOverageEnabledUC := usecases.NewSetOverageEnabledUseCase(tenantRepo, auditRepo)
	getProjectedChargesUC := usecases.NewGetProjectedChargesUseCase(tenantRepo)

	// Initialize use cases — Webhooks
	updateSubscriptionUC := usecases.NewUpdateSubscriptionMetadataUseCase(tenantRepo, auditRepo)
	processLSWebhookUC := usecases.NewProcessLemonSqueezyWebhookUseCase(tenantRepo, auditRepo, updateSubscriptionUC, suspendTenantUC)
	processStripeWebhookUC := usecases.NewProcessStripeWebhookUseCase(tenantRepo, auditRepo, updateSubscriptionUC, suspendTenantUC)

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
		createCheckoutUC, getPortalUC, getOverageSummaryUC, setOverageEnabledUC, getProjectedChargesUC,
	)
	ipRuleHandler := httphandlers.NewIPRuleHandler(createIPRuleUC, listIPRulesUC, deleteIPRuleUC)
	adminBillingHandler := httphandlers.NewAdminBillingHandler(adminListInvoicesUC, adminRevenueUC, adminRefundUC, adminDunningUC)
	webhookHandler := httphandlers.NewWebhookHandler(processLSWebhookUC, processStripeWebhookUC)

	return &Container{
		TenantRepo:                 tenantRepo,
		PolicyRepo:                 policyRepo,
		AuditRepo:                  auditRepo,
		OperationRepo:              operationRepo,
		ConfigRepo:                 configRepo,
		AlertRuleRepo:              alertRuleRepo,
		SLORepo:                    sloRepo,
		IPRuleRepo:                 ipRuleRepo,
		CreateTenantUC:             createTenantUC,
		GetTenantUC:                getTenantUC,
		ListTenantsUC:              listTenantsUC,
		UpdateTenantUC:             updateTenantUC,
		UpdateTenantQuotasUC:       updateTenantQuotasUC,
		SuspendTenantUC:            suspendTenantUC,
		ActivateTenantUC:           activateTenantUC,
		BulkTenantUC:               bulkTenantUC,
		DeleteTenantUC:             deleteTenantUC,
		EvaluatePolicyUC:           evaluatePolicyUC,
		CreatePolicyUC:             createPolicyUC,
		GetPolicyUC:                getPolicyUC,
		ListPoliciesUC:             listPoliciesUC,
		UpdatePolicyUC:             updatePolicyUC,
		DeletePolicyUC:             deletePolicyUC,
		CreateSnapshotUC:           createSnapshotUC,
		TriggerCompactionUC:        triggerCompactionUC,
		StartReplayUC:              startReplayUC,
		GetReplayProgressUC:        getReplayProgressUC,
		CancelReplayUC:             cancelReplayUC,
		ListOperationsUC:           listOperationsUC,
		GetClusterStatusUC:         getClusterStatusUC,
		RegisterSchemaUC:           registerSchemaUC,
		ListSchemasUC:              listSchemasUC,
		ValidateEventUC:            validateEventUC,
		QueryAuditUC:               queryAuditUC,
		QueryTokenAuditUC:          queryTokenAuditUC,
		DetectSuspiciousActivityUC: detectSuspiciousActivityUC,
		CreateConfigUC:             createConfigUC,
		GetConfigUC:                getConfigUC,
		ListConfigsUC:              listConfigsUC,
		UpdateConfigUC:             updateConfigUC,
		DeleteConfigUC:             deleteConfigUC,
		CreateAlertRuleUC:          createAlertRuleUC,
		ListAlertRulesUC:           listAlertRulesUC,
		UpdateAlertRuleUC:          updateAlertRuleUC,
		DeleteAlertRuleUC:          deleteAlertRuleUC,
		CreateIPRuleUC:             createIPRuleUC,
		ListIPRulesUC:              listIPRulesUC,
		DeleteIPRuleUC:             deleteIPRuleUC,
		CreateSLOUC:                createSLOUC,
		ListSLOsUC:                 listSLOsUC,
		DeleteSLOUC:                deleteSLOUC,
		CreateCheckoutUC:           createCheckoutUC,
		GetPortalUC:                getPortalUC,
		GetOverageSummaryUC:        getOverageSummaryUC,
		SetOverageEnabledUC:        setOverageEnabledUC,
		GetProjectedChargesUC:      getProjectedChargesUC,
		CalculateOverageUC:         calculateOverageUC,
		ReportUsageUC:              reportUsageUC,
		CheckUsageWarningsUC:       checkUsageWarningsUC,
		AdminListInvoicesUC:        adminListInvoicesUC,
		AdminRevenueUC:             adminRevenueUC,
		AdminRefundUC:              adminRefundUC,
		AdminDunningUC:             adminDunningUC,
		Scheduler:                  scheduler,
		AlertHandler:               alertHandler,
		SLOHandler:                 sloHandler,
		AdminTenantHandler:         adminTenantHandler,
		TenantHandler:              tenantHandler,
		PolicyHandler:              policyHandler,
		OperationsHandler:          operationsHandler,
		SchemaHandler:              schemaHandler,
		AuditHandler:               auditHandler,
		TokenAuditHandler:          tokenAuditHandler,
		SuspiciousActivityHandler:  suspiciousActivityHandler,
		ConfigHandler:              configHandler,
		IPRuleHandler:              ipRuleHandler,
		BillingHandler:             billingHandler,
		AdminBillingHandler:        adminBillingHandler,
		WebhookHandler:             webhookHandler,
		ProcessLSWebhookUC:         processLSWebhookUC,
		ProcessStripeWebhookUC:     processStripeWebhookUC,
		UpdateSubscriptionUC:       updateSubscriptionUC,
	}
}

// NewContainer creates and wires up all dependencies using in-memory repositories.
// Retained for backward compatibility with existing tests.
func NewContainer() *Container {
	return NewContainerWithConfig(ContainerConfig{})
}
