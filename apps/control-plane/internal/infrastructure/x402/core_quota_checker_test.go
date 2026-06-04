package x402

import (
	"context"
	"errors"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// quotaTestCoreClient stubs GetTenant for quota checker tests.
type quotaTestCoreClient struct {
	clients.CoreClient
	tenant *clients.TenantResponse
	err    error
}

func (m *quotaTestCoreClient) GetTenant(_ context.Context, _ string) (*clients.TenantResponse, error) {
	return m.tenant, m.err
}

func tenantWithQuota(eventsQuota, eventsUsed, queriesQuota, queriesUsed float64) *clients.TenantResponse {
	return &clients.TenantResponse{
		ID: "tenant-1",
		Metadata: map[string]any{
			"quota": map[string]any{
				"events_quota":  eventsQuota,
				"events_used":   eventsUsed,
				"queries_quota": queriesQuota,
				"queries_used":  queriesUsed,
			},
		},
	}
}

func TestCoreQuotaChecker_EventsRoute_UnderQuota(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(10000, 500, 1000, 0)}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (under quota), got false")
	}
}

func TestCoreQuotaChecker_EventsRoute_AtQuota(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(10000, 10000, 1000, 0)}
	q := NewCoreQuotaChecker(mock)
	if q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want false (at quota), got true")
	}
}

func TestCoreQuotaChecker_EventsRoute_OverQuota(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(10000, 15000, 1000, 0)}
	q := NewCoreQuotaChecker(mock)
	if q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want false (over quota), got true")
	}
}

func TestCoreQuotaChecker_QueryRoute_UnderQuota(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(10000, 0, 1000, 100)}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "GET /api/v1/events/query") {
		t.Error("want true (under quota), got false")
	}
}

func TestCoreQuotaChecker_QueryRoute_Exceeded(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(10000, 0, 1000, 1001)}
	q := NewCoreQuotaChecker(mock)
	if q.HasQuota("tenant-1", "GET /api/v1/events/query") {
		t.Error("want false (exceeded), got true")
	}
}

func TestCoreQuotaChecker_UnlimitedQuota(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithQuota(-1, 99999, -1, 99999)}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (unlimited quota), got false")
	}
	if !q.HasQuota("tenant-1", "GET /api/v1/events/query") {
		t.Error("want true (unlimited quota), got false")
	}
}

func TestCoreQuotaChecker_TenantNotFound_ReturnsTrue(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: nil, err: errors.New("not found")}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("missing-tenant", "POST /api/v1/events") {
		t.Error("want true (safe default on error), got false")
	}
}

func TestCoreQuotaChecker_NoQuotaMetadata_ReturnsTrue(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: &clients.TenantResponse{
		ID:       "tenant-1",
		Metadata: map[string]any{"other": "data"},
	}}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (no quota metadata), got false")
	}
}

func TestCoreQuotaChecker_NilClient_ReturnsTrue(t *testing.T) {
	q := NewCoreQuotaChecker(nil)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (nil client), got false")
	}
}

func TestCoreQuotaChecker_NilTenant_ReturnsTrue(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: nil, err: nil}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (nil tenant response), got false")
	}
}

func tenantWithX402(allowance, used float64) *clients.TenantResponse {
	return &clients.TenantResponse{
		ID: "tenant-1",
		Metadata: map[string]any{
			"quotas": map[string]any{
				"x402_allowance": allowance,
				"x402_used":      used,
			},
		},
	}
}

func TestCoreQuotaChecker_X402Allowance_Basic(t *testing.T) {
	// unlimited (-1) → always true
	q := NewCoreQuotaChecker(&quotaTestCoreClient{tenant: tenantWithX402(-1, 9_999_999)})
	if !q.HasX402Allowance("tenant-1") {
		t.Error("unlimited allowance: want true")
	}
	// no allowance (0) → false
	q = NewCoreQuotaChecker(&quotaTestCoreClient{tenant: tenantWithX402(0, 0)})
	if q.HasX402Allowance("tenant-1") {
		t.Error("zero allowance: want false")
	}
	// within allowance → true; at/over → false
	q = NewCoreQuotaChecker(&quotaTestCoreClient{tenant: tenantWithX402(100, 99)})
	if !q.HasX402Allowance("tenant-1") {
		t.Error("under allowance: want true")
	}
	q = NewCoreQuotaChecker(&quotaTestCoreClient{tenant: tenantWithX402(100, 100)})
	if q.HasX402Allowance("tenant-1") {
		t.Error("at allowance: want false")
	}
}

func TestCoreQuotaChecker_X402Allowance_FailsClosedOnError(t *testing.T) {
	q := NewCoreQuotaChecker(&quotaTestCoreClient{tenant: nil, err: errors.New("core down")})
	if q.HasX402Allowance("tenant-1") {
		t.Error("error path must fail closed (false), got true")
	}
}

// The in-process counter must stop a tenant overshooting its included allowance
// between reconciler ticks: after `allowance` free calls are recorded, the gate
// denies further free calls even though Core's x402_used has not been updated.
func TestCoreQuotaChecker_X402Allowance_LocalCounterPreventsOvershoot(t *testing.T) {
	q := NewCoreQuotaChecker(&quotaTestCoreClient{tenant: tenantWithX402(2, 0)})

	if !q.HasX402Allowance("tenant-1") {
		t.Fatal("call 1: want true")
	}
	q.RecordAllowanceConsumed("tenant-1")
	if !q.HasX402Allowance("tenant-1") {
		t.Fatal("call 2: want true")
	}
	q.RecordAllowanceConsumed("tenant-1")
	// Core still reports used=0, but locally we've served 2 of 2 → deny.
	if q.HasX402Allowance("tenant-1") {
		t.Error("call 3: want false (allowance spent locally), got true — overshoot")
	}
}

// When the reconciler advances Core's x402_used, the local delta is dropped
// (rebaselined) so consumption is not double-counted.
func TestCoreQuotaChecker_X402Allowance_RebaselineOnCoreAdvance(t *testing.T) {
	mock := &quotaTestCoreClient{tenant: tenantWithX402(5, 0)}
	q := NewCoreQuotaChecker(mock)

	q.RecordAllowanceConsumed("tenant-1")
	q.RecordAllowanceConsumed("tenant-1")
	q.RecordAllowanceConsumed("tenant-1") // local=3, effective=3
	if !q.HasX402Allowance("tenant-1") {
		t.Fatal("effective 3/5: want true")
	}

	// Reconciler catches up: Core now reports used=3. Local delta must reset so
	// effective stays 3 (not 6) — no double count.
	mock.tenant = tenantWithX402(5, 3)
	if !q.HasX402Allowance("tenant-1") { // triggers rebaseline
		t.Fatal("after rebaseline effective 3/5: want true")
	}
	q.RecordAllowanceConsumed("tenant-1") // local=1, effective=4
	if !q.HasX402Allowance("tenant-1") {
		t.Fatal("effective 4/5: want true")
	}
	q.RecordAllowanceConsumed("tenant-1") // local=2, effective=5
	if q.HasX402Allowance("tenant-1") {
		t.Error("effective 5/5: want false")
	}
}

func TestCoreQuotaChecker_IntMetadataValues(t *testing.T) {
	// Metadata may arrive as int or int64 instead of float64 in some code paths.
	mock := &quotaTestCoreClient{tenant: &clients.TenantResponse{
		ID: "tenant-1",
		Metadata: map[string]any{
			"quota": map[string]any{
				"events_quota": int64(10000),
				"events_used":  int64(5000),
			},
		},
	}}
	q := NewCoreQuotaChecker(mock)
	if !q.HasQuota("tenant-1", "POST /api/v1/events") {
		t.Error("want true (int64 metadata values), got false")
	}
}
