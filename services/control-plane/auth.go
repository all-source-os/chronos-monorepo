package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"
)

// Role represents a user's role in the system
type Role string

const (
	RoleAdmin          Role = "Admin"
	RoleDeveloper      Role = "Developer"
	RoleReadOnly       Role = "ReadOnly"
	RoleServiceAccount Role = "ServiceAccount"
)

// Permission represents a specific permission
type Permission string

const (
	PermissionRead            Permission = "Read"
	PermissionWrite           Permission = "Write"
	PermissionAdmin           Permission = "Admin"
	PermissionMetrics         Permission = "Metrics"
	PermissionManageSchemas   Permission = "ManageSchemas"
	PermissionManagePipelines Permission = "ManagePipelines"
	PermissionManageTenants   Permission = "ManageTenants"
)

// Claims represents JWT claims
type Claims struct {
	UserID    string `json:"sub"`
	Username  string `json:"username"`
	TenantID  string `json:"tenant_id"`
	Role      Role   `json:"role"`
	IsAPIKey  bool   `json:"is_api_key,omitempty"`
	jwt.StandardClaims
}

// AuthContext holds authentication information for a request
type AuthContext struct {
	UserID   string
	Username string
	TenantID string
	Role     Role
	IsAPIKey bool
}

// AuthClient handles authentication with the core service
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
		return "", errors.New("authorization header missing")
	}

	parts := strings.SplitN(authHeader, " ", 2)
	if len(parts) != 2 || parts[0] != "Bearer" {
		return "", errors.New("invalid authorization header format")
	}

	return parts[1], nil
}

// HasPermission checks if a role has a specific permission
func (r Role) HasPermission(perm Permission) bool {
	switch r {
	case RoleAdmin:
		return true // Admin has all permissions
	case RoleDeveloper:
		switch perm {
		case PermissionRead, PermissionWrite, PermissionMetrics, PermissionManageSchemas, PermissionManagePipelines:
			return true
		}
	case RoleReadOnly:
		return perm == PermissionRead || perm == PermissionMetrics
	case RoleServiceAccount:
		return perm == PermissionRead || perm == PermissionWrite
	}
	return false
}

// AuthMiddleware validates JWT tokens and adds auth context to requests
func AuthMiddleware(authClient *AuthClient) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Skip auth for health endpoints
		if c.Request.URL.Path == "/health" || c.Request.URL.Path == "/metrics" {
			c.Next()
			return
		}

		// Extract token
		token, err := ExtractToken(c)
		if err != nil {
			c.JSON(401, gin.H{"error": "unauthorized", "message": err.Error()})
			c.Abort()
			return
		}

		// Validate token
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
		c.Next()
	}
}

// RequirePermission returns a middleware that checks for a specific permission
func RequirePermission(perm Permission) gin.HandlerFunc {
	return func(c *gin.Context) {
		authCtx, exists := c.Get("auth")
		if !exists {
			c.JSON(401, gin.H{"error": "unauthorized", "message": "authentication required"})
			c.Abort()
			return
		}

		auth := authCtx.(*AuthContext)
		if !auth.Role.HasPermission(perm) {
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
	return RequirePermission(PermissionAdmin)
}

// GetAuthContext retrieves the auth context from the gin context
func GetAuthContext(c *gin.Context) (*AuthContext, error) {
	authCtx, exists := c.Get("auth")
	if !exists {
		return nil, errors.New("no auth context found")
	}

	auth, ok := authCtx.(*AuthContext)
	if !ok {
		return nil, errors.New("invalid auth context")
	}

	return auth, nil
}

// LoginRequest represents a login request
type LoginRequest struct {
	Username string `json:"username" binding:"required"`
	Password string `json:"password" binding:"required"`
}

// LoginResponse represents a login response
type LoginResponse struct {
	Token     string    `json:"token"`
	ExpiresAt time.Time `json:"expires_at"`
	User      UserInfo  `json:"user"`
}

// UserInfo represents user information
type UserInfo struct {
	ID       string `json:"id"`
	Username string `json:"username"`
	Email    string `json:"email"`
	Role     Role   `json:"role"`
	TenantID string `json:"tenant_id"`
}

// RegisterRequest represents a user registration request
type RegisterRequest struct {
	Username string `json:"username" binding:"required"`
	Email    string `json:"email" binding:"required"`
	Password string `json:"password" binding:"required"`
	Role     Role   `json:"role"`
}

// LoginHandler handles user login by proxying to the core service
func (cp *ControlPlane) loginHandler(c *gin.Context) {
	var req LoginRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Proxy to core service
	resp, err := cp.client.R().
		SetBody(req).
		Post("/api/v1/auth/login")

	if err != nil {
		cp.metrics.CoreHealthCheckTotal.WithLabelValues("error").Inc()
		c.JSON(500, gin.H{"error": "failed to authenticate with core service"})
		return
	}

	if resp.StatusCode() != 200 {
		var errResp map[string]interface{}
		json.Unmarshal(resp.Body(), &errResp)
		c.JSON(resp.StatusCode(), errResp)
		return
	}

	var loginResp LoginResponse
	if err := json.Unmarshal(resp.Body(), &loginResp); err != nil {
		c.JSON(500, gin.H{"error": "failed to parse response"})
		return
	}

	c.JSON(200, loginResp)
}

// RegisterHandler handles user registration by proxying to the core service
func (cp *ControlPlane) registerHandler(c *gin.Context) {
	var req RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Default to Developer role if not specified
	if req.Role == "" {
		req.Role = RoleDeveloper
	}

	// Proxy to core service
	resp, err := cp.client.R().
		SetBody(req).
		Post("/api/v1/auth/register")

	if err != nil {
		c.JSON(500, gin.H{"error": "failed to register with core service"})
		return
	}

	if resp.StatusCode() != 201 {
		var errResp map[string]interface{}
		json.Unmarshal(resp.Body(), &errResp)
		c.JSON(resp.StatusCode(), errResp)
		return
	}

	var userResp UserInfo
	if err := json.Unmarshal(resp.Body(), &userResp); err != nil {
		c.JSON(500, gin.H{"error": "failed to parse response"})
		return
	}

	c.JSON(201, userResp)
}

// MeHandler returns the current user information
func (cp *ControlPlane) meHandler(c *gin.Context) {
	auth, err := GetAuthContext(c)
	if err != nil {
		c.JSON(401, gin.H{"error": "unauthorized"})
		return
	}

	c.JSON(200, gin.H{
		"user_id":   auth.UserID,
		"username":  auth.Username,
		"tenant_id": auth.TenantID,
		"role":      auth.Role,
		"is_api_key": auth.IsAPIKey,
	})
}
