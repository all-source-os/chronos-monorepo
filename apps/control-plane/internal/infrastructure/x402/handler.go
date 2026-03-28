package x402

import (
	"encoding/json"
	"net/http"

	"github.com/gin-gonic/gin"
)

// Handler exposes the facilitator's /verify and /settle endpoints.
type Handler struct {
	facilitator *Facilitator
}

// NewHandler creates a new x402 HTTP handler.
func NewHandler(facilitator *Facilitator) *Handler {
	return &Handler{facilitator: facilitator}
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
