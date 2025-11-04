package internal

import (
	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
	httphandlers "github.com/allsource/control-plane/internal/interfaces/http"
)

// Container holds all application dependencies
type Container struct {
	// Repositories
	TenantRepo repositories.TenantRepository
	UserRepo   repositories.UserRepository
	PolicyRepo repositories.PolicyRepository
	AuditRepo  repositories.AuditRepository

	// Use Cases
	CreateTenantUC   *usecases.CreateTenantUseCase
	EvaluatePolicyUC *usecases.EvaluatePolicyUseCase

	// HTTP Handlers
	TenantHandler *httphandlers.TenantHandler
	PolicyHandler *httphandlers.PolicyHandler
}

// NewContainer creates and wires up all dependencies
func NewContainer() *Container {
	// Initialize repositories (Layer 3)
	tenantRepo := persistence.NewMemoryTenantRepository()
	userRepo := persistence.NewMemoryUserRepository()
	policyRepo := persistence.NewMemoryPolicyRepository()
	auditRepo := persistence.NewMemoryAuditRepository()

	// Initialize use cases (Layer 2)
	createTenantUC := usecases.NewCreateTenantUseCase(tenantRepo, auditRepo)
	evaluatePolicyUC := usecases.NewEvaluatePolicyUseCase(policyRepo)

	// Initialize HTTP handlers (Layer 4)
	tenantHandler := httphandlers.NewTenantHandler(createTenantUC)
	policyHandler := httphandlers.NewPolicyHandler(evaluatePolicyUC)

	return &Container{
		TenantRepo:       tenantRepo,
		UserRepo:         userRepo,
		PolicyRepo:       policyRepo,
		AuditRepo:        auditRepo,
		CreateTenantUC:   createTenantUC,
		EvaluatePolicyUC: evaluatePolicyUC,
		TenantHandler:    tenantHandler,
		PolicyHandler:    policyHandler,
	}
}
