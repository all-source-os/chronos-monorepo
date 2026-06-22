package usecases

import (
	"context"
	"net/http"
	"sort"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/application/usecases/signals"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// FleetHealthUseCase computes the fleet-wide and per-tenant health model from
// the systems that already hold the signals (CONTROL_PLANE_TENANT_HEALTH_RECOVERY.md
// §2). It owns the health logic ONCE; the admin UI and MCP are thin consumers.
//
// Architecture invariants honored here (CLAUDE.md / MEMORY.md):
//   - Core IS the database. Recency is read with a `?limit=1&order=desc` probe,
//     NOT a stored last_event_at column (get_tenant_usage.go gap).
//   - "empty data" is a read-path/identity symptom, never data loss.
//   - No PostgreSQL — every signal is a Core / Core-metadata / QS read.
type FleetHealthUseCase struct {
	tenantRepo repositories.TenantRepository
	coreClient clients.CoreClient
	httpClient *http.Client

	// queryHealthURL is the QS /health endpoint used by the edition probe. The
	// edition trap is a Query Service setting, never a CP one
	// (apps/query-service/lib/query_service_ex/edition.ex). We HTTP-probe it; we
	// never import QS (monorepo isolation).
	queryHealthURL string

	// nonCommunityDataTenantsThreshold is the §2.1 edition-trap heuristic
	// boundary: QS edition=community while the fleet has >= N non-community
	// tenants with data is the runbook §5 #5 failure.
	editionTrapThreshold int
}

// FleetHealthConfig configures the use case.
type FleetHealthConfig struct {
	TenantRepo     repositories.TenantRepository
	CoreClient     clients.CoreClient
	HTTPClient     *http.Client
	QueryHealthURL string
}

// NewFleetHealthUseCase creates a FleetHealthUseCase.
func NewFleetHealthUseCase(cfg FleetHealthConfig) *FleetHealthUseCase {
	hc := cfg.HTTPClient
	if hc == nil {
		hc = &http.Client{Timeout: 4 * time.Second}
	}
	return &FleetHealthUseCase{
		tenantRepo:           cfg.TenantRepo,
		coreClient:           cfg.CoreClient,
		httpClient:           hc,
		queryHealthURL:       cfg.QueryHealthURL,
		editionTrapThreshold: 2,
	}
}

// FleetSummary holds the per-tier counts.
type FleetSummary struct {
	Total    int `json:"total"`
	Healthy  int `json:"healthy"`
	Degraded int `json:"degraded"`
	AtRisk   int `json:"at_risk"`
	Critical int `json:"critical"`
}

// WorstTenant is one entry in the worst-N list.
type WorstTenant struct {
	TenantID string           `json:"tenant_id"`
	Name     string           `json:"name"`
	Tier     string           `json:"tier"`
	Reasons  []signals.Signal `json:"reasons"`
}

// FleetHealthResult is the aggregate returned by the fleet endpoint.
type FleetHealthResult struct {
	Summary FleetSummary  `json:"summary"`
	Worst   []WorstTenant `json:"worst"`
}

// TenantHealthResult is the full single-tenant assessment.
type TenantHealthResult struct {
	TenantID     string                         `json:"tenant_id"`
	Name         string                         `json:"name"`
	Tier         string                         `json:"tier"`
	BillingTier  string                         `json:"billing_tier"`
	Signals      []signals.Signal               `json:"signals"`
	Subscription *entities.SubscriptionMetadata `json:"subscription,omitempty"`
}

// fleetContext holds the fleet-wide signal inputs computed once per fleet pass
// (edition trap, durability, replication) so every tenant reuses them.
type fleetContext struct {
	editionTrap    bool
	durable        bool
	durWarnings    []string
	replMaxLagMs   int64
	replUnreachabl bool
	now            time.Time
}

// ComputeFleet lists every tenant, assesses each, and rolls up the counts +
// worst-N. tierFilter (optional: "critical"/"at_risk"/"degraded"/"healthy")
// limits the worst list; limit caps it (default 25).
func (uc *FleetHealthUseCase) ComputeFleet(ctx context.Context, tierFilter string, limit int) (*FleetHealthResult, error) {
	if limit <= 0 {
		limit = 25
	}
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return nil, err
	}

	fctx := uc.buildFleetContext(ctx, tenants)

	summary := FleetSummary{Total: len(tenants)}
	worst := make([]WorstTenant, 0, len(tenants))

	for _, t := range tenants {
		assess := uc.assessTenant(ctx, t, fctx)
		switch assess.Tier {
		case signals.TierCritical:
			summary.Critical++
		case signals.TierAtRisk:
			summary.AtRisk++
		case signals.TierDegraded:
			summary.Degraded++
		default:
			summary.Healthy++
		}

		if assess.Tier == signals.TierHealthy {
			continue // healthy tenants never appear in worst-N
		}
		if tierFilter != "" && string(assess.Tier) != tierFilter {
			continue
		}
		worst = append(worst, WorstTenant{
			TenantID: t.ID,
			Name:     t.Name,
			Tier:     string(assess.Tier),
			Reasons:  assess.Reasons(),
		})
	}

	// Sort worst-first by severity, then tenant id for stability.
	sort.SliceStable(worst, func(i, j int) bool {
		si := signals.Tier(worst[i].Tier).Severity()
		sj := signals.Tier(worst[j].Tier).Severity()
		if si != sj {
			return si > sj
		}
		return worst[i].TenantID < worst[j].TenantID
	})
	if len(worst) > limit {
		worst = worst[:limit]
	}

	return &FleetHealthResult{Summary: summary, Worst: worst}, nil
}

// ComputeTenant assesses a single tenant and returns the full signal list.
func (uc *FleetHealthUseCase) ComputeTenant(ctx context.Context, id string) (*TenantHealthResult, error) {
	t, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}
	// For a single-tenant view we still need the fleet-wide signals (edition,
	// durability, replication), but the edition heuristic needs the whole fleet.
	allTenants, ferr := uc.tenantRepo.FindAll()
	if ferr != nil {
		allTenants = []*entities.Tenant{t}
	}
	fctx := uc.buildFleetContext(ctx, allTenants)

	assess := uc.assessTenant(ctx, t, fctx)
	sub := extractSubscriptionForHealth(t.Metadata)

	return &TenantHealthResult{
		TenantID:     t.ID,
		Name:         t.Name,
		Tier:         string(assess.Tier),
		BillingTier:  assess.EffTier,
		Signals:      assess.Signals,
		Subscription: &sub,
	}, nil
}

// assessTenant evaluates all 11 signals for one tenant and rolls them up.
func (uc *FleetHealthUseCase) assessTenant(ctx context.Context, t *entities.Tenant, fctx fleetContext) signals.Assessment {
	now := fctx.now
	sub := extractSubscriptionForHealth(t.Metadata)
	quota := extractQuotaForHealth(t.Metadata)
	overage := extractOverageEnabled(t.Metadata)

	// Effective billing tier (highest active among tracked subs; falls back to
	// the flat subscription.tier).
	billingTier := effectiveBillingTier(t.Metadata, sub)
	paid := signals.PaidTier(billingTier)

	// Recency probes (desc, limit 1). Best-effort: a Core error leaves the
	// timestamp zero, which the evaluator treats as "never ingested".
	stats := uc.tenantStats(ctx, t.ID)
	hasData := stats != nil && stats.EventCount > 0
	newestEvent := uc.newestEventTime(ctx, t.ID, "")
	newestSync := uc.newestEventTime(ctx, t.ID, "prime.")
	hasSync := !newestSync.IsZero()

	grandfatherActive := sub.IsGrandfathered(now)

	sigs := []signals.Signal{
		signals.LastEventAge(now, newestEvent, hasData, paid),
		signals.LastSyncAge(now, newestSync, hasSync),
		signals.EventsQuotaPct(quota.EventsUsed, quota.EventsQuota, overage),
		signals.SubscriptionState(sub, now),
		signals.GrandfatherWindow(sub, now),
		signals.EditionTrap(fctx.editionTrap),
		signals.Durability(fctx.durable, fctx.durWarnings),
		signals.ReplicationLag(fctx.replMaxLagMs, fctx.replUnreachabl),
		// api_key_validity + empty_read_symptom_rate are surfaced as Healthy by
		// default; CP cannot cheaply probe a representative key per tenant on
		// every fleet pass without minting/validating keys (a write), and the
		// empty-read rate needs a QS read-symptom counter that is not yet
		// exposed. They are wired so the model is complete and the recovery
		// diagnostics (diagnose-identity, rotate-keys) populate them on demand.
		signals.APIKeyValidity(true, false),
		signals.EmptyReadSymptom(0, hasData),
	}

	tier := signals.Rollup(sigs, grandfatherActive)
	return signals.Assessment{
		TenantID: t.ID,
		Name:     t.Name,
		EffTier:  billingTier,
		Tier:     tier,
		Signals:  sigs,
	}
}

// buildFleetContext computes the fleet-wide signal inputs once.
func (uc *FleetHealthUseCase) buildFleetContext(ctx context.Context, tenants []*entities.Tenant) fleetContext {
	fctx := fleetContext{now: time.Now(), durable: true}

	// Durability: read Core's deep health. Core /health returns status + details;
	// we treat a non-"healthy" status as a durability concern.
	if uc.coreClient != nil {
		if h, err := uc.coreClient.HealthCheck(ctx); err == nil && h != nil {
			fctx.durable, fctx.durWarnings = durabilityFromHealth(h)
		}
	}

	// Edition trap (cross-service): probe QS /health for an edition field; if the
	// field is absent (current QS does not expose it) fall back to the runbook
	// heuristic — community edition pins every request to the community tenant,
	// the smell of which is "QS edition=community while >=N non-community tenants
	// have data". We can only assert the heuristic's fleet half here; the QS
	// probe contributes the edition value when present.
	edition := uc.probeQSEdition(ctx)
	if edition == "community" {
		if countNonCommunityDataTenants(ctx, uc, tenants) >= uc.editionTrapThreshold {
			fctx.editionTrap = true
		}
	}

	// Replication lag is not yet exposed via a typed CoreClient method (cluster
	// status is a CP proxy handler). Left at zero/false = Healthy until a typed
	// follower-status read lands; the signal is wired so the model is complete.

	return fctx
}

// --- Core probe helpers ---

func (uc *FleetHealthUseCase) tenantStats(ctx context.Context, id string) *clients.TenantStatsResponse {
	if uc.coreClient == nil {
		return nil
	}
	stats, err := uc.coreClient.GetTenantStats(ctx, id)
	if err != nil {
		return nil
	}
	return stats
}

// newestEventTime issues the documented desc-probe and returns the newest
// matching event's timestamp (zero time if none / Core error). eventTypePrefix
// filters to a sync source when non-empty.
func (uc *FleetHealthUseCase) newestEventTime(ctx context.Context, tenantID, eventTypePrefix string) time.Time {
	if uc.coreClient == nil {
		return time.Time{}
	}
	resp, err := uc.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
		TenantID:  tenantID,
		EventType: eventTypePrefix,
		Limit:     1,
		Order:     "desc",
	})
	if err != nil || resp == nil || len(resp.Events) == 0 {
		return time.Time{}
	}
	ts, _ := signals.ParseEventTime(resp.Events[0].Timestamp)
	return ts
}

// probeQSEdition hits QS /health and returns the edition string if present
// ("community"/"enterprise"), else "". Best-effort; a probe failure returns "".
func (uc *FleetHealthUseCase) probeQSEdition(ctx context.Context) string {
	if uc.queryHealthURL == "" {
		return ""
	}
	body, ok := httpGetJSON(ctx, uc.httpClient, uc.queryHealthURL)
	if !ok {
		return ""
	}
	// QS /health may carry edition under "edition" or nested "details.edition".
	if v, ok := body["edition"].(string); ok && v != "" {
		return strings.ToLower(v)
	}
	if checks, ok := body["checks"].(map[string]any); ok {
		if v, ok := checks["edition"].(string); ok && v != "" {
			return strings.ToLower(v)
		}
	}
	return ""
}

// QSEdition exposes the probed edition for the diagnose-edition endpoint.
func (uc *FleetHealthUseCase) QSEdition(ctx context.Context) string {
	return uc.probeQSEdition(ctx)
}

// EditionTrapDetected re-runs the trap heuristic for the diagnose endpoint and
// returns (detected, edition, nonCommunityDataTenants).
func (uc *FleetHealthUseCase) EditionTrapDetected(ctx context.Context) (bool, string, int) {
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return false, "", 0
	}
	edition := uc.probeQSEdition(ctx)
	n := countNonCommunityDataTenants(ctx, uc, tenants)
	detected := edition == "community" && n >= uc.editionTrapThreshold
	return detected, edition, n
}

// CoreClient exposes the underlying Core client for recovery use cases that need
// it (e.g. identity divergence probe). Read-only accessor.
func (uc *FleetHealthUseCase) CoreClient() clients.CoreClient { return uc.coreClient }

// TenantRepo exposes the tenant repository (read-only accessor).
func (uc *FleetHealthUseCase) TenantRepo() repositories.TenantRepository { return uc.tenantRepo }
