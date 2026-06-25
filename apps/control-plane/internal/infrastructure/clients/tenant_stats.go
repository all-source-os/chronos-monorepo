package clients

import "encoding/json"

// UnmarshalJSON tolerantly decodes Core's GET /api/v1/tenants/{id}/stats body
// into the flat TenantStatsResponse the Control Plane consumes.
//
// WHY a custom decoder: Core's build_tenant_stats (apps/core/.../tenant_api.rs
// build_tenant_stats) does NOT emit a flat "event_count". The real per-tenant
// event total lives in the nested "usage" block
// (usage.total_events / usage.events_today), and storage lives under
// usage.storage_bytes. A plain struct decode against the documented flat fields
// therefore deserialized EventCount/StorageUsed to 0 for every live call — which
// is exactly why the admin tenant list/detail showed 0 events. This decoder
// reads the real shape while staying back-compatible: a flat "event_count"
// (used by unit-test fixtures / any future flat emitter) still wins when present
// and non-zero, otherwise we fall back to the nested usage totals.
func (s *TenantStatsResponse) UnmarshalJSON(data []byte) error {
	// raw mirrors the flat fields plus the nested usage block Core actually sends.
	var raw struct {
		TenantID    string `json:"tenant_id"`
		EventCount  int64  `json:"event_count"`
		StorageUsed int64  `json:"storage_used"`
		StreamCount int64  `json:"stream_count"`
		Usage       *struct {
			EventsToday   int64 `json:"events_today"`
			TotalEvents   int64 `json:"total_events"`
			StorageBytes  int64 `json:"storage_bytes"`
			ActiveStreams int64 `json:"active_pipelines"`
		} `json:"usage"`
	}
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}

	s.TenantID = raw.TenantID
	s.EventCount = raw.EventCount
	s.StorageUsed = raw.StorageUsed
	s.StreamCount = raw.StreamCount

	// Fall back to the nested usage totals Core actually emits when the flat
	// fields are absent/zero. total_events is the lifetime count (the number the
	// admin Events column wants); events_today is the daily-windowed counter the
	// quota uses — prefer the lifetime total, fall back to today's.
	if raw.Usage != nil {
		if s.EventCount == 0 {
			if raw.Usage.TotalEvents > 0 {
				s.EventCount = raw.Usage.TotalEvents
			} else {
				s.EventCount = raw.Usage.EventsToday
			}
		}
		if s.StorageUsed == 0 {
			s.StorageUsed = raw.Usage.StorageBytes
		}
		if s.StreamCount == 0 {
			s.StreamCount = raw.Usage.ActiveStreams
		}
	}
	return nil
}
