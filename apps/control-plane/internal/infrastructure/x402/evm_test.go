package x402

import "testing"

func TestIsEVMNetwork(t *testing.T) {
	tests := []struct {
		network string
		want    bool
	}{
		{NetworkBaseMainnet, true},
		{NetworkBaseSepolia, true},
		{"eip155:1", true},
		{NetworkSolanaMainnet, false},
		{"bitcoin:mainnet", false},
		{"", false},
	}

	for _, tt := range tests {
		if got := IsEVMNetwork(tt.network); got != tt.want {
			t.Errorf("IsEVMNetwork(%q) = %v, want %v", tt.network, got, tt.want)
		}
	}
}

func TestEVMChainID(t *testing.T) {
	tests := []struct {
		network string
		want    int64
		wantErr bool
	}{
		{NetworkBaseMainnet, 8453, false},
		{NetworkBaseSepolia, 84532, false},
		{"eip155:1", 1, false},
		{NetworkSolanaMainnet, 0, true},
		{"invalid", 0, true},
	}

	for _, tt := range tests {
		got, err := EVMChainID(tt.network)
		if (err != nil) != tt.wantErr {
			t.Errorf("EVMChainID(%q) error = %v, wantErr %v", tt.network, err, tt.wantErr)
			continue
		}
		if got != tt.want {
			t.Errorf("EVMChainID(%q) = %d, want %d", tt.network, got, tt.want)
		}
	}
}

func TestUSDCAddressForNetwork(t *testing.T) {
	addr, err := USDCAddressForNetwork(NetworkBaseMainnet)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if addr != USDCBaseMainnet {
		t.Errorf("want %s, got %s", USDCBaseMainnet, addr)
	}

	_, err = USDCAddressForNetwork("eip155:999")
	if err == nil {
		t.Error("expected error for unknown network")
	}
}

func TestIsSVMNetwork(t *testing.T) {
	tests := []struct {
		network string
		want    bool
	}{
		{NetworkSolanaMainnet, true},
		{NetworkSolanaDevnet, true},
		{NetworkBaseMainnet, false},
		{"", false},
	}

	for _, tt := range tests {
		if got := IsSVMNetwork(tt.network); got != tt.want {
			t.Errorf("IsSVMNetwork(%q) = %v, want %v", tt.network, got, tt.want)
		}
	}
}

func TestSVMClusterID(t *testing.T) {
	id, err := SVMClusterID(NetworkSolanaMainnet)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if id != "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp" {
		t.Errorf("unexpected cluster ID: %s", id)
	}

	_, err = SVMClusterID(NetworkBaseMainnet)
	if err == nil {
		t.Error("expected error for EVM network")
	}
}

func TestUSDCSolanaAddress(t *testing.T) {
	addr, err := USDCSolanaAddress(NetworkSolanaMainnet)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if addr != USDCSolanaMainnet {
		t.Errorf("want %s, got %s", USDCSolanaMainnet, addr)
	}

	_, err = USDCSolanaAddress(NetworkSolanaDevnet)
	if err == nil {
		t.Error("expected error for devnet (not configured)")
	}
}
