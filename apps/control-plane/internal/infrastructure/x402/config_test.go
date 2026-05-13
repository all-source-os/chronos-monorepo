package x402

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadPricingConfig_NoEnvVar(t *testing.T) {
	t.Setenv("X402_PRICING_CONFIG", "")
	t.Setenv("X402_ENABLED", "")

	config, err := LoadPricingConfig()
	if err != nil {
		t.Fatalf("LoadPricingConfig: %v", err)
	}
	if config.Enabled {
		t.Error("expected disabled when no config")
	}
}

func TestLoadPricingConfig_FromFile(t *testing.T) {
	configJSON := `{
		"enabled": true,
		"recipientAddress": "0xTestRecipient",
		"routes": {
			"POST /api/v1/events": {
				"amount": "100",
				"networks": ["eip155:8453", "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"],
				"description": "Event ingestion"
			},
			"GET /api/v1/events/query": {
				"amount": "1000",
				"networks": ["eip155:8453"],
				"description": "Event query"
			}
		}
	}`

	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "pricing.json")
	if err := os.WriteFile(configPath, []byte(configJSON), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	t.Setenv("X402_PRICING_CONFIG", configPath)
	t.Setenv("X402_ENABLED", "")

	config, err := LoadPricingConfig()
	if err != nil {
		t.Fatalf("LoadPricingConfig: %v", err)
	}
	if !config.Enabled {
		t.Error("expected enabled")
	}
	if config.RecipientAddress != "0xTestRecipient" {
		t.Errorf("recipient: want 0xTestRecipient, got %s", config.RecipientAddress)
	}
	if len(config.Routes) != 2 {
		t.Fatalf("routes: want 2, got %d", len(config.Routes))
	}

	eventsRoute := config.Routes["POST /api/v1/events"]
	if eventsRoute.Amount != "100" {
		t.Errorf("events amount: want 100, got %s", eventsRoute.Amount)
	}
	if len(eventsRoute.Networks) != 2 {
		t.Errorf("events networks: want 2, got %d", len(eventsRoute.Networks))
	}
}

func TestLoadPricingConfig_EnvOverrides(t *testing.T) {
	configJSON := `{
		"enabled": false,
		"recipientAddress": "0xOriginal",
		"routes": {}
	}`

	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "pricing.json")
	if err := os.WriteFile(configPath, []byte(configJSON), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	t.Setenv("X402_PRICING_CONFIG", configPath)
	t.Setenv("X402_ENABLED", "true")
	t.Setenv("X402_RECIPIENT_ADDRESS", "0xOverridden")

	config, err := LoadPricingConfig()
	if err != nil {
		t.Fatalf("LoadPricingConfig: %v", err)
	}
	if !config.Enabled {
		t.Error("expected enabled from env override")
	}
	if config.RecipientAddress != "0xOverridden" {
		t.Errorf("recipient: want 0xOverridden, got %s", config.RecipientAddress)
	}
}

func TestLoadPricingConfig_InvalidFile(t *testing.T) {
	t.Setenv("X402_PRICING_CONFIG", "/nonexistent/path.json")
	_, err := LoadPricingConfig()
	if err == nil {
		t.Fatal("expected error for nonexistent file")
	}
}

func TestLoadPricingConfig_InvalidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "bad.json")
	if err := os.WriteFile(configPath, []byte("not json"), 0o644); err != nil {
		t.Fatalf("write: %v", err)
	}

	t.Setenv("X402_PRICING_CONFIG", configPath)
	_, err := LoadPricingConfig()
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
}

func TestRequirementsForRoute(t *testing.T) {
	config := &PricingConfig{
		Enabled:          true,
		RecipientAddress: "0xRecipient",
		Routes: map[string]RoutePricing{
			"POST /api/v1/events": {
				Amount:      "100",
				Networks:    []string{NetworkBaseMainnet, NetworkSolanaMainnet},
				Description: "Event ingestion",
			},
		},
	}

	t.Run("known route", func(t *testing.T) {
		req := config.RequirementsForRoute("POST /api/v1/events")
		if req == nil {
			t.Fatal("expected non-nil requirements")
			return
		}
		if req.X402Version != Version {
			t.Errorf("version: want %d, got %d", Version, req.X402Version)
		}
		if len(req.Accepts) != 2 {
			t.Fatalf("accepts: want 2, got %d", len(req.Accepts))
		}

		// EVM offer
		evm := req.Accepts[0]
		if evm.Network != NetworkBaseMainnet {
			t.Errorf("evm network: want %s, got %s", NetworkBaseMainnet, evm.Network)
		}
		if evm.Asset != USDCBaseMainnet {
			t.Errorf("evm asset: want %s, got %s", USDCBaseMainnet, evm.Asset)
		}
		if evm.PayTo != "0xRecipient" {
			t.Errorf("evm payTo: want 0xRecipient, got %s", evm.PayTo)
		}

		// Solana offer gets Solana USDC address
		sol := req.Accepts[1]
		if sol.Asset != USDCSolanaMainnet {
			t.Errorf("sol asset: want %s, got %s", USDCSolanaMainnet, sol.Asset)
		}
	})

	t.Run("unknown route", func(t *testing.T) {
		req := config.RequirementsForRoute("DELETE /api/v1/events")
		if req != nil {
			t.Error("expected nil for unknown route")
		}
	})
}

func TestRouteKey(t *testing.T) {
	if key := RouteKey("POST", "/api/v1/events"); key != "POST /api/v1/events" {
		t.Errorf("want 'POST /api/v1/events', got %q", key)
	}
}
