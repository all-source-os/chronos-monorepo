package clients

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/go-resty/resty/v2"
)

const (
	stripeBaseURL = "https://api.stripe.com"
	stripeTimeout = 30 * time.Second
)

// StripeClient defines methods for interacting with the Stripe API.
type StripeClient interface {
	CreateCheckoutSession(ctx context.Context, req StripeCheckoutRequest) (*StripeCheckoutResponse, error)
	GetCustomerPortalURL(ctx context.Context, customerID, returnURL string) (string, error)
	GetSubscription(ctx context.Context, subscriptionID string) (*StripeSubscriptionResponse, error)
	// LookupPriceID resolves a tier + billing period to a Stripe price ID.
	// period is "monthly" or "annual"; an empty period defaults to monthly.
	LookupPriceID(tier, period string) (string, error)
}

// StripePriceMap maps plan tier names to Stripe price IDs.
type StripePriceMap map[string]string

// --- Request/Response types ---

// StripeCheckoutRequest contains parameters for creating a Stripe Checkout session.
type StripeCheckoutRequest struct {
	PriceID       string            `json:"price_id"`
	CustomerEmail string            `json:"customer_email,omitempty"`
	Metadata      map[string]string `json:"metadata,omitempty"`
	SuccessURL    string            `json:"success_url"`
	CancelURL     string            `json:"cancel_url,omitempty"`
}

// StripeCheckoutResponse contains the result of creating a Checkout session.
type StripeCheckoutResponse struct {
	ID  string `json:"id"`
	URL string `json:"url"`
}

// StripeSubscriptionResponse contains subscription details from Stripe.
type StripeSubscriptionResponse struct {
	ID                string            `json:"id"`
	Status            string            `json:"status"`
	CustomerID        string            `json:"customer"`
	CurrentPeriodEnd  int64             `json:"current_period_end"`
	CancelAtPeriodEnd bool              `json:"cancel_at_period_end"`
	Metadata          map[string]string `json:"metadata"`
	Items             *stripeSubItems   `json:"items"`
}

type stripeSubItems struct {
	Data []stripeSubItem `json:"data"`
}

type stripeSubItem struct {
	ID    string      `json:"id"`
	Price stripePrice `json:"price"`
}

type stripePrice struct {
	ID        string `json:"id"`
	ProductID string `json:"product"`
}

// --- Implementation ---

type stripeClient struct {
	client   *resty.Client
	priceMap StripePriceMap
}

// NewStripeClient creates a StripeClient from environment variables:
//   - STRIPE_SECRET_KEY (required) — use a sk_test_… key in non-prod.
//   - STRIPE_PRICE_MAP (optional JSON). Keys are "<tier>:<period>" where period
//     is "monthly" or "annual"; a bare "<tier>" key is treated as the monthly
//     price. 011 tiers: indie, studio, scale. Example:
//     {"indie:monthly":"price_a","indie:annual":"price_b",
//     "studio:monthly":"price_c","studio:annual":"price_d",
//     "scale:monthly":"price_e","scale:annual":"price_f"}
//     See docs/runbooks/PRICING_BILLING_CUTOVER.md for how the price IDs are
//     created (test + live) and wired in.
func NewStripeClient() (StripeClient, error) {
	secretKey := os.Getenv("STRIPE_SECRET_KEY")
	if secretKey == "" {
		return nil, fmt.Errorf("STRIPE_SECRET_KEY is required")
	}

	var priceMap StripePriceMap
	if raw := os.Getenv("STRIPE_PRICE_MAP"); raw != "" {
		if err := json.Unmarshal([]byte(raw), &priceMap); err != nil {
			return nil, fmt.Errorf("invalid STRIPE_PRICE_MAP: %w", err)
		}
	}

	return newStripeClient(stripeBaseURL, secretKey, priceMap), nil
}

func newStripeClient(baseURL, secretKey string, priceMap StripePriceMap) *stripeClient {
	transport := &http.Transport{
		MaxIdleConns:        20,
		MaxIdleConnsPerHost: 20,
		IdleConnTimeout:     90 * time.Second,
	}

	client := resty.NewWithClient(&http.Client{Transport: transport}).
		SetBaseURL(baseURL).
		SetTimeout(stripeTimeout).
		SetHeader("Authorization", "Bearer "+secretKey).
		SetRetryCount(2).
		SetRetryWaitTime(500 * time.Millisecond).
		SetRetryMaxWaitTime(3 * time.Second).
		AddRetryCondition(func(r *resty.Response, err error) bool {
			if err != nil {
				return true
			}
			return r.StatusCode() == 429 || r.StatusCode() >= 500
		})

	return &stripeClient{
		client:   client,
		priceMap: priceMap,
	}
}

func (c *stripeClient) CreateCheckoutSession(ctx context.Context, req StripeCheckoutRequest) (*StripeCheckoutResponse, error) {
	// Stripe API uses form-encoded parameters
	params := url.Values{}
	params.Set("mode", "subscription")
	params.Set("line_items[0][price]", req.PriceID)
	params.Set("line_items[0][quantity]", "1")
	params.Set("success_url", req.SuccessURL)
	if req.CancelURL != "" {
		params.Set("cancel_url", req.CancelURL)
	}
	if req.CustomerEmail != "" {
		params.Set("customer_email", req.CustomerEmail)
	}
	for k, v := range req.Metadata {
		params.Set(fmt.Sprintf("metadata[%s]", k), v)
	}
	// Pass metadata to subscription as well so webhooks can access it
	for k, v := range req.Metadata {
		params.Set(fmt.Sprintf("subscription_data[metadata][%s]", k), v)
	}

	resp, err := c.client.R().
		SetContext(ctx).
		SetHeader("Content-Type", "application/x-www-form-urlencoded").
		SetBody(params.Encode()).
		Post("/v1/checkout/sessions")

	if err != nil {
		return nil, fmt.Errorf("create checkout session request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return nil, fmt.Errorf("create checkout session returned status %d: %s", resp.StatusCode(), resp.String())
	}

	var result struct {
		ID  string `json:"id"`
		URL string `json:"url"`
	}
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		return nil, fmt.Errorf("unmarshal checkout session response: %w", err)
	}

	return &StripeCheckoutResponse{
		ID:  result.ID,
		URL: result.URL,
	}, nil
}

func (c *stripeClient) GetCustomerPortalURL(ctx context.Context, customerID, returnURL string) (string, error) {
	params := url.Values{}
	params.Set("customer", customerID)
	if returnURL != "" {
		params.Set("return_url", returnURL)
	}

	resp, err := c.client.R().
		SetContext(ctx).
		SetHeader("Content-Type", "application/x-www-form-urlencoded").
		SetBody(params.Encode()).
		Post("/v1/billing_portal/sessions")

	if err != nil {
		return "", fmt.Errorf("create portal session request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return "", fmt.Errorf("create portal session returned status %d: %s", resp.StatusCode(), resp.String())
	}

	var result struct {
		URL string `json:"url"`
	}
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		return "", fmt.Errorf("unmarshal portal session response: %w", err)
	}

	return result.URL, nil
}

func (c *stripeClient) GetSubscription(ctx context.Context, subscriptionID string) (*StripeSubscriptionResponse, error) {
	resp, err := c.client.R().
		SetContext(ctx).
		Get("/v1/subscriptions/" + subscriptionID)

	if err != nil {
		return nil, fmt.Errorf("get subscription request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return nil, fmt.Errorf("get subscription returned status %d: %s", resp.StatusCode(), resp.String())
	}

	var sub StripeSubscriptionResponse
	if err := json.Unmarshal(resp.Body(), &sub); err != nil {
		return nil, fmt.Errorf("unmarshal subscription response: %w", err)
	}

	return &sub, nil
}

// LookupPriceID resolves a plan tier + billing period to a Stripe price ID.
// Resolution order (first hit wins):
//  1. "<tier>:<period>"  (e.g. "studio:annual")
//  2. "<tier>:monthly"   when period is empty/unknown
//  3. "<tier>"           legacy bare-tier key (treated as monthly)
//
// This keeps backwards compatibility with old single-price-per-tier maps while
// supporting the 011 monthly/annual split.
func (c *stripeClient) LookupPriceID(tier, period string) (string, error) {
	if c.priceMap == nil {
		return "", fmt.Errorf("price map not configured")
	}
	t := strings.ToLower(tier)
	p := strings.ToLower(period)
	if p == "" || (p != "monthly" && p != "annual" && p != "yearly") {
		p = "monthly"
	}
	if p == "yearly" {
		p = "annual"
	}

	if id, ok := c.priceMap[t+":"+p]; ok {
		return id, nil
	}
	if id, ok := c.priceMap[t+":monthly"]; ok {
		return id, nil
	}
	if id, ok := c.priceMap[t]; ok {
		return id, nil
	}
	return "", fmt.Errorf("no Stripe price configured for tier %q period %q", tier, period)
}
