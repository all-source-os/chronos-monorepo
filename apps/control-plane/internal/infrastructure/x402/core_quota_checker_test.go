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
