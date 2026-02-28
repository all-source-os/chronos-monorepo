package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
)

// OnboardRequest represents a request to start onboarding.
type OnboardRequest struct {
	Email string `json:"email" binding:"required"`
	Name  string `json:"name"`
}

// OnboardHandler provisions a new tenant with an API key and sample events.
// POST /api/v1/onboard/start
func (cp *ControlPlane) OnboardHandler(c *gin.Context) {
	var req OnboardRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Generate a deterministic tenant ID from email
	tenantSlug := strings.ReplaceAll(strings.ToLower(req.Email), "@", "-at-")
	tenantSlug = strings.ReplaceAll(tenantSlug, ".", "-")
	tenantID := fmt.Sprintf("onboard-%s", tenantSlug)

	name := req.Name
	if name == "" {
		name = strings.Split(req.Email, "@")[0]
	}

	// Create tenant via the CreateTenantUseCase
	tenantResp, err := cp.container.CreateTenantUC.Execute(dto.CreateTenantRequest{
		ID:          tenantID,
		Name:        name,
		Description: fmt.Sprintf("Onboarded tenant for %s", req.Email),
		Metadata: map[string]interface{}{
			"email": req.Email,
			"subscription": map[string]interface{}{
				"tier":   "free",
				"status": "active",
			},
			"quota": map[string]interface{}{
				"events_quota": 10000,
			},
		},
	})
	if err != nil {
		// If tenant already exists, that's fine — return a helpful error
		if strings.Contains(err.Error(), "already exists") {
			c.JSON(http.StatusConflict, gin.H{
				"error":   "tenant_exists",
				"message": "A tenant with this email already exists. Use /api/v1/auth/login to authenticate.",
			})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to create tenant", "message": err.Error()})
		return
	}

	// Generate API key as a long-lived JWT with IsAPIKey flag
	now := time.Now()
	apiKeyClaims := &Claims{
		UserID:   tenantID,
		Username: name,
		Email:    req.Email,
		Name:     name,
		TenantID: tenantResp.ID,
		Role:     entities.RoleDeveloper,
		IsAPIKey: true,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: now.Add(365 * 24 * time.Hour).Unix(), // 1 year
			IssuedAt:  now.Unix(),
			Issuer:    "allsource",
			Subject:   tenantID,
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, apiKeyClaims)
	apiKey, err := token.SignedString([]byte(cp.authClient.jwtSecret))
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to generate API key"})
		return
	}

	// Ingest sample events into Core for the new tenant
	sampleEvents := buildSampleEvents(tenantResp.ID)
	var ingestedCount int
	for _, event := range sampleEvents {
		resp, postErr := cp.client.R().
			SetBody(event).
			Post("/api/v1/events")
		if postErr == nil && resp.StatusCode() < 400 {
			ingestedCount++
		}
	}

	// Build sample curl commands
	baseURL := cp.getPublicBaseURL(c)
	curls := buildSampleCurls(baseURL, apiKey, tenantResp.ID)

	c.JSON(http.StatusCreated, gin.H{
		"tenant_id":       tenantResp.ID,
		"api_key":         apiKey,
		"sample_events":   ingestedCount,
		"tier":            "free",
		"events_quota":    10000,
		"getting_started": curls,
	})
}

// DemoStartHandler provisions a demo account through the normal registration flow.
// Generates demo credentials, registers them via Core auth, creates a demo tenant,
// seeds sample data, and returns the email + password for the user to log in normally.
//
// POST /api/v1/demo/start
func (cp *ControlPlane) DemoStartHandler(c *gin.Context) {
	// Generate unique demo credentials
	demoSlug := uuid.New().String()[:8]
	email := fmt.Sprintf("demo-%s@demo.allsource.dev", demoSlug)
	password := fmt.Sprintf("demo-%s-%s", demoSlug, uuid.New().String()[:8])
	name := "Demo User"

	// Compute tenant ID deterministically from the email BEFORE registration
	// so we can pass the same ID to both Core auth (user.tenant_id) and Core tenants.
	tenantSlug := strings.ReplaceAll(strings.ToLower(email), "@", "-at-")
	tenantSlug = strings.ReplaceAll(tenantSlug, ".", "-")
	tenantID := tenantSlug

	// Step 1: Register credentials in Core (same path as normal RegisterHandler)
	// Include tenant_id so Core's user record references the right tenant.
	regResp, err := cp.client.R().
		SetBody(map[string]interface{}{
			"username":  email,
			"email":     email,
			"password":  password,
			"tenant_id": tenantID,
		}).
		Post("/api/v1/auth/register")

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "service_unavailable", "message": "registration service is temporarily unavailable"})
		return
	}

	if regResp.StatusCode() != 201 && regResp.StatusCode() != 200 {
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "failed to create demo account",
			"message": fmt.Sprintf("registration returned HTTP %d", regResp.StatusCode()),
		})
		return
	}

	// Step 2: Create tenant with is_demo: true and enterprise quotas
	// Use the same tenantID that was registered with the user.
	tenantBody := map[string]interface{}{
		"id":      tenantID,
		"name":    name,
		"slug":    tenantSlug,
		"is_demo": true,
		"metadata": map[string]interface{}{
			"email": email,
			"subscription": map[string]interface{}{
				"tier":   "enterprise",
				"status": "active",
			},
			"quotas": map[string]interface{}{
				"events_quota":  -1,
				"queries_quota": -1,
			},
		},
	}

	tenantResp, err := cp.client.R().
		SetBody(tenantBody).
		Post("/api/v1/tenants")
	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "core service unavailable", "message": err.Error()})
		return
	}

	switch {
	case tenantResp.StatusCode() == 201 || tenantResp.StatusCode() == 200:
		// Tenant created successfully — tenantID already set
	case tenantResp.StatusCode() == 409:
		// Tenant already exists — tenantID already set
	default:
		c.JSON(http.StatusInternalServerError, gin.H{
			"error":   "failed to create demo tenant",
			"message": fmt.Sprintf("Core returned HTTP %d", tenantResp.StatusCode()),
		})
		return
	}

	// Step 3: Seed sample data
	sampleEvents := buildSampleEvents(tenantID)
	var ingestedCount int
	for _, event := range sampleEvents {
		r, postErr := cp.client.R().
			SetBody(event).
			Post("/api/v1/events")
		if postErr == nil && r.StatusCode() < 400 {
			ingestedCount++
		}
	}

	// Trigger Core's rich demo seed (1000 events with embeddings, idempotent)
	seedResp, seedErr := cp.client.R().
		Post("/api/v1/demo/seed")
	if seedErr == nil && seedResp.StatusCode() < 400 {
		var seedResult map[string]interface{}
		if json.Unmarshal(seedResp.Body(), &seedResult) == nil {
			if count, ok := seedResult["event_count"].(float64); ok {
				ingestedCount += int(count)
			}
		}
	}

	// Return credentials — the client logs in through the normal login flow
	c.JSON(http.StatusCreated, gin.H{
		"email":         email,
		"password":      password,
		"is_demo":       true,
		"sample_events": ingestedCount,
	})
}

// getPublicBaseURL returns the base URL for curl examples.
func (cp *ControlPlane) getPublicBaseURL(c *gin.Context) string {
	scheme := "https"
	if c.Request.TLS == nil {
		scheme = "http"
	}
	return fmt.Sprintf("%s://%s", scheme, c.Request.Host)
}

// buildSampleEvents returns 7 sample events that demonstrate common event sourcing patterns.
func buildSampleEvents(tenantID string) []map[string]interface{} {
	userID := uuid.New().String()
	orderID := uuid.New().String()

	return []map[string]interface{}{
		{
			"event_type": "user.signed_up",
			"entity_id":  userID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"email":    "alice@example.com",
				"plan":     "free",
				"referral": "organic",
			},
			"metadata": map[string]interface{}{
				"source":  "onboarding_sample",
				"ip":      "192.168.1.1",
				"browser": "Chrome/120",
			},
		},
		{
			"event_type": "user.profile_updated",
			"entity_id":  userID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"field":     "display_name",
				"old_value": "",
				"new_value": "Alice Smith",
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
		{
			"event_type": "order.created",
			"entity_id":  orderID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"user_id":  userID,
				"total":    49.99,
				"currency": "USD",
				"items": []map[string]interface{}{
					{"sku": "WIDGET-001", "qty": 2, "price": 24.99},
				},
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
		{
			"event_type": "payment.processed",
			"entity_id":  orderID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"amount":         49.99,
				"currency":       "USD",
				"payment_method": "card",
				"last_four":      "4242",
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
		{
			"event_type": "order.fulfilled",
			"entity_id":  orderID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"tracking_number": "1Z999AA10123456784",
				"carrier":         "UPS",
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
		{
			"event_type": "user.feature_used",
			"entity_id":  userID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"feature":  "dashboard",
				"duration": 120,
				"actions":  5,
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
		{
			"event_type": "notification.sent",
			"entity_id":  userID,
			"tenant_id":  tenantID,
			"payload": map[string]interface{}{
				"channel":  "email",
				"template": "order_confirmation",
				"order_id": orderID,
			},
			"metadata": map[string]interface{}{
				"source": "onboarding_sample",
			},
		},
	}
}

// buildSampleCurls returns sample curl commands for the new tenant.
func buildSampleCurls(baseURL, apiKey, tenantID string) map[string]string {
	return map[string]string{
		"ingest_event": fmt.Sprintf(
			`curl -X POST %s/api/v1/events -H "Authorization: Bearer %s" -H "Content-Type: application/json" -d '{"event_type":"user.action","entity_id":"user-123","payload":{"action":"click","page":"/home"}}'`,
			baseURL, apiKey,
		),
		"query_events": fmt.Sprintf(
			`curl %s/api/v1/events -H "Authorization: Bearer %s"`,
			baseURL, apiKey,
		),
		"query_by_type": fmt.Sprintf(
			`curl "%s/api/v1/events?event_type=order.created" -H "Authorization: Bearer %s"`,
			baseURL, apiKey,
		),
		"list_streams": fmt.Sprintf(
			`curl %s/api/v1/streams -H "Authorization: Bearer %s"`,
			baseURL, apiKey,
		),
		"list_event_types": fmt.Sprintf(
			`curl %s/api/v1/event-types -H "Authorization: Bearer %s"`,
			baseURL, apiKey,
		),
	}
}
