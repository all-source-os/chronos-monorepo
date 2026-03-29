package x402

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// CoinbaseX402FacilitatorURL is the hosted facilitator endpoint provided by Coinbase.
const CoinbaseX402FacilitatorURL = "https://x402.org/facilitator"

// PaymentFacilitator is the interface used by middleware and handlers.
// Both the local self-hosted Facilitator and RemoteFacilitator implement it.
type PaymentFacilitator interface {
	// Verify checks that a payment is valid without settling it.
	Verify(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*VerifyResponse, error)
	// Settle verifies and submits the payment on-chain, returning the settlement receipt.
	Settle(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*SettleResponse, error)
}

// RemoteFacilitator delegates verify and settle to an external facilitator endpoint
// (e.g., Coinbase's hosted facilitator at https://x402.org/facilitator).
// This eliminates the need to run a local blockchain node or hold private keys.
type RemoteFacilitator struct {
	baseURL    string
	httpClient *http.Client
}

// NewRemoteFacilitator creates a facilitator that calls an external endpoint.
// Pass CoinbaseX402FacilitatorURL to use Coinbase's hosted facilitator.
//
// The 30-second HTTP timeout protects against hung connections: if the facilitator
// is unreachable or unresponsive, each auto-pay attempt will fail after 30s and
// fall back to a standard 402 response rather than blocking the handler goroutine
// indefinitely.
func NewRemoteFacilitator(baseURL string) *RemoteFacilitator {
	return NewRemoteFacilitatorWithTimeout(baseURL, 30*time.Second)
}

// NewRemoteFacilitatorWithTimeout creates a facilitator with a custom HTTP timeout.
// Useful in tests to simulate slow or unresponsive facilitators.
func NewRemoteFacilitatorWithTimeout(baseURL string, timeout time.Duration) *RemoteFacilitator {
	return &RemoteFacilitator{
		baseURL: strings.TrimRight(baseURL, "/"),
		httpClient: &http.Client{
			Timeout: timeout,
		},
	}
}

// BaseURL returns the base URL of the facilitator endpoint.
func (r *RemoteFacilitator) BaseURL() string {
	return r.baseURL
}

// Verify calls the remote /verify endpoint.
func (r *RemoteFacilitator) Verify(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*VerifyResponse, error) {
	var resp VerifyResponse
	if err := r.post(ctx, "/verify", payment, requirements, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// Settle calls the remote /settle endpoint.
func (r *RemoteFacilitator) Settle(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*SettleResponse, error) {
	var resp SettleResponse
	if err := r.post(ctx, "/settle", payment, requirements, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// post marshals the payment and requirements into a FacilitatorRequest, POSTs to the
// given path on the remote facilitator, and decodes the JSON response into out.
// Non-2xx HTTP responses are returned as errors with the response body included so
// callers can distinguish a protocol failure from a negative result (e.g. IsValid:false).
//
// Retries are intentionally absent here — this runs inside an active HTTP handler
// goroutine. Retry logic belongs at a higher level if needed.
func (r *RemoteFacilitator) post(ctx context.Context, path string, payment *PaymentPayload, requirements *PaymentRequired, out any) error {
	payloadBytes, err := json.Marshal(payment)
	if err != nil {
		return fmt.Errorf("marshal payment: %w", err)
	}
	reqBytes, err := json.Marshal(requirements)
	if err != nil {
		return fmt.Errorf("marshal requirements: %w", err)
	}
	body, err := json.Marshal(FacilitatorRequest{
		PaymentPayload:      payloadBytes,
		PaymentRequirements: reqBytes,
	})
	if err != nil {
		return fmt.Errorf("marshal facilitator request: %w", err)
	}

	httpReq, err := http.NewRequestWithContext(ctx, http.MethodPost, r.baseURL+path, strings.NewReader(string(body)))
	if err != nil {
		return fmt.Errorf("build request: %w", err)
	}
	httpReq.Header.Set("Content-Type", "application/json")

	httpResp, err := r.httpClient.Do(httpReq)
	if err != nil {
		return fmt.Errorf("call remote facilitator %s: %w", path, err)
	}
	defer httpResp.Body.Close() //nolint:errcheck

	if httpResp.StatusCode < 200 || httpResp.StatusCode >= 300 {
		errBody, _ := io.ReadAll(io.LimitReader(httpResp.Body, 4096)) //nolint:errcheck
		return fmt.Errorf("facilitator %s: HTTP %d: %s", path, httpResp.StatusCode, strings.TrimSpace(string(errBody)))
	}

	if err := json.NewDecoder(httpResp.Body).Decode(out); err != nil {
		return fmt.Errorf("decode response from %s: %w", path, err)
	}
	return nil
}
