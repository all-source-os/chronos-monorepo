package http

import (
	"errors"
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain"
)

// FleetHealthHandler exposes the read-only fleet-health + diagnose endpoints
// (§5.1 P0). It is registered inside the existing /api/v1/admin group, so it
// inherits AdminAuthMiddleware with zero new auth code.
//
// Where admin-ness originates (real-code note, §6.2): the admin gate
// (AdminAuthMiddleware, admin_middleware.go:69) enforces the JWT claim
// `role == "admin"`. It does NOT read ADMIN_EMAILS. The allowlist is applied
// UPSTREAM at token-mint — auth.go:roleForEmail() maps an email in the
// comma-separated ADMIN_EMAILS env to entities.RoleAdmin when the JWT is signed
// at login/OAuth callback (main.go LoginHandler / OAuthCallback). So whoever the
// allowlist admits is exactly who reaches these endpoints; there is intentionally
// NO second allowlist here — we trust the same role claim the rest of
// /api/v1/admin trusts.
type FleetHealthHandler struct {
	fleetUC *usecases.FleetHealthUseCase
}

// NewFleetHealthHandler creates a FleetHealthHandler.
func NewFleetHealthHandler(fleetUC *usecases.FleetHealthUseCase) *FleetHealthHandler {
	return &FleetHealthHandler{fleetUC: fleetUC}
}

// fleetHealthHALResponse wraps the fleet aggregate with HAL _links.
type fleetHealthHALResponse struct {
	HALResource
	Summary usecases.FleetSummary  `json:"summary"`
	Worst   []usecases.WorstTenant `json:"worst"`
}

// GetFleetHealth handles GET /api/v1/admin/fleet/health?tier=&limit=.
func (h *FleetHealthHandler) GetFleetHealth(c *gin.Context) {
	tier := c.Query("tier")
	limit, _ := strconv.Atoi(c.DefaultQuery("limit", "25")) //nolint:errcheck // default is a valid int

	result, err := h.fleetUC.ComputeFleet(c.Request.Context(), tier, limit)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, fleetHealthHALResponse{
		HALResource: HALResource{Links: map[string]Link{
			"self":             SelfLink("/api/v1/admin/fleet/health"),
			"tenant_template":  NewLink("/api/v1/admin/fleet/health/{id}", WithTemplated(), WithTitle("Single-tenant health")),
			"diagnose_edition": NewLink("/api/v1/admin/recovery/diagnose/edition", WithTitle("Diagnose edition trap")),
		}},
		Summary: result.Summary,
		Worst:   result.Worst,
	})
}

// tenantHealthHALResponse wraps a single-tenant assessment with HAL _links.
type tenantHealthHALResponse struct {
	HALResource
	*usecases.TenantHealthResult
}

// GetTenantHealth handles GET /api/v1/admin/fleet/health/:id.
func (h *FleetHealthHandler) GetTenantHealth(c *gin.Context) {
	id := c.Param("id")

	result, err := h.fleetUC.ComputeTenant(c.Request.Context(), id)
	if err != nil {
		if errors.Is(err, domain.ErrTenantNotFound) {
			c.JSON(http.StatusNotFound, gin.H{"error": err.Error()})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, tenantHealthHALResponse{
		HALResource: HALResource{Links: map[string]Link{
			"self":              SelfLink("/api/v1/admin/fleet/health/" + id),
			"fleet":             NewLink("/api/v1/admin/fleet/health", WithTitle("Fleet health")),
			"tenant":            NewLink("/api/v1/admin/tenants/"+id, WithTitle("Tenant detail")),
			"diagnose_identity": NewLink("/api/v1/admin/recovery/"+id+"/diagnose-identity", WithTitle("Diagnose identity divergence")),
		}},
		TenantHealthResult: result,
	})
}

// DiagnoseEdition handles GET /api/v1/admin/recovery/diagnose/edition (Safe,
// read-only). Detects the edition trap and returns the exact remediation command.
func (h *FleetHealthHandler) DiagnoseEdition(c *gin.Context) {
	d := h.fleetUC.DiagnoseEdition(c.Request.Context())
	c.JSON(http.StatusOK, gin.H{
		"_links":    map[string]Link{"self": SelfLink("/api/v1/admin/recovery/diagnose/edition")},
		"diagnosis": d,
	})
}

// DiagnoseIdentity handles GET /api/v1/admin/recovery/:id/diagnose-identity
// (Safe, read-only). Compares the caller's JWT tenant against the requested
// tenant + the store, and returns the re-login fix. No mutation.
func (h *FleetHealthHandler) DiagnoseIdentity(c *gin.Context) {
	id := c.Param("id")

	// Pull the caller's claimed tenant + email from the admin auth context
	// (populated by AdminAuthMiddleware from the JWT claims).
	var jwtTenant, jwtEmail string
	if actx, err := GetAdminAuthContext(c); err == nil && actx != nil {
		jwtTenant = actx.TenantID
		jwtEmail = actx.Email
	}

	d := h.fleetUC.DiagnoseIdentity(c.Request.Context(), id, jwtTenant, jwtEmail)
	c.JSON(http.StatusOK, gin.H{
		"_links":    map[string]Link{"self": SelfLink("/api/v1/admin/recovery/" + id + "/diagnose-identity")},
		"diagnosis": d,
	})
}
