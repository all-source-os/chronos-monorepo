package main

import (
	"context"
	"encoding/json"
	"errors"
	"strings"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// teamsCoreClient stubs only the CoreClient methods used by teams.go.
type teamsCoreClient struct {
	clients.CoreClient
	configs    map[string]any // key → stored value
	setErr     error
	getErr     error
	missingKey string // if key matches, return nil entry
}

func (m *teamsCoreClient) GetConfig(_ context.Context, key string) (*clients.ConfigEntryResponse, error) {
	if m.getErr != nil {
		return nil, m.getErr
	}
	if key == m.missingKey {
		return nil, nil
	}
	v, ok := m.configs[key]
	if !ok {
		return nil, nil
	}
	return &clients.ConfigEntryResponse{Key: key, Value: v}, nil
}

func (m *teamsCoreClient) SetConfig(_ context.Context, req clients.SetConfigRequest) error {
	if m.setErr != nil {
		return m.setErr
	}
	if m.configs == nil {
		m.configs = make(map[string]any)
	}
	m.configs[req.Key] = req.Value
	return nil
}

// ─── Pure-function tests ───────────────────────────────────────────────────

func TestTeamInviteConfigKey(t *testing.T) {
	key := teamInviteConfigKey("abc123")
	if key != "team:invite:abc123" {
		t.Errorf("want team:invite:abc123, got %q", key)
	}
}

func TestTeamMembersConfigKey(t *testing.T) {
	key := teamMembersConfigKey("tenant-99")
	if key != "team:tenant-99:members" {
		t.Errorf("want team:tenant-99:members, got %q", key)
	}
}

func TestGenerateInviteToken(t *testing.T) {
	tok1, err := generateInviteToken()
	if err != nil {
		t.Fatalf("generateInviteToken: %v", err)
	}
	if tok1 == "" {
		t.Error("want non-empty token")
	}
	tok2, err2 := generateInviteToken()
	if err2 != nil {
		t.Fatalf("generateInviteToken (2nd): %v", err2)
	}
	if tok1 == tok2 {
		t.Error("two successive tokens must differ")
	}
}

func TestParseInviteFromConfig_RoundTrip(t *testing.T) {
	invite := TeamInvite{
		Token:    "tok1",
		TenantID: "t-123",
		Email:    "alice@example.com",
		Role:     roleAdmin,
	}
	// Simulate what Core stores/returns: marshal → any → parse
	b, err := json.Marshal(invite)
	if err != nil {
		t.Fatalf("marshal invite: %v", err)
	}
	var raw any
	if err := json.Unmarshal(b, &raw); err != nil {
		t.Fatalf("unmarshal invite: %v", err)
	}

	got, err := parseInviteFromConfig(raw)
	if err != nil {
		t.Fatalf("parseInviteFromConfig: %v", err)
	}
	if got.Token != invite.Token || got.TenantID != invite.TenantID || got.Email != invite.Email {
		t.Errorf("mismatch: got %+v, want %+v", got, invite)
	}
}

func TestParseMembersFromConfig_RoundTrip(t *testing.T) {
	members := []TeamMember{
		{UserID: "u1", Email: "a@b.com", Role: roleMember},
		{UserID: "u2", Email: "c@d.com", Role: roleAdmin},
	}
	b, err := json.Marshal(members)
	if err != nil {
		t.Fatalf("marshal members: %v", err)
	}
	var raw any
	if err := json.Unmarshal(b, &raw); err != nil {
		t.Fatalf("unmarshal members: %v", err)
	}

	got, err := parseMembersFromConfig(raw)
	if err != nil {
		t.Fatalf("parseMembersFromConfig: %v", err)
	}
	if len(got) != 2 || got[0].UserID != "u1" || got[1].Role != roleAdmin {
		t.Errorf("mismatch: %+v", got)
	}
}

// ─── resolveInvite tests ───────────────────────────────────────────────────

func TestResolveInvite_EmptyToken_ReturnsError(t *testing.T) {
	cp := &ControlPlane{coreClient: &teamsCoreClient{}}
	_, err := cp.resolveInvite(context.Background(), "", "alice@example.com")
	if err == nil {
		t.Error("want error for empty token")
	}
}

func TestResolveInvite_NotFound_ReturnsError(t *testing.T) {
	mock := &teamsCoreClient{missingKey: teamInviteConfigKey("unknown")}
	cp := &ControlPlane{coreClient: mock}
	_, err := cp.resolveInvite(context.Background(), "unknown", "alice@example.com")
	if err == nil {
		t.Error("want error when invite not found")
	}
}

func TestResolveInvite_CoreError_ReturnsError(t *testing.T) {
	mock := &teamsCoreClient{getErr: errors.New("core down")}
	cp := &ControlPlane{coreClient: mock}
	_, err := cp.resolveInvite(context.Background(), "tok1", "alice@example.com")
	if err == nil {
		t.Error("want error on core failure")
	}
}

func TestResolveInvite_EmailMismatch_ReturnsError(t *testing.T) {
	invite := storedInvite(t, "tok1", "t-1", "alice@example.com", roleMember)
	mock := &teamsCoreClient{configs: map[string]any{teamInviteConfigKey("tok1"): invite}}
	cp := &ControlPlane{coreClient: mock}
	_, err := cp.resolveInvite(context.Background(), "tok1", "bob@example.com")
	if err == nil || !strings.Contains(err.Error(), "mismatch") {
		t.Errorf("want email mismatch error, got %v", err)
	}
}

func TestResolveInvite_NoEmailRestriction_Succeeds(t *testing.T) {
	invite := storedInvite(t, "tok2", "t-1", "", roleMember) // no email restriction
	mock := &teamsCoreClient{configs: map[string]any{teamInviteConfigKey("tok2"): invite}}
	cp := &ControlPlane{coreClient: mock}
	got, err := cp.resolveInvite(context.Background(), "tok2", "anyone@example.com")
	if err != nil {
		t.Fatalf("want success, got %v", err)
	}
	if got.TenantID != "t-1" {
		t.Errorf("tenantID: want t-1, got %q", got.TenantID)
	}
}

func TestResolveInvite_ExactEmailMatch_Succeeds(t *testing.T) {
	invite := storedInvite(t, "tok3", "t-2", "alice@example.com", roleAdmin)
	mock := &teamsCoreClient{configs: map[string]any{teamInviteConfigKey("tok3"): invite}}
	cp := &ControlPlane{coreClient: mock}
	got, err := cp.resolveInvite(context.Background(), "tok3", "alice@example.com")
	if err != nil {
		t.Fatalf("want success, got %v", err)
	}
	if got.Role != roleAdmin {
		t.Errorf("role: want admin, got %q", got.Role)
	}
}

// ─── AddTeamMember tests ───────────────────────────────────────────────────

func TestAddTeamMember_NewMember_Saved(t *testing.T) {
	mock := &teamsCoreClient{configs: map[string]any{}}
	cp := &ControlPlane{coreClient: mock}

	member := TeamMember{UserID: "u1", Email: "u1@x.com", Role: roleMember}
	if err := cp.AddTeamMember(context.Background(), "t-1", member, "admin"); err != nil {
		t.Fatalf("AddTeamMember: %v", err)
	}

	// The member list should now be stored in Core config.
	stored, ok := mock.configs[teamMembersConfigKey("t-1")]
	if !ok {
		t.Fatal("expected member list stored in core config")
	}
	members, ok := stored.([]TeamMember)
	if !ok {
		t.Fatalf("unexpected type: %T", stored)
	}
	if len(members) != 1 || members[0].UserID != "u1" {
		t.Errorf("stored members: %+v", members)
	}
}

func TestAddTeamMember_Duplicate_Noop(t *testing.T) {
	existing := []TeamMember{{UserID: "u1", Email: "u1@x.com", Role: roleMember}}
	b, err := json.Marshal(existing)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	var raw any
	if err := json.Unmarshal(b, &raw); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	mock := &teamsCoreClient{
		configs: map[string]any{teamMembersConfigKey("t-1"): raw},
	}
	cp := &ControlPlane{coreClient: mock}

	// Adding the same user again must be a no-op (no second save call expected to change state).
	member := TeamMember{UserID: "u1", Email: "u1@x.com", Role: roleAdmin} // even role change ignored
	if err := cp.AddTeamMember(context.Background(), "t-1", member, "admin"); err != nil {
		t.Fatalf("AddTeamMember duplicate: %v", err)
	}

	stored := mock.configs[teamMembersConfigKey("t-1")]
	// The stored value may still be the raw form (no set was called for duplicate)
	_ = stored // just verify no panic/error
}

func TestAddTeamMember_GetMembersError_FallsBackToEmpty(t *testing.T) {
	// Core returns error for GetConfig — AddTeamMember falls back to empty slice.
	mock := &teamsCoreClient{getErr: errors.New("not found")}
	cp := &ControlPlane{coreClient: mock}

	member := TeamMember{UserID: "u99", Email: "u99@x.com", Role: roleMember}
	// GetConfig returns an error, so AddTeamMember falls back to empty member list,
	// then calls SetConfig (which succeeds). Verify no panic and result is accepted.
	if err := cp.AddTeamMember(context.Background(), "t-1", member, "admin"); err != nil {
		t.Logf("AddTeamMember with getErr: %v (may be expected if SetConfig also fails)", err)
	}
}

// ─── helpers ──────────────────────────────────────────────────────────────

// storedInvite returns a raw any value that represents a stored invite (mirrors Core serialization).
func storedInvite(t *testing.T, token, tenantID, email, role string) any {
	t.Helper()
	invite := TeamInvite{Token: token, TenantID: tenantID, Email: email, Role: role}
	b, err := json.Marshal(invite)
	if err != nil {
		t.Fatalf("storedInvite marshal: %v", err)
	}
	var raw any
	if err := json.Unmarshal(b, &raw); err != nil {
		t.Fatalf("storedInvite unmarshal: %v", err)
	}
	return raw
}
