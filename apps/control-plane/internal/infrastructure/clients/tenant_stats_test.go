package clients

import (
	"encoding/json"
	"testing"
)

// realCoreStatsBody mirrors what apps/core build_tenant_stats actually returns:
// the event total lives in the nested "usage" block, NOT a flat "event_count".
const realCoreStatsBody = `{
  "tenant_id": "t-real",
  "name": "Real Corp",
  "active": true,
  "is_demo": false,
  "usage": {
    "events_today": 120,
    "total_events": 4200,
    "storage_bytes": 8192,
    "queries_this_hour": 3,
    "active_api_keys": 1,
    "active_projections": 0,
    "active_pipelines": 2,
    "last_daily_reset": "2026-06-25T00:00:00Z",
    "last_hourly_reset": "2026-06-25T00:00:00Z"
  },
  "quotas": {"max_events_per_day": 100000},
  "utilization": {"events_today": {"used": 120, "limit": 100000, "percentage": 0.12}},
  "created_at": "2026-06-01T00:00:00Z",
  "updated_at": "2026-06-25T00:00:00Z"
}`

func TestTenantStats_UnmarshalRealCoreShape(t *testing.T) {
	var s TenantStatsResponse
	if err := json.Unmarshal([]byte(realCoreStatsBody), &s); err != nil {
		t.Fatalf("unmarshal real core stats: %v", err)
	}
	if s.TenantID != "t-real" {
		t.Errorf("tenant_id: got %q", s.TenantID)
	}
	// Prefer the lifetime total_events for the Events column.
	if s.EventCount != 4200 {
		t.Errorf("event_count: want 4200 from usage.total_events, got %d", s.EventCount)
	}
	if s.StorageUsed != 8192 {
		t.Errorf("storage_used: want 8192 from usage.storage_bytes, got %d", s.StorageUsed)
	}
	if s.StreamCount != 2 {
		t.Errorf("stream_count: want 2 from usage.active_pipelines, got %d", s.StreamCount)
	}
}

func TestTenantStats_UnmarshalFlatShapeWins(t *testing.T) {
	// A flat event_count (test fixture / future emitter) takes precedence.
	body := `{"tenant_id":"t-flat","event_count":99,"storage_used":7,"stream_count":1}`
	var s TenantStatsResponse
	if err := json.Unmarshal([]byte(body), &s); err != nil {
		t.Fatalf("unmarshal flat stats: %v", err)
	}
	if s.EventCount != 99 || s.StorageUsed != 7 || s.StreamCount != 1 {
		t.Errorf("flat fields not preserved: %+v", s)
	}
}

func TestTenantStats_UnmarshalFallsBackToEventsToday(t *testing.T) {
	// No total_events → fall back to events_today.
	body := `{"tenant_id":"t-today","usage":{"events_today":55,"total_events":0,"storage_bytes":0}}`
	var s TenantStatsResponse
	if err := json.Unmarshal([]byte(body), &s); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if s.EventCount != 55 {
		t.Errorf("event_count: want 55 from usage.events_today, got %d", s.EventCount)
	}
}

func TestTenantStats_UnmarshalZeroUsageStaysZero(t *testing.T) {
	// A zero-usage tenant must decode to a clean 0, never an error.
	body := `{"tenant_id":"t-zero","usage":{"events_today":0,"total_events":0,"storage_bytes":0}}`
	var s TenantStatsResponse
	if err := json.Unmarshal([]byte(body), &s); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if s.EventCount != 0 || s.StorageUsed != 0 {
		t.Errorf("zero-usage should stay zero, got %+v", s)
	}
}
