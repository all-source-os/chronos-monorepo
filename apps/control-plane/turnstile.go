package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"
)

const turnstileSiteverifyURL = "https://challenges.cloudflare.com/turnstile/v0/siteverify"

// TurnstileVerifier validates Cloudflare Turnstile tokens. If the secret key
// is not configured (TURNSTILE_SECRET_KEY unset), verification is skipped —
// this allows the onboard endpoint to work for CLI/agent callers that don't
// render a browser widget.
type TurnstileVerifier struct {
	secretKey string
	client    *http.Client
}

// NewTurnstileVerifier creates a verifier from the TURNSTILE_SECRET_KEY env var.
// Returns nil if the env var is unset (Turnstile is optional).
func NewTurnstileVerifier() *TurnstileVerifier {
	secret := os.Getenv("TURNSTILE_SECRET_KEY")
	if secret == "" {
		return nil
	}
	return &TurnstileVerifier{
		secretKey: secret,
		client:    &http.Client{Timeout: 5 * time.Second},
	}
}

// Verify checks a Turnstile token with Cloudflare's siteverify API.
// Returns nil if the token is valid, an error otherwise.
func (tv *TurnstileVerifier) Verify(ctx context.Context, token, remoteIP string) error {
	if tv == nil || tv.secretKey == "" {
		return nil // Turnstile not configured — skip verification
	}

	form := url.Values{
		"secret":   {tv.secretKey},
		"response": {token},
	}
	if remoteIP != "" {
		form.Set("remoteip", remoteIP)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, turnstileSiteverifyURL, strings.NewReader(form.Encode()))
	if err != nil {
		return fmt.Errorf("build turnstile request: %w", err)
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := tv.client.Do(req)
	if err != nil {
		return fmt.Errorf("turnstile siteverify failed: %w", err)
	}
	defer func() { _ = resp.Body.Close() }() //nolint:errcheck // close-on-defer, non-actionable

	var result struct {
		Success    bool     `json:"success"`
		ErrorCodes []string `json:"error-codes"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return fmt.Errorf("decode turnstile response: %w", err)
	}

	if !result.Success {
		return fmt.Errorf("turnstile verification failed: %v", result.ErrorCodes)
	}
	return nil
}
