package usecases

import (
	"context"
	"fmt"
	"strings"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// analyzeMockCore implements the three CoreClient methods the analysis touches:
// GetTenantStats (event totals cross-check), GetConfig (team-members), and
// ListTenants (is_demo flags). The embedded interface satisfies the rest and
// panics if an un-mocked method is called — catching any accidental write.
type analyzeMockCore struct {
	clients.CoreClient

	statsByTenant map[string]int64
	demoTenants   map[string]bool
	listErr       bool
}

func newAnalyzeMockCore() *analyzeMockCore {
	return &analyzeMockCore{
		statsByTenant: map[string]int64{},
		demoTenants:   map[string]bool{},
	}
}

func (m *analyzeMockCore) GetTenantStats(_ context.Context, tenantID string) (*clients.TenantStatsResponse, error) {
	v, ok := m.statsByTenant[tenantID]
	if !ok {
		return nil, fmt.Errorf("no stats for %s", tenantID)
	}
	return &clients.TenantStatsResponse{TenantID: tenantID, EventCount: v}, nil
}

func (m *analyzeMockCore) GetConfig(_ context.Context, _ string) (*clients.ConfigEntryResponse, error) {
	// No member list stored → clean 0 (ok=true path in memberCountFromCore).
	return nil, nil
}

func (m *analyzeMockCore) ListTenants(_ context.Context) (*clients.ListTenantsResponse, error) {
	if m.listErr {
		return nil, fmt.Errorf("core list unavailable")
	}
	out := []clients.TenantResponse{}
	for id, demo := range m.demoTenants {
		out = append(out, clients.TenantResponse{ID: id, Name: id, IsDemo: demo})
	}
	return &clients.ListTenantsResponse{Tenants: out, Total: len(out)}, nil
}

// --- helpers ---

func seedAnalyzeTenant(t *testing.T, repo *persistence.MemoryTenantRepository, id string, created time.Time, md map[string]interface{}) {
	t.Helper()
	if err := repo.Save(&entities.Tenant{
		ID: id, Name: id, Status: entities.TenantStatusActive,
		CreatedAt: created, UpdatedAt: created, Metadata: md,
	}); err != nil {
		t.Fatalf("seed %s: %v", id, err)
	}
}

func subMeta(tier, status string) map[string]interface{} {
	return map[string]interface{}{
		"subscription": map[string]interface{}{"tier": tier, "status": status},
	}
}

func quotaMeta(used, quota int64) map[string]interface{} {
	return map[string]interface{}{
		"quotas": map[string]interface{}{"events_used": used, "events_quota": quota},
	}
}

// findTenant returns the analyzed tenant by id (fatal if absent).
func findTenant(t *testing.T, rep *dto.AnalysisReport, id string) dto.AnalysisTenant {
	t.Helper()
	for i := range rep.Tenants {
		if rep.Tenants[i].ID == id {
			return rep.Tenants[i]
		}
	}
	t.Fatalf("tenant %s not in report", id)
	return dto.AnalysisTenant{}
}

// hasFinding reports whether a tenant has a finding with the given code, and
// returns it.
func hasFinding(tn dto.AnalysisTenant, code string) (dto.AnalysisFinding, bool) {
	for _, f := range tn.Findings {
		if f.Code == code {
			return f, true
		}
	}
	return dto.AnalysisFinding{}, false
}

func hasFleetFinding(rep *dto.AnalysisReport, code string) (dto.AnalysisFinding, bool) {
	for _, f := range rep.FleetFindings {
		if f.Code == code {
			return f, true
		}
	}
	return dto.AnalysisFinding{}, false
}

func oldEnough() time.Time { return time.Now().Add(-72 * time.Hour) }

// --- tests ---

func TestAnalyze_CountCapArtifact(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["capped"] = backfillCountCap // exactly the cap
	seedAnalyzeTenant(t, repo, "capped", oldEnough(), subMeta("studio", "active"))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	tn := findTenant(t, rep, "capped")
	f, ok := hasFinding(tn, CodeCountCapArtifact)
	if !ok {
		t.Fatalf("expected %s finding, got %+v", CodeCountCapArtifact, tn.Findings)
	}
	if f.Severity != dto.AnalysisSeverityCritical {
		t.Errorf("cap artifact severity = %q, want critical", f.Severity)
	}
	if f.SuggestedAction == nil || f.SuggestedAction.Target != "/api/v1/admin/billing/backfill-usage" {
		t.Errorf("cap artifact action = %+v, want backfill-usage", f.SuggestedAction)
	}
	if tn.WorstSeverity != dto.AnalysisSeverityCritical {
		t.Errorf("worst_severity = %q, want critical", tn.WorstSeverity)
	}
	// A capped tenant must NOT also be flagged unmetered_or_empty.
	if _, dup := hasFinding(tn, CodeUnmeteredOrEmpty); dup {
		t.Errorf("capped tenant double-flagged as unmetered_or_empty")
	}
}

func TestAnalyze_PlanNotInCatalog(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["weird"] = 10
	seedAnalyzeTenant(t, repo, "weird", oldEnough(), subMeta("platinum", "active")) // bogus tier

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	tn := findTenant(t, rep, "weird")
	f, ok := hasFinding(tn, CodePlanNotInCatalog)
	if !ok {
		t.Fatalf("expected %s finding, got %+v", CodePlanNotInCatalog, tn.Findings)
	}
	if f.Severity != dto.AnalysisSeverityCritical {
		t.Errorf("plan_not_in_catalog severity = %q, want critical", f.Severity)
	}

	// A retired alias must NOT be flagged (it maps to a canonical tier).
	seedAnalyzeTenant(t, repo, "legacy", oldEnough(), subMeta("growth", "active")) // retired → studio
	core.statsByTenant["legacy"] = 10
	rep2, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("uc.Execute: %v", err)
	}
	if _, ok := hasFinding(findTenant(t, rep2, "legacy"), CodePlanNotInCatalog); ok {
		t.Errorf("retired tier 'growth' wrongly flagged plan_not_in_catalog")
	}
}

func TestAnalyze_DemoLitter(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["demo-1"] = 5
	core.demoTenants["demo-1"] = true
	seedAnalyzeTenant(t, repo, "demo-1", oldEnough(), subMeta("free", ""))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	tn := findTenant(t, rep, "demo-1")
	f, ok := hasFinding(tn, CodeDemoLitter)
	if !ok {
		t.Fatalf("expected %s finding, got %+v", CodeDemoLitter, tn.Findings)
	}
	if f.SuggestedAction == nil || f.SuggestedAction.Target != "/api/v1/admin/tenants/reap-demo" {
		t.Errorf("demo_litter action = %+v, want reap-demo", f.SuggestedAction)
	}
}

func TestAnalyze_EmptyTenant(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["empty"] = 0 // 0 events, no members, old → debris
	seedAnalyzeTenant(t, repo, "empty", oldEnough(), subMeta("free", ""))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	tn := findTenant(t, rep, "empty")
	if _, ok := hasFinding(tn, CodeEmptyTenant); !ok {
		t.Fatalf("expected %s finding, got %+v", CodeEmptyTenant, tn.Findings)
	}
	// A brand-new empty tenant (inside the grace window) must NOT be flagged empty.
	seedAnalyzeTenant(t, repo, "fresh", time.Now().Add(-1*time.Hour), subMeta("free", ""))
	core.statsByTenant["fresh"] = 0
	rep2, _ := uc.Execute(context.Background(), AnalyzeRequest{}) //nolint:errcheck // test asserts on the report below
	if _, ok := hasFinding(findTenant(t, rep2, "fresh"), CodeEmptyTenant); ok {
		t.Errorf("fresh tenant inside grace window wrongly flagged empty_tenant")
	}
}

func TestAnalyze_QuotaPressure(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()

	// 95% → warn
	core.statsByTenant["warn"] = 950
	seedAnalyzeTenant(t, repo, "warn", oldEnough(), mergeMeta(subMeta("indie", "active"), quotaMeta(950, 1000)))
	// 120% → critical
	core.statsByTenant["crit"] = 1200
	seedAnalyzeTenant(t, repo, "crit", oldEnough(), mergeMeta(subMeta("indie", "active"), quotaMeta(1200, 1000)))
	// 50% → no finding
	core.statsByTenant["okq"] = 500
	seedAnalyzeTenant(t, repo, "okq", oldEnough(), mergeMeta(subMeta("indie", "active"), quotaMeta(500, 1000)))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	if f, ok := hasFinding(findTenant(t, rep, "warn"), CodeQuotaPressure); !ok || f.Severity != dto.AnalysisSeverityWarn {
		t.Errorf("warn tenant quota_pressure = %+v (ok=%v), want warn", f, ok)
	}
	if f, ok := hasFinding(findTenant(t, rep, "crit"), CodeQuotaPressure); !ok || f.Severity != dto.AnalysisSeverityCritical {
		t.Errorf("crit tenant quota_pressure = %+v (ok=%v), want critical", f, ok)
	}
	if _, ok := hasFinding(findTenant(t, rep, "okq"), CodeQuotaPressure); ok {
		t.Errorf("50%% tenant wrongly flagged quota_pressure")
	}
}

func TestAnalyze_FleetCreatedAtNotReal(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	// 8 tenants stamped on the SAME calendar date → the time.Now()-on-load smell.
	sameDay := time.Date(2026, 6, 26, 12, 0, 0, 0, time.UTC)
	for i := 0; i < 8; i++ {
		id := fmt.Sprintf("t-%d", i)
		core.statsByTenant[id] = int64(i + 1)
		seedAnalyzeTenant(t, repo, id, sameDay.Add(time.Duration(i)*time.Millisecond), subMeta("free", ""))
	}

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	f, ok := hasFleetFinding(rep, CodeCreatedAtNotReal)
	if !ok {
		t.Fatalf("expected fleet finding %s, got %+v", CodeCreatedAtNotReal, rep.FleetFindings)
	}
	if f.Severity != dto.AnalysisSeverityCritical {
		t.Errorf("created_at_not_real severity = %q, want critical", f.Severity)
	}
	if f.AffectedCount != 8 {
		t.Errorf("created_at_not_real affected_count = %d, want 8", f.AffectedCount)
	}

	// 7 sharing a date is below the threshold → no fleet finding.
	repo2 := persistence.NewMemoryTenantRepository()
	core2 := newAnalyzeMockCore()
	for i := 0; i < 7; i++ {
		id := fmt.Sprintf("t-%d", i)
		core2.statsByTenant[id] = 1
		seedAnalyzeTenant(t, repo2, id, sameDay, subMeta("free", ""))
	}
	uc2 := NewAnalyzeTenantsUseCase(repo2, core2, nil, nil)
	rep2, err := uc2.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("uc2.Execute: %v", err)
	}
	if _, ok := hasFleetFinding(rep2, CodeCreatedAtNotReal); ok {
		t.Errorf("7 tenants sharing a date wrongly tripped created_at_not_real")
	}

	// Regression guard (false positive proven in prod 2026-06-26): a LEGITIMATE
	// same-day batch — e.g. an e2e run seeding 9 test tenants in one minute — that
	// is a MINORITY among many other real distinct dates must NOT be flagged. Here
	// 9 share one day but 14 more each have their own date (23 total, worst bucket
	// 9/23 ≈ 0.39 < 0.7 dominance, 15 distinct dates), so created_at IS real.
	repo3 := persistence.NewMemoryTenantRepository()
	core3 := newAnalyzeMockCore()
	batchDay := time.Date(2026, 4, 17, 18, 0, 0, 0, time.UTC)
	for i := 0; i < 9; i++ {
		id := fmt.Sprintf("batch-%d", i)
		core3.statsByTenant[id] = 1
		seedAnalyzeTenant(t, repo3, id, batchDay.Add(time.Duration(i)*time.Minute), subMeta("free", ""))
	}
	for i := 0; i < 14; i++ {
		id := fmt.Sprintf("real-%d", i)
		core3.statsByTenant[id] = 1
		// Each on its own distinct calendar date.
		seedAnalyzeTenant(t, repo3, id, time.Date(2026, 1, 1+i, 9, 0, 0, 0, time.UTC), subMeta("free", ""))
	}
	uc3 := NewAnalyzeTenantsUseCase(repo3, core3, nil, nil)
	rep3, err := uc3.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("uc3.Execute: %v", err)
	}
	if _, ok := hasFleetFinding(rep3, CodeCreatedAtNotReal); ok {
		t.Errorf("legit same-day batch (minority of many real dates) wrongly tripped created_at_not_real")
	}
}

func TestAnalyze_PerTenantDegradedDoesNotFailRun(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()

	// A "poison" tenant whose subscription metadata is a type the reused
	// extractors don't expect (a bare string under the nested subscription path),
	// alongside a healthy tenant. The run must complete and flag the poison tenant
	// as plan_shape_drift / degraded rather than 500-ing the whole request.
	core.statsByTenant["good"] = 10
	seedAnalyzeTenant(t, repo, "good", oldEnough(), subMeta("indie", "active"))

	core.statsByTenant["poison"] = 10
	seedAnalyzeTenant(t, repo, "poison", oldEnough(), map[string]interface{}{
		"plan":         12345,       // non-string flat plan → shape drift
		"subscription": "not-a-map", // unexpected subscription shape
	})

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute must not error on a weird tenant: %v", err)
	}
	if rep.Summary.TotalTenants != 2 {
		t.Fatalf("total_tenants = %d, want 2 (run completed)", rep.Summary.TotalTenants)
	}

	// The poison tenant is flagged plan_shape_drift; the run did not abort.
	poison := findTenant(t, rep, "poison")
	if _, ok := hasFinding(poison, CodePlanShapeDrift); !ok {
		t.Errorf("poison tenant not flagged plan_shape_drift: %+v", poison.Findings)
	}
	// The healthy tenant is still present and unflagged for plan issues.
	good := findTenant(t, rep, "good")
	if _, ok := hasFinding(good, CodePlanNotInCatalog); ok {
		t.Errorf("good tenant wrongly flagged plan_not_in_catalog")
	}
}

func TestAnalyze_CategoryFilter(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	// One tenant with BOTH a data_integrity issue (cap artifact) and a litter
	// issue (is_demo). Filtering to litter must drop the data_integrity finding.
	core.statsByTenant["multi"] = backfillCountCap
	core.demoTenants["multi"] = true
	seedAnalyzeTenant(t, repo, "multi", oldEnough(), subMeta("free", ""))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{Category: dto.AnalysisCategoryLitter})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	tn := findTenant(t, rep, "multi")
	if _, ok := hasFinding(tn, CodeDemoLitter); !ok {
		t.Errorf("litter-only filter should keep demo_litter: %+v", tn.Findings)
	}
	if _, ok := hasFinding(tn, CodeCountCapArtifact); ok {
		t.Errorf("litter-only filter should drop data_integrity cap artifact: %+v", tn.Findings)
	}
}

func TestAnalyze_TenantIDFilter(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["a"] = 10
	core.statsByTenant["b"] = 10
	seedAnalyzeTenant(t, repo, "a", oldEnough(), subMeta("indie", "active"))
	seedAnalyzeTenant(t, repo, "b", oldEnough(), subMeta("indie", "active"))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{TenantID: "a"})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}
	if len(rep.Tenants) != 1 || rep.Tenants[0].ID != "a" {
		t.Fatalf("tenant_id filter returned %d tenants, want only 'a'", len(rep.Tenants))
	}
}

func TestAnalyze_DegradesWhenCoreListErrors(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.listErr = true // demo-flag read fails
	core.statsByTenant["x"] = 10
	seedAnalyzeTenant(t, repo, "x", oldEnough(), subMeta("indie", "active"))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("a Core list error must not fail the run: %v", err)
	}
	if rep.Summary.TotalTenants != 1 {
		t.Fatalf("total_tenants = %d, want 1", rep.Summary.TotalTenants)
	}
	// No demo findings (degraded to empty flag set), but the run succeeded.
	if _, ok := hasFinding(findTenant(t, rep, "x"), CodeDemoLitter); ok {
		t.Errorf("demo finding present despite Core list error")
	}
}

func TestAnalyze_ReportShapeMatchesContract(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	core := newAnalyzeMockCore()
	core.statsByTenant["t1"] = 10
	seedAnalyzeTenant(t, repo, "t1", oldEnough(), subMeta("indie", "active"))

	uc := NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	rep, err := uc.Execute(context.Background(), AnalyzeRequest{})
	if err != nil {
		t.Fatalf("Execute: %v", err)
	}

	// generated_at is RFC3339.
	if _, perr := time.Parse(time.RFC3339, rep.GeneratedAt); perr != nil {
		t.Errorf("generated_at %q not RFC3339: %v", rep.GeneratedAt, perr)
	}
	// summary maps carry all four categories + three severities (never nil).
	for _, cat := range []string{
		dto.AnalysisCategoryDataIntegrity, dto.AnalysisCategoryPlanBilling,
		dto.AnalysisCategoryLitter, dto.AnalysisCategoryUsageHealth,
	} {
		if _, ok := rep.Summary.ByCategory[cat]; !ok {
			t.Errorf("summary.by_category missing %q", cat)
		}
	}
	for _, sev := range []string{dto.AnalysisSeverityCritical, dto.AnalysisSeverityWarn, dto.AnalysisSeverityInfo} {
		if _, ok := rep.Summary.BySeverity[sev]; !ok {
			t.Errorf("summary.by_severity missing %q", sev)
		}
	}
	// fleet_findings + tenants are non-nil slices.
	if rep.FleetFindings == nil {
		t.Errorf("fleet_findings is nil, want [] for clean JSON")
	}
	if rep.Tenants == nil {
		t.Errorf("tenants is nil")
	}
	// Every suggested_action target on every finding is an /api/v1/... route or a
	// task command — never empty.
	checkAction := func(f dto.AnalysisFinding) {
		if f.SuggestedAction == nil {
			return // optional
		}
		if f.SuggestedAction.Target == "" {
			t.Errorf("finding %s has empty suggested_action.target", f.Code)
		}
		if f.SuggestedAction.Kind == dto.AnalysisActionKindLink && !strings.HasPrefix(f.SuggestedAction.Target, "/api/v1/") {
			t.Errorf("finding %s link target %q is not an /api/v1/ route", f.Code, f.SuggestedAction.Target)
		}
	}
	for _, f := range rep.FleetFindings {
		checkAction(f)
	}
	for _, tn := range rep.Tenants {
		for _, f := range tn.Findings {
			checkAction(f)
		}
	}
}

// mergeMeta shallow-merges two metadata maps (right wins) for test setup.
func mergeMeta(a, b map[string]interface{}) map[string]interface{} {
	out := map[string]interface{}{}
	for k, v := range a {
		out[k] = v
	}
	for k, v := range b {
		out[k] = v
	}
	return out
}
