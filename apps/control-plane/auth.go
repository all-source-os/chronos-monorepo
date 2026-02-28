package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// Claims represents JWT claims
type Claims struct {
	UserID   string        `json:"sub"`
	Username string        `json:"username"`
	Email    string        `json:"email,omitempty"`
	Name     string        `json:"name,omitempty"`
	TenantID string        `json:"tenant_id"`
	Role     entities.Role `json:"role"`
	IsAPIKey bool          `json:"is_api_key,omitempty"`
	jwt.StandardClaims
}

// AuthContext holds authentication information for a request
type AuthContext struct {
	UserID   string
	Username string
	TenantID string
	Role     entities.Role
	IsAPIKey bool
}

// AuthClient handles authentication with the core service.
// NOTE: JWT_SECRET is shared between CP and QS via HMAC-SHA256 (HS256).
// Adding new services to the fleet requires distributing another copy.
// TODO: Migrate to RS256 with a JWKS endpoint on CP so other services can verify
// tokens without possessing the signing key.
type AuthClient struct {
	jwtSecret string
}

// NewAuthClient creates a new authentication client
func NewAuthClient(jwtSecret string) *AuthClient {
	if jwtSecret == "" {
		jwtSecret = "default-secret-change-in-production"
	}
	return &AuthClient{
		jwtSecret: jwtSecret,
	}
}

// ValidateToken validates a JWT token and returns claims
func (a *AuthClient) ValidateToken(tokenString string) (*Claims, error) {
	// Parse the token
	token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(token *jwt.Token) (interface{}, error) {
		// Validate the signing method
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
		}
		return []byte(a.jwtSecret), nil
	})

	if err != nil {
		return nil, fmt.Errorf("failed to parse token: %w", err)
	}

	claims, ok := token.Claims.(*Claims)
	if !ok || !token.Valid {
		return nil, errors.New("invalid token")
	}

	// Check expiration
	if claims.ExpiresAt < time.Now().Unix() {
		return nil, errors.New("token expired")
	}

	return claims, nil
}

// ExtractToken extracts the token from the Authorization header
func ExtractToken(c *gin.Context) (string, error) {
	authHeader := c.GetHeader("Authorization")
	if authHeader == "" {
		return "", errors.New("no authorization header")
	}

	// Expected format: "Bearer <token>"
	parts := strings.SplitN(authHeader, " ", 2)
	if len(parts) != 2 || parts[0] != "Bearer" {
		return "", errors.New("invalid authorization header format")
	}

	return parts[1], nil
}

// RoleHasPermission checks if a role has a specific permission.
func RoleHasPermission(role entities.Role, perm entities.Permission) bool {
	switch role {
	case entities.RoleAdmin:
		return true
	case entities.RoleDeveloper:
		return perm == entities.PermissionRead || perm == entities.PermissionWrite ||
			perm == entities.PermissionMetrics || perm == entities.PermissionManageSchemas ||
			perm == entities.PermissionManagePipelines
	case entities.RoleReadOnly:
		return perm == entities.PermissionRead || perm == entities.PermissionMetrics
	case entities.RoleServiceAccount:
		return perm == entities.PermissionRead || perm == entities.PermissionWrite
	default:
		return false
	}
}

// AuthMiddleware validates JWT tokens and adds auth context to requests
func AuthMiddleware(authClient *AuthClient) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Skip auth for health, metrics, public cluster health, webhooks, and auth endpoints
		if c.Request.URL.Path == pathHealth || c.Request.URL.Path == pathMetrics || c.Request.URL.Path == "/docs" || c.Request.URL.Path == "/openapi" || c.Request.URL.Path == "/api/v1/cluster/health" || strings.HasPrefix(c.Request.URL.Path, "/api/v1/webhooks/") || strings.HasPrefix(c.Request.URL.Path, "/api/v1/auth/") || strings.HasPrefix(c.Request.URL.Path, "/api/v1/onboard/") || strings.HasPrefix(c.Request.URL.Path, "/api/v1/demo/") {
			c.Next()
			return
		}

		token, err := ExtractToken(c)
		if err != nil {
			c.JSON(401, gin.H{"error": "unauthorized", "message": err.Error()})
			c.Abort()
			return
		}

		claims, err := authClient.ValidateToken(token)
		if err != nil {
			c.JSON(401, gin.H{"error": "unauthorized", "message": err.Error()})
			c.Abort()
			return
		}

		// Create auth context
		authCtx := &AuthContext{
			UserID:   claims.UserID,
			Username: claims.Username,
			TenantID: claims.TenantID,
			Role:     claims.Role,
			IsAPIKey: claims.IsAPIKey,
		}

		// Store in context
		c.Set("auth", authCtx)
		c.Set("auth_role", authCtx.Role)      // Separate key for cross-package access
		c.Set("auth_user_id", authCtx.UserID) // Separate key for cross-package access
		c.Next()
	}
}

// RequirePermission returns a middleware that checks for a specific permission
func RequirePermission(perm entities.Permission) gin.HandlerFunc {
	return func(c *gin.Context) {
		authCtx, exists := c.Get("auth")
		if !exists {
			c.JSON(401, gin.H{"error": "unauthorized", "message": "authentication required"})
			c.Abort()
			return
		}

		auth, ok := authCtx.(*AuthContext)
		if !ok {
			c.JSON(500, gin.H{"error": "internal error", "message": "invalid auth context"})
			c.Abort()
			return
		}
		if !RoleHasPermission(auth.Role, perm) {
			c.JSON(403, gin.H{
				"error":   "forbidden",
				"message": fmt.Sprintf("permission denied: %s required", perm),
			})
			c.Abort()
			return
		}

		c.Next()
	}
}

// RequireAdmin returns a middleware that requires admin role
func RequireAdmin() gin.HandlerFunc {
	return RequirePermission(entities.PermissionAdmin)
}

// GetAuthContext retrieves the auth context from the gin context
func GetAuthContext(c *gin.Context) (*AuthContext, error) {
	authCtx, exists := c.Get("auth")
	if !exists {
		return nil, errors.New("no auth context found")
	}

	auth, ok := authCtx.(*AuthContext)
	if !ok {
		return nil, errors.New("invalid auth context type")
	}

	return auth, nil
}

// oauthUserResult holds the result of finding or creating an OAuth user.
type oauthUserResult struct {
	Token     string
	UserID    string
	TenantID  string
	IsNewUser bool
}

// findOrCreateOAuthUser creates or finds a tenant for the OAuth user and signs a JWT.
func (cp *ControlPlane) findOrCreateOAuthUser(provider, providerID, email, name string) (*oauthUserResult, error) {
	// Generate a deterministic user ID from provider info
	userID := fmt.Sprintf("oauth:%s:%s", provider, providerID)

	// Build tenant slug from email
	tenantSlug := strings.ReplaceAll(strings.ToLower(email), "@", "-at-")
	tenantSlug = strings.ReplaceAll(tenantSlug, ".", "-")

	tenantBody := map[string]interface{}{
		"id":   userID,
		"name": name,
		"slug": tenantSlug,
		"metadata": map[string]interface{}{
			"subscription": map[string]interface{}{
				"tier":   "free",
				"status": "active",
			},
			"quota": map[string]interface{}{
				"events_quota": 10000,
			},
		},
	}

	// Create or get tenant from Core
	resp, err := cp.client.R().
		SetBody(tenantBody).
		Post("/api/v1/tenants")

	if err != nil {
		return nil, fmt.Errorf("core service unavailable: %w", err)
	}

	var tenantID string
	isNewUser := false

	switch {
	case resp.StatusCode() == 201 || resp.StatusCode() == 200:
		var result map[string]interface{}
		if parseErr := json.Unmarshal(resp.Body(), &result); parseErr == nil {
			if id, ok := result["id"].(string); ok {
				tenantID = id
			}
			isNewUser = resp.StatusCode() == 201
		}
		if tenantID == "" {
			tenantID = userID
		}
	case resp.StatusCode() == 409:
		// Tenant already exists — returning user
		tenantID = userID
		isNewUser = false
	default:
		return nil, fmt.Errorf("failed to create tenant (HTTP %d)", resp.StatusCode())
	}

	// Sign JWT
	now := time.Now()
	claims := &Claims{
		UserID:   userID,
		Username: name,
		Email:    email,
		Name:     name,
		TenantID: tenantID,
		Role:     entities.RoleDeveloper,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: now.Add(7 * 24 * time.Hour).Unix(),
			IssuedAt:  now.Unix(),
			Issuer:    "allsource",
			Subject:   userID,
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenString, err := token.SignedString([]byte(cp.authClient.jwtSecret))
	if err != nil {
		return nil, fmt.Errorf("failed to sign JWT: %w", err)
	}

	return &oauthUserResult{
		Token:     tokenString,
		UserID:    userID,
		TenantID:  tenantID,
		IsNewUser: isNewUser,
	}, nil
}

// LoginRequest represents a login request from the frontend.
// Accepts "email" (preferred) or "username" for the identifier field.
type LoginRequest struct {
	Email    string `json:"email"`
	Username string `json:"username"`
	Password string `json:"password" binding:"required"`
}

// RegisterRequest represents a registration request from the frontend.
type RegisterRequest struct {
	Name     string `json:"name" binding:"required"`
	Email    string `json:"email" binding:"required"`
	Password string `json:"password" binding:"required"`
}

// LoginHandler handles user login.
// Proxies credential verification to Core, then signs a CP-issued JWT on success.
// TODO: CP should own credential storage (PostgreSQL) instead of proxying to Core.
func (cp *ControlPlane) LoginHandler(c *gin.Context) {
	var req LoginRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Frontend sends "email", Core expects "username" — translate
	username := req.Email
	if username == "" {
		username = req.Username
	}
	if username == "" {
		c.JSON(400, gin.H{"error": "invalid request", "message": "email or username is required"})
		return
	}

	// Verify credentials against Core
	resp, err := cp.client.R().
		SetBody(map[string]string{
			"username": username,
			"password": req.Password,
		}).
		Post("/api/v1/auth/login")

	if err != nil {
		c.JSON(503, gin.H{"error": "service_unavailable", "message": "authentication service is temporarily unavailable"})
		return
	}

	if resp.StatusCode() != 200 {
		// Forward Core's error response (wrong password, user not found, etc.)
		var errResult map[string]interface{}
		if json.Unmarshal(resp.Body(), &errResult) == nil {
			c.JSON(resp.StatusCode(), errResult)
			return
		}
		c.JSON(401, gin.H{"error": "invalid_credentials", "message": "Invalid email or password"})
		return
	}

	// Parse Core's response to extract user info
	var coreResp struct {
		Token string `json:"token"`
		User  struct {
			ID       string `json:"id"`
			Username string `json:"username"`
			Email    string `json:"email"`
			Role     string `json:"role"`
			TenantID string `json:"tenant_id"`
		} `json:"user"`
	}
	if err := json.Unmarshal(resp.Body(), &coreResp); err != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to parse authentication response"})
		return
	}

	// Sign a CP-issued JWT (consistent with OAuth flow)
	userID := coreResp.User.ID
	if userID == "" {
		userID = fmt.Sprintf("email:%s", username)
	}
	tenantID := coreResp.User.TenantID
	if tenantID == "" {
		tenantID = userID
	}
	displayName := coreResp.User.Username
	if displayName == "" {
		displayName = username
	}

	now := time.Now()
	claims := &Claims{
		UserID:   userID,
		Username: displayName,
		Email:    username,
		Name:     displayName,
		TenantID: tenantID,
		Role:     entities.RoleDeveloper,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: now.Add(7 * 24 * time.Hour).Unix(),
			IssuedAt:  now.Unix(),
			Issuer:    "allsource",
			Subject:   userID,
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenString, err := token.SignedString([]byte(cp.authClient.jwtSecret))
	if err != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to create session"})
		return
	}

	c.JSON(200, gin.H{
		"token":    tokenString,
		"new_user": false,
		"user": gin.H{
			"id":        userID,
			"email":     username,
			"name":      displayName,
			"tenant_id": tenantID,
		},
	})
}

// RegisterHandler handles user registration.
// Proxies credential creation to Core, creates a tenant, and signs a CP-issued JWT.
// TODO: CP should own credential storage (PostgreSQL) instead of proxying to Core.
func (cp *ControlPlane) RegisterHandler(c *gin.Context) {
	var req RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Register credentials in Core (Core handles password hashing)
	resp, err := cp.client.R().
		SetBody(map[string]string{
			"username": req.Email,
			"email":    req.Email,
			"password": req.Password,
		}).
		Post("/api/v1/auth/register")

	if err != nil {
		c.JSON(503, gin.H{"error": "service_unavailable", "message": "registration service is temporarily unavailable"})
		return
	}

	if resp.StatusCode() != 201 && resp.StatusCode() != 200 {
		body := string(resp.Body())
		// Core returns 400/409 with "already exists" for duplicate usernames
		if resp.StatusCode() == 409 || strings.Contains(strings.ToLower(body), "already exists") {
			c.JSON(409, gin.H{"error": "email_exists", "message": "An account with this email already exists"})
			return
		}
		var errResult map[string]interface{}
		if json.Unmarshal(resp.Body(), &errResult) == nil {
			c.JSON(resp.StatusCode(), errResult)
			return
		}
		c.JSON(resp.StatusCode(), gin.H{"error": "registration_failed", "message": "Registration failed"})
		return
	}

	// Parse Core's response for user ID
	var coreResp struct {
		UserID   string `json:"user_id"`
		Username string `json:"username"`
		Email    string `json:"email"`
		TenantID string `json:"tenant_id"`
	}
	if err := json.Unmarshal(resp.Body(), &coreResp); err != nil {
		coreResp.UserID = fmt.Sprintf("email:%s", req.Email)
	}

	userID := coreResp.UserID
	if userID == "" {
		userID = fmt.Sprintf("email:%s", req.Email)
	}

	// Create tenant (same pattern as OAuth)
	result, err := cp.findOrCreateOAuthUser("email", userID, req.Email, req.Name)
	if err != nil {
		c.JSON(500, gin.H{"error": "internal_error", "message": "failed to create account"})
		return
	}

	c.JSON(201, gin.H{
		"token":    result.Token,
		"new_user": true,
		"user": gin.H{
			"id":        result.UserID,
			"email":     req.Email,
			"name":      req.Name,
			"tenant_id": result.TenantID,
		},
	})
}
