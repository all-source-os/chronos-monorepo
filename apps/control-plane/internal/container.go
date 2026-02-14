// Package internal contains the dependency injection container and internal wiring.
package internal

import (
	"github.com/allsource/control-plane/internal/application/usecases"
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

	// Use Cases — Tenants
	CreateTenantUC   *usecases.CreateTenantUseCase
	GetTenantUC      *usecases.GetTenantUseCase
	ListTenantsUC    *usecases.ListTenantsUseCase
	UpdateTenantUC   *usecases.UpdateTenantUseCase
	SuspendTenantUC  *usecases.SuspendTenantUseCase
	ActivateTenantUC *usecases.ActivateTenantUseCase
	DeleteTenantUC   *usecases.DeleteTenantUseCase

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
	QueryAuditUC *usecases.QueryAuditUseCase

	// Use Cases — Config
	CreateConfigUC *usecases.CreateConfigUseCase
	GetConfigUC    *usecases.GetConfigUseCase
	ListConfigsUC  *usecases.ListConfigsUseCase
	UpdateConfigUC *usecases.UpdateConfigUseCase
	DeleteConfigUC *usecases.DeleteConfigUseCase

	// Scheduler
	Scheduler *usecases.OperationScheduler

	// HTTP Handlers
	TenantHandler     *httphandlers.TenantHandler
	PolicyHandler     *httphandlers.PolicyHandler
	OperationsHandler *httphandlers.OperationsHandler
	SchemaHandler     *httphandlers.SchemaHandler
	AuditHandler      *httphandlers.AuditHandler
	ConfigHandler     *httphandlers.ConfigHandler
}

// ContainerConfig holds configuration for dependency injection.
type ContainerConfig struct {
	CoreClient clients.CoreClient
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

	// Initialize use cases — Tenants (no coreClient needed, repo delegates directly)
	createTenantUC := usecases.NewCreateTenantUseCase(tenantRepo, auditRepo)
	getTenantUC := usecases.NewGetTenantUseCase(tenantRepo)
	listTenantsUC := usecases.NewListTenantsUseCase(tenantRepo)
	updateTenantUC := usecases.NewUpdateTenantUseCase(tenantRepo, auditRepo)
	suspendTenantUC := usecases.NewSuspendTenantUseCase(tenantRepo, auditRepo)
	activateTenantUC := usecases.NewActivateTenantUseCase(tenantRepo, auditRepo)
	deleteTenantUC := usecases.NewDeleteTenantUseCase(tenantRepo, auditRepo)

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

	// Initialize use cases — Config (Layer 2)
	createConfigUC := usecases.NewCreateConfigUseCase(configRepo, auditRepo)
	getConfigUC := usecases.NewGetConfigUseCase(configRepo)
	listConfigsUC := usecases.NewListConfigsUseCase(configRepo)
	updateConfigUC := usecases.NewUpdateConfigUseCase(configRepo, auditRepo)
	deleteConfigUC := usecases.NewDeleteConfigUseCase(configRepo, auditRepo)

	// Initialize scheduler
	scheduler := usecases.NewOperationScheduler(operationRepo, auditRepo, cfg.CoreClient)

	// Initialize HTTP handlers (Layer 4)
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
	configHandler := httphandlers.NewConfigHandler(
		createConfigUC, getConfigUC, listConfigsUC, updateConfigUC, deleteConfigUC,
	)

	return &Container{
		TenantRepo:          tenantRepo,
		PolicyRepo:          policyRepo,
		AuditRepo:           auditRepo,
		OperationRepo:       operationRepo,
		ConfigRepo:          configRepo,
		CreateTenantUC:      createTenantUC,
		GetTenantUC:         getTenantUC,
		ListTenantsUC:       listTenantsUC,
		UpdateTenantUC:      updateTenantUC,
		SuspendTenantUC:     suspendTenantUC,
		ActivateTenantUC:    activateTenantUC,
		DeleteTenantUC:      deleteTenantUC,
		EvaluatePolicyUC:    evaluatePolicyUC,
		CreatePolicyUC:      createPolicyUC,
		GetPolicyUC:         getPolicyUC,
		ListPoliciesUC:      listPoliciesUC,
		UpdatePolicyUC:      updatePolicyUC,
		DeletePolicyUC:      deletePolicyUC,
		CreateSnapshotUC:    createSnapshotUC,
		TriggerCompactionUC: triggerCompactionUC,
		StartReplayUC:       startReplayUC,
		GetReplayProgressUC: getReplayProgressUC,
		CancelReplayUC:      cancelReplayUC,
		ListOperationsUC:    listOperationsUC,
		GetClusterStatusUC:  getClusterStatusUC,
		RegisterSchemaUC:    registerSchemaUC,
		ListSchemasUC:       listSchemasUC,
		ValidateEventUC:     validateEventUC,
		QueryAuditUC:        queryAuditUC,
		CreateConfigUC:      createConfigUC,
		GetConfigUC:         getConfigUC,
		ListConfigsUC:       listConfigsUC,
		UpdateConfigUC:      updateConfigUC,
		DeleteConfigUC:      deleteConfigUC,
		Scheduler:           scheduler,
		TenantHandler:       tenantHandler,
		PolicyHandler:       policyHandler,
		OperationsHandler:   operationsHandler,
		SchemaHandler:       schemaHandler,
		AuditHandler:        auditHandler,
		ConfigHandler:       configHandler,
	}
}

// NewContainer creates and wires up all dependencies using in-memory repositories.
// Retained for backward compatibility with existing tests.
func NewContainer() *Container {
	return NewContainerWithConfig(ContainerConfig{})
}
