package x402

import (
	"encoding/json"
	"fmt"
	"os"
)

// PricingConfig holds per-endpoint x402 pricing configuration.
type PricingConfig struct {
	// Enabled is the master switch for x402 payments. Default: false.
	Enabled bool `json:"enabled"`

	// RecipientAddress is the wallet address that receives payments.
	RecipientAddress string `json:"recipientAddress"`

	// Routes maps "METHOD /path" to pricing for that endpoint.
	Routes map[string]RoutePricing `json:"routes"`
}

// RoutePricing defines the price and accepted networks for a single route.
type RoutePricing struct {
	// Amount in the token's smallest unit (USDC: 6 decimals, so "100" = $0.0001).
	Amount string `json:"amount"`

	// Asset is the token contract address (default: USDC).
	Asset string `json:"asset,omitempty"`

	// Networks is the list of accepted CAIP-2 network IDs.
	Networks []string `json:"networks"`

	// Description is a human-readable description shown in the 402 response.
	Description string `json:"description,omitempty"`
}

// LoadPricingConfig loads pricing from the X402_PRICING_CONFIG env var (file path)
// or returns a disabled config if not set.
func LoadPricingConfig() (*PricingConfig, error) {
	configPath := os.Getenv("X402_PRICING_CONFIG")
	if configPath == "" {
		return &PricingConfig{Enabled: false}, nil
	}

	// configPath comes from X402_PRICING_CONFIG env var, set by the operator
	// at deploy time — not user input. Path traversal is not a threat here.
	data, err := os.ReadFile(configPath) //nolint:gosec // G304/G703
	if err != nil {
		return nil, fmt.Errorf("read pricing config %s: %w", configPath, err)
	}

	var config PricingConfig
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("parse pricing config: %w", err)
	}

	// Override enabled flag from env var if set
	if os.Getenv("X402_ENABLED") == "true" {
		config.Enabled = true
	} else if os.Getenv("X402_ENABLED") == "false" {
		config.Enabled = false
	}

	// Override recipient from env var if set
	if recipient := os.Getenv("X402_RECIPIENT_ADDRESS"); recipient != "" {
		config.RecipientAddress = recipient
	}

	return &config, nil
}

// RequirementsForRoute builds a PaymentRequired response for the given route key.
// Returns nil if the route has no pricing configured.
func (pc *PricingConfig) RequirementsForRoute(routeKey string) *PaymentRequired {
	pricing, ok := pc.Routes[routeKey]
	if !ok {
		return nil
	}

	asset := pricing.Asset
	if asset == "" {
		asset = USDCBaseMainnet // default
	}

	var accepts []PaymentRequirements
	for _, network := range pricing.Networks {
		// Override asset per-network if needed
		routeAsset := asset
		if IsSVMNetwork(network) {
			routeAsset = USDCSolanaMainnet
		}

		accepts = append(accepts, PaymentRequirements{
			Scheme:  SchemeExact,
			Network: network,
			Asset:   routeAsset,
			Amount:  pricing.Amount,
			PayTo:   pc.RecipientAddress,
		})
	}

	return &PaymentRequired{
		X402Version: Version,
		Accepts:     accepts,
		Resource: &ResourceInfo{
			URL:         routeKey,
			Description: pricing.Description,
		},
	}
}

// RouteKey builds a route key from an HTTP method and path.
func RouteKey(method, path string) string {
	return method + " " + path
}
