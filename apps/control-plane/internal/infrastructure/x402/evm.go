package x402

import (
	"fmt"
	"strings"
)

// IsEVMNetwork returns true if the CAIP-2 network ID is an EVM chain.
func IsEVMNetwork(network string) bool {
	return strings.HasPrefix(network, "eip155:")
}

// EVMChainID extracts the numeric chain ID from a CAIP-2 EVM network identifier.
// e.g., "eip155:8453" → 8453.
func EVMChainID(network string) (int64, error) {
	if !IsEVMNetwork(network) {
		return 0, fmt.Errorf("not an EVM network: %s", network)
	}
	var chainID int64
	_, err := fmt.Sscanf(network, "eip155:%d", &chainID)
	if err != nil {
		return 0, fmt.Errorf("parse chain ID from %s: %w", network, err)
	}
	return chainID, nil
}

// USDCAddressForNetwork returns the USDC contract address for a given EVM network.
func USDCAddressForNetwork(network string) (string, error) {
	switch network {
	case NetworkBaseMainnet:
		return USDCBaseMainnet, nil
	case NetworkBaseSepolia:
		return USDCBaseSepolia, nil
	default:
		return "", fmt.Errorf("no USDC address configured for network %s", network)
	}
}

// EIP3009Verifier verifies EIP-3009 transferWithAuthorization signatures.
// Implementation requires go-ethereum (added in US-004).
type EIP3009Verifier interface {
	Verify(auth *TransferAuthorization, signature string, network string) error
}
