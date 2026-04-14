package x402

import (
	"encoding/json"
	"net/http"

	"github.com/gin-gonic/gin"
)

// Handler exposes the facilitator's /verify, /settle, and /routes endpoints.
type Handler struct {
	facilitator PaymentFacilitator
	pricing     *PricingConfig
}

// NewHandler creates a new x402 HTTP handler. pricing may be nil; if nil, the
// /routes endpoint reports x402 as disabled with an empty route list.
func NewHandler(facilitator PaymentFacilitator, pricing *PricingConfig) *Handler {
	return &Handler{facilitator: facilitator, pricing: pricing}
}

// RouteEntry is a single priced route in the /routes discovery response.
type RouteEntry struct {
	Method      string   `json:"method"`
	Path        string   `json:"path"`
	Amount      string   `json:"amount"`
	Asset       string   `json:"asset,omitempty"`
	Networks    []string `json:"networks"`
	Description string   `json:"description,omitempty"`
}

// RoutesResponse is the body returned by GET /x402/routes.
type RoutesResponse struct {
	Enabled          bool         `json:"enabled"`
	RecipientAddress string       `json:"recipientAddress,omitempty"`
	Routes           []RouteEntry `json:"routes"`
}

// Routes handles GET /x402/routes — returns the list of priced routes so
// clients can introspect pricing without trial-and-error 402s.
func (h *Handler) Routes(c *gin.Context) {
	resp := RoutesResponse{Routes: []RouteEntry{}}
	if h.pricing == nil {
		c.JSON(http.StatusOK, resp)
		return
	}

	resp.Enabled = h.pricing.Enabled
	resp.RecipientAddress = h.pricing.RecipientAddress

	for key, pricing := range h.pricing.Routes {
		method, path := splitRouteKey(key)
		resp.Routes = append(resp.Routes, RouteEntry{
			Method:      method,
			Path:        path,
			Amount:      pricing.Amount,
			Asset:       pricing.Asset,
			Networks:    pricing.Networks,
			Description: pricing.Description,
		})
	}

	c.JSON(http.StatusOK, resp)
}

// splitRouteKey reverses RouteKey ("METHOD /path" → "METHOD", "/path").
func splitRouteKey(key string) (string, string) {
	for i := 0; i < len(key); i++ {
		if key[i] == ' ' {
			return key[:i], key[i+1:]
		}
	}
	return "", key
}

// Verify handles POST /x402/verify.
func (h *Handler) Verify(c *gin.Context) {
	var req FacilitatorRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid_request", "message": err.Error()})
		return
	}

	payment, requirements, err := h.decodeFacilitatorRequest(&req)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "decode_failed", "message": err.Error()})
		return
	}

	resp, err := h.facilitator.Verify(c.Request.Context(), payment, requirements)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "verify_error", "message": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}

// Settle handles POST /x402/settle.
func (h *Handler) Settle(c *gin.Context) {
	var req FacilitatorRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid_request", "message": err.Error()})
		return
	}

	payment, requirements, err := h.decodeFacilitatorRequest(&req)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "decode_failed", "message": err.Error()})
		return
	}

	resp, err := h.facilitator.Settle(c.Request.Context(), payment, requirements)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "settle_error", "message": err.Error()})
		return
	}

	c.JSON(http.StatusOK, resp)
}

// decodeFacilitatorRequest unmarshals the raw JSON fields.
func (h *Handler) decodeFacilitatorRequest(req *FacilitatorRequest) (*PaymentPayload, *PaymentRequired, error) {
	var payment PaymentPayload
	if err := json.Unmarshal(req.PaymentPayload, &payment); err != nil {
		return nil, nil, err
	}
	var requirements PaymentRequired
	if err := json.Unmarshal(req.PaymentRequirements, &requirements); err != nil {
		return nil, nil, err
	}
	return &payment, &requirements, nil
}
