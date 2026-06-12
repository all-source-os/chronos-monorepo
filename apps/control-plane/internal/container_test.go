package internal

import (
	"context"
	"errors"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// walletCoreClient is a minimal CoreClient stub for CoreWalletLookup tests.
type walletCoreClient struct {
	clients.CoreClient
	entry *clients.ConfigEntryResponse
	err   error
}

func (m *walletCoreClient) GetConfig(_ context.Context, _ string) (*clients.ConfigEntryResponse, error) {
	return m.entry, m.err
}

func TestCoreWalletLookup_NilClient_ReturnsEmpty(t *testing.T) {
	lookup := &CoreWalletLookup{coreClient: nil}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error, got %v", err)
	}
	if id != "" || addr != "" {
		t.Errorf("want empty strings, got id=%q addr=%q", id, addr)
	}
}

func TestCoreWalletLookup_CoreError_ReturnsEmpty(t *testing.T) {
	mock := &walletCoreClient{err: errors.New("core unavailable")}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error (no-wallet is not an error), got %v", err)
	}
	if id != "" || addr != "" {
		t.Errorf("want empty strings on error, got id=%q addr=%q", id, addr)
	}
}

func TestCoreWalletLookup_NilEntry_ReturnsEmpty(t *testing.T) {
	mock := &walletCoreClient{entry: nil}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error, got %v", err)
	}
	if id != "" || addr != "" {
		t.Errorf("want empty strings for nil entry, got id=%q addr=%q", id, addr)
	}
}

func TestCoreWalletLookup_NonStringValue_ReturnsEmpty(t *testing.T) {
	mock := &walletCoreClient{entry: &clients.ConfigEntryResponse{Key: "k", Value: 42}}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error, got %v", err)
	}
	if id != "" || addr != "" {
		t.Errorf("want empty strings for non-string value, got id=%q addr=%q", id, addr)
	}
}

func TestCoreWalletLookup_EmptyStringValue_ReturnsEmpty(t *testing.T) {
	mock := &walletCoreClient{entry: &clients.ConfigEntryResponse{Key: "k", Value: ""}}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error, got %v", err)
	}
	if id != "" || addr != "" {
		t.Errorf("want empty strings for empty value, got id=%q addr=%q", id, addr)
	}
}

func TestCoreWalletLookup_ValidEntry_ReturnsParsed(t *testing.T) {
	mock := &walletCoreClient{
		entry: &clients.ConfigEntryResponse{
			Key:   "agent:tenant-1:cdp_wallet",
			Value: "wallet-abc|0xDeadBeef",
		},
	}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Fatalf("want nil error, got %v", err)
	}
	if id != "wallet-abc" {
		t.Errorf("walletID: want wallet-abc, got %q", id)
	}
	if addr != "0xDeadBeef" {
		t.Errorf("address: want 0xDeadBeef, got %q", addr)
	}
}

func TestCoreWalletLookup_NoSeparator_ReturnsIDEmpty(t *testing.T) {
	// Edge case: value with no '|' → walletID=whole string, address=""
	mock := &walletCoreClient{
		entry: &clients.ConfigEntryResponse{Key: "k", Value: "wallet-only"},
	}
	lookup := &CoreWalletLookup{coreClient: mock}
	id, addr, err := lookup.GetAgentWallet(context.Background(), "tenant-1")
	if err != nil {
		t.Errorf("want nil error, got %v", err)
	}
	if id != "wallet-only" {
		t.Errorf("walletID: want wallet-only, got %q", id)
	}
	if addr != "" {
		t.Errorf("address: want empty, got %q", addr)
	}
}

func TestSplitTwo(t *testing.T) {
	tests := []struct {
		input     string
		sep       byte
		wantFirst string
		wantRest  string
	}{
		{"a|b", '|', "a", "b"},
		{"wallet-id|0xAddress", '|', "wallet-id", "0xAddress"},
		{"no-sep", '|', "no-sep", ""},
		{"", '|', "", ""},
		{"first|second|third", '|', "first", "second|third"},
	}
	for _, tt := range tests {
		got := splitTwo(tt.input, tt.sep)
		if got[0] != tt.wantFirst || got[1] != tt.wantRest {
			t.Errorf("splitTwo(%q, %q) = [%q, %q], want [%q, %q]",
				tt.input, string(tt.sep), got[0], got[1], tt.wantFirst, tt.wantRest)
		}
	}
}

func TestBuildVariantTierMap_StripsPeriodSuffix(t *testing.T) {
	t.Setenv("LEMON_SQUEEZY_VARIANT_MAP", `{"studio:monthly":"111","studio:annual":"222","indie":"333"}`)
	m := buildVariantTierMap()

	// A variant ID must resolve to the BARE tier, not "studio:monthly".
	if m["111"] != "studio" {
		t.Errorf(`m["111"] = %q, want "studio"`, m["111"])
	}
	if m["222"] != "studio" {
		t.Errorf(`m["222"] = %q, want "studio"`, m["222"])
	}
	// Legacy bare-key entries pass through unchanged.
	if m["333"] != "indie" {
		t.Errorf(`m["333"] = %q, want "indie"`, m["333"])
	}
	// variant_id is the sole authority — no tier-name self-map entries.
	if _, ok := m["studio"]; ok {
		t.Errorf("unexpected tier-name self-map entry: %v", m)
	}
}

func TestBuildVariantTierMap_Empty(t *testing.T) {
	t.Setenv("LEMON_SQUEEZY_VARIANT_MAP", "")
	if buildVariantTierMap() != nil {
		t.Error("empty env should yield nil")
	}
}
