package clients

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"time"

	"github.com/go-resty/resty/v2"
)

const (
	lemonSqueezyBaseURL = "https://api.lemonsqueezy.com"
	lemonSqueezyTimeout = 30 * time.Second
	lemonSqueezyTracer  = "lemonsqueezy-client"
)

// LemonSqueezyClient defines methods for interacting with the LemonSqueezy API.
type LemonSqueezyClient interface {
	CreateCheckout(ctx context.Context, req CreateCheckoutRequest) (*CheckoutResponse, error)
	GetCustomerPortalURL(ctx context.Context, subscriptionID string) (string, error)
	ReportUsage(ctx context.Context, req ReportUsageRequest) error
	GetSubscription(ctx context.Context, subscriptionID string) (*SubscriptionResponse, error)
	LookupVariantID(tier string) (string, error)
}

// VariantMap maps plan tier names to LemonSqueezy variant IDs.
type VariantMap map[string]string

// --- Request types ---

// CreateCheckoutRequest contains parameters for creating a LemonSqueezy checkout.
type CreateCheckoutRequest struct {
	VariantID   string            `json:"variant_id"`
	Email       string            `json:"email,omitempty"`
	Name        string            `json:"name,omitempty"`
	CustomData  map[string]string `json:"custom_data,omitempty"`
	RedirectURL string            `json:"redirect_url,omitempty"`
}

// ReportUsageRequest contains parameters for reporting usage to LemonSqueezy.
type ReportUsageRequest struct {
	SubscriptionItemID string `json:"subscription_item_id"`
	Quantity           int    `json:"quantity"`
	Action             string `json:"action,omitempty"` // "increment" (default) or "set"
}

// --- Response types ---

// CheckoutResponse contains the result of creating a checkout.
type CheckoutResponse struct {
	ID  string `json:"id"`
	URL string `json:"url"`
}

// SubscriptionResponse contains subscription details from LemonSqueezy.
type SubscriptionResponse struct {
	ID                     string `json:"id"`
	Status                 string `json:"status"`
	VariantID              int    `json:"variant_id"`
	ProductID              int    `json:"product_id"`
	CustomerID             int    `json:"customer_id"`
	CardBrand              string `json:"card_brand,omitempty"`
	CardLastFour           string `json:"card_last_four,omitempty"`
	RenewsAt               string `json:"renews_at,omitempty"`
	EndsAt                 string `json:"ends_at,omitempty"`
	CustomerPortalURL      string `json:"customer_portal_url,omitempty"`
	UpdatePaymentMethodURL string `json:"update_payment_method_url,omitempty"`
}

// --- JSON:API envelope types (internal) ---

type jsonAPIEnvelope struct {
	Data jsonAPIData `json:"data"`
}

type jsonAPIData struct {
	Type          string          `json:"type"`
	ID            string          `json:"id,omitempty"`
	Attributes    json.RawMessage `json:"attributes"`
	Relationships json.RawMessage `json:"relationships,omitempty"`
}

type checkoutAttributes struct {
	StoreID        int                  `json:"store_id,omitempty"`
	URL            string               `json:"url,omitempty"`
	CheckoutData   *checkoutDataAttrs   `json:"checkout_data,omitempty"`
	ProductOptions *productOptionsAttrs `json:"product_options,omitempty"`
}

type checkoutDataAttrs struct {
	Email  string            `json:"email,omitempty"`
	Name   string            `json:"name,omitempty"`
	Custom map[string]string `json:"custom,omitempty"`
}

type productOptionsAttrs struct {
	RedirectURL string `json:"redirect_url,omitempty"`
}

type subscriptionAttributes struct {
	Status       string           `json:"status"`
	StoreID      int              `json:"store_id"`
	CustomerID   int              `json:"customer_id"`
	OrderID      int              `json:"order_id"`
	ProductID    int              `json:"product_id"`
	VariantID    int              `json:"variant_id"`
	CardBrand    string           `json:"card_brand"`
	CardLastFour string           `json:"card_last_four"`
	RenewsAt     string           `json:"renews_at"`
	EndsAt       string           `json:"ends_at"`
	URLs         subscriptionURLs `json:"urls"`
}

type subscriptionURLs struct {
	UpdatePaymentMethod string `json:"update_payment_method"`
	CustomerPortal      string `json:"customer_portal"`
}

type jsonAPIRelationship struct {
	Data struct {
		Type string `json:"type"`
		ID   string `json:"id"`
	} `json:"data"`
}

// --- Implementation ---

type lemonSqueezyClient struct {
	client     *resty.Client
	storeID    string
	variantMap VariantMap
}

// NewLemonSqueezyClient creates a LemonSqueezyClient from environment variables:
//   - LEMON_SQUEEZY_API_KEY (required)
//   - LEMON_SQUEEZY_STORE_ID (required)
//   - LEMON_SQUEEZY_VARIANT_MAP (optional JSON: {"free":"var_1","pro":"var_2","team":"var_3"})
func NewLemonSqueezyClient() (LemonSqueezyClient, error) {
	apiKey := os.Getenv("LEMON_SQUEEZY_API_KEY")
	if apiKey == "" {
		return nil, fmt.Errorf("LEMON_SQUEEZY_API_KEY is required")
	}

	storeID := os.Getenv("LEMON_SQUEEZY_STORE_ID")
	if storeID == "" {
		return nil, fmt.Errorf("LEMON_SQUEEZY_STORE_ID is required")
	}

	var variantMap VariantMap
	if raw := os.Getenv("LEMON_SQUEEZY_VARIANT_MAP"); raw != "" {
		if err := json.Unmarshal([]byte(raw), &variantMap); err != nil {
			return nil, fmt.Errorf("invalid LEMON_SQUEEZY_VARIANT_MAP: %w", err)
		}
	}

	return newLemonSqueezyClient(lemonSqueezyBaseURL, apiKey, storeID, variantMap), nil
}

func newLemonSqueezyClient(baseURL, apiKey, storeID string, variantMap VariantMap) *lemonSqueezyClient {
	transport := &http.Transport{
		MaxIdleConns:        20,
		MaxIdleConnsPerHost: 20,
		IdleConnTimeout:     90 * time.Second,
	}

	client := resty.NewWithClient(&http.Client{Transport: transport}).
		SetBaseURL(baseURL).
		SetTimeout(lemonSqueezyTimeout).
		SetHeader("Accept", "application/vnd.api+json").
		SetHeader("Content-Type", "application/vnd.api+json").
		SetHeader("Authorization", "Bearer "+apiKey).
		SetRetryCount(2).
		SetRetryWaitTime(500 * time.Millisecond).
		SetRetryMaxWaitTime(3 * time.Second).
		AddRetryCondition(func(r *resty.Response, err error) bool {
			if err != nil {
				return true
			}
			return r.StatusCode() == 429 || r.StatusCode() >= 500
		})

	return &lemonSqueezyClient{
		client:     client,
		storeID:    storeID,
		variantMap: variantMap,
	}
}

func (c *lemonSqueezyClient) CreateCheckout(ctx context.Context, req CreateCheckoutRequest) (*CheckoutResponse, error) {
	attrs := checkoutAttributes{}
	if req.Email != "" || req.Name != "" || len(req.CustomData) > 0 {
		attrs.CheckoutData = &checkoutDataAttrs{
			Email:  req.Email,
			Name:   req.Name,
			Custom: req.CustomData,
		}
	}
	if req.RedirectURL != "" {
		attrs.ProductOptions = &productOptionsAttrs{
			RedirectURL: req.RedirectURL,
		}
	}

	attrsJSON, err := json.Marshal(attrs)
	if err != nil {
		return nil, fmt.Errorf("marshal checkout attributes: %w", err)
	}

	rels := map[string]jsonAPIRelationship{
		"store": {Data: struct {
			Type string `json:"type"`
			ID   string `json:"id"`
		}{Type: "stores", ID: c.storeID}},
		"variant": {Data: struct {
			Type string `json:"type"`
			ID   string `json:"id"`
		}{Type: "variants", ID: req.VariantID}},
	}

	relsJSON, err := json.Marshal(rels)
	if err != nil {
		return nil, fmt.Errorf("marshal checkout relationships: %w", err)
	}

	body := jsonAPIEnvelope{
		Data: jsonAPIData{
			Type:          "checkouts",
			Attributes:    attrsJSON,
			Relationships: relsJSON,
		},
	}

	var respEnvelope jsonAPIEnvelope
	resp, err := c.client.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&respEnvelope).
		Post("/v1/checkouts")

	if err != nil {
		return nil, fmt.Errorf("create checkout request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return nil, fmt.Errorf("create checkout returned status %d: %s", resp.StatusCode(), resp.String())
	}

	var respAttrs checkoutAttributes
	if err := json.Unmarshal(respEnvelope.Data.Attributes, &respAttrs); err != nil {
		return nil, fmt.Errorf("unmarshal checkout response: %w", err)
	}

	return &CheckoutResponse{
		ID:  respEnvelope.Data.ID,
		URL: respAttrs.URL,
	}, nil
}

func (c *lemonSqueezyClient) GetCustomerPortalURL(ctx context.Context, subscriptionID string) (string, error) {
	sub, err := c.GetSubscription(ctx, subscriptionID)
	if err != nil {
		return "", err
	}
	if sub.CustomerPortalURL == "" {
		return "", fmt.Errorf("no customer portal URL for subscription %s", subscriptionID)
	}
	return sub.CustomerPortalURL, nil
}

func (c *lemonSqueezyClient) ReportUsage(ctx context.Context, req ReportUsageRequest) error {
	action := req.Action
	if action == "" {
		action = "increment"
	}

	type usageAttrs struct {
		Quantity int    `json:"quantity"`
		Action   string `json:"action"`
	}

	attrs, err := json.Marshal(usageAttrs{Quantity: req.Quantity, Action: action})
	if err != nil {
		return fmt.Errorf("marshal usage attributes: %w", err)
	}

	rels := map[string]jsonAPIRelationship{
		"subscription-item": {Data: struct {
			Type string `json:"type"`
			ID   string `json:"id"`
		}{Type: "subscription-items", ID: req.SubscriptionItemID}},
	}

	relsJSON, err := json.Marshal(rels)
	if err != nil {
		return fmt.Errorf("marshal usage relationships: %w", err)
	}

	body := jsonAPIEnvelope{
		Data: jsonAPIData{
			Type:          "usage-records",
			Attributes:    attrs,
			Relationships: relsJSON,
		},
	}

	resp, err := c.client.R().
		SetContext(ctx).
		SetBody(body).
		Post("/v1/usage-records")

	if err != nil {
		return fmt.Errorf("report usage request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return fmt.Errorf("report usage returned status %d: %s", resp.StatusCode(), resp.String())
	}

	return nil
}

func (c *lemonSqueezyClient) GetSubscription(ctx context.Context, subscriptionID string) (*SubscriptionResponse, error) {
	var respEnvelope jsonAPIEnvelope
	resp, err := c.client.R().
		SetContext(ctx).
		SetResult(&respEnvelope).
		Get("/v1/subscriptions/" + subscriptionID)

	if err != nil {
		return nil, fmt.Errorf("get subscription request failed: %w", err)
	}
	if resp.StatusCode() >= 400 {
		return nil, fmt.Errorf("get subscription returned status %d: %s", resp.StatusCode(), resp.String())
	}

	var attrs subscriptionAttributes
	if err := json.Unmarshal(respEnvelope.Data.Attributes, &attrs); err != nil {
		return nil, fmt.Errorf("unmarshal subscription response: %w", err)
	}

	return &SubscriptionResponse{
		ID:                     respEnvelope.Data.ID,
		Status:                 attrs.Status,
		VariantID:              attrs.VariantID,
		ProductID:              attrs.ProductID,
		CustomerID:             attrs.CustomerID,
		CardBrand:              attrs.CardBrand,
		CardLastFour:           attrs.CardLastFour,
		RenewsAt:               attrs.RenewsAt,
		EndsAt:                 attrs.EndsAt,
		CustomerPortalURL:      attrs.URLs.CustomerPortal,
		UpdatePaymentMethodURL: attrs.URLs.UpdatePaymentMethod,
	}, nil
}

// LookupVariantID resolves a plan tier name to a LemonSqueezy variant ID using the configured variant map.
func (c *lemonSqueezyClient) LookupVariantID(tier string) (string, error) {
	if c.variantMap == nil {
		return "", fmt.Errorf("variant map not configured")
	}
	id, ok := c.variantMap[tier]
	if !ok {
		return "", fmt.Errorf("unknown plan tier: %s", tier)
	}
	return id, nil
}
