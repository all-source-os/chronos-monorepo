package persistence

import (
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Core emits an `active` bool, not a `status` string. The repo used to read only
// the (never-sent) status field, so every tenant defaulted to active and the
// admin could never reflect a suspended/archived tenant — the bulk Archive/Suspend
// "no-op" (2026-06-29). This pins the active-flag + lifecycle-hint derivation.
func TestCoreTenantToEntity_StatusFromActiveFlag(t *testing.T) {
	cases := []struct {
		name string
		resp clients.TenantResponse
		want entities.TenantStatus
	}{
		{
			name: "active flag true -> active",
			resp: clients.TenantResponse{ID: "a", Active: true},
			want: entities.TenantStatusActive,
		},
		{
			name: "active flag false, no hint -> suspended",
			resp: clients.TenantResponse{ID: "b", Active: false},
			want: entities.TenantStatusSuspended,
		},
		{
			name: "active false + lifecycle_status=archived -> archived",
			resp: clients.TenantResponse{ID: "c", Active: false, Metadata: map[string]any{"lifecycle_status": "archived"}},
			want: entities.TenantStatusArchived,
		},
		{
			name: "explicit Core status wins (forward-compat)",
			resp: clients.TenantResponse{ID: "d", Active: false, Status: "deleted"},
			want: entities.TenantStatusDeleted,
		},
		{
			name: "active true + stale archived hint -> still active (active flag wins)",
			resp: clients.TenantResponse{ID: "e", Active: true, Metadata: map[string]any{"lifecycle_status": "archived"}},
			want: entities.TenantStatusActive,
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got := coreTenantToEntity(&tc.resp)
			if got.Status != tc.want {
				t.Errorf("status = %q, want %q", got.Status, tc.want)
			}
		})
	}
}
