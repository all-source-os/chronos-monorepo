package x402

import (
	"fmt"
	"strings"
)

// IsSVMNetwork returns true if the CAIP-2 network ID is a Solana chain.
func IsSVMNetwork(network string) bool {
	return strings.HasPrefix(network, "solana:")
}

// SVMClusterID extracts the cluster genesis hash from a CAIP-2 Solana network identifier.
func SVMClusterID(network string) (string, error) {
	if !IsSVMNetwork(network) {
		return "", fmt.Errorf("not a Solana network: %s", network)
	}
	parts := strings.SplitN(network, ":", 2)
	if len(parts) != 2 || parts[1] == "" {
		return "", fmt.Errorf("invalid Solana CAIP-2 format: %s", network)
	}
	return parts[1], nil
}

// USDCSolanaAddress returns the USDC SPL token address for a given Solana network.
func USDCSolanaAddress(network string) (string, error) {
	switch network {
	case NetworkSolanaMainnet:
		return USDCSolanaMainnet, nil
	default:
		return "", fmt.Errorf("no USDC address configured for Solana network %s", network)
	}
}

// SPLTransferVerifier verifies Solana SPL TransferChecked authorization signatures.
// Implementation requires a Solana Go SDK (added in US-004).
type SPLTransferVerifier interface {
	Verify(transaction string, network string) error
}
