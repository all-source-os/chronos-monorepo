package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"
)

// Claims represents JWT claims
type Claims struct {
	UserID    string        `json:"sub"`
	Username  string        `json:"username"`
	TenantID  string        `json:"tenant_id"`
	Role      entities.Role `json:"role"`
	IsAPIKey  bool          `json:"is_api_key,omitempty"`
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
		return "", errors.New("no authorization header")
	}

	// Expected format: "Bearer <token>"
	parts := strings.SplitN(authHeader, " ", 2)
	if len(parts) != 2 || parts[0] != "Bearer" {
		return "", errors.New("invalid authorization header format")
	}

	return parts[1], nil
}

// RoleHasPermission checks if a role has a specific permission
func RoleHasPermission(role entities.Role, perm entities.Permission) bool {
	user, err := entities.NewUser("temp", "temp", "temp", role)
	if err != nil {
		return false
	}
	return user.HasPermission(perm)
}

// AuthMiddleware validates JWT tokens and adds auth context to requests
func AuthMiddleware(authClient *AuthClient) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Skip auth for health endpoints
		if c.Request.URL.Path == "/health" || c.Request.URL.Path == "/metrics" {
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

		auth := authCtx.(*AuthContext)
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

// LoginRequest represents a login request
type LoginRequest struct {
	Username string `json:"username" binding:"required"`
	Password string `json:"password" binding:"required"`
}

// RegisterRequest represents a registration request
type RegisterRequest struct {
	Username string        `json:"username" binding:"required"`
	Password string        `json:"password" binding:"required"`
	TenantID string        `json:"tenant_id"`
	Role     entities.Role `json:"role"`
}

// LoginHandler handles user login
func (cp *ControlPlane) LoginHandler(c *gin.Context) {
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
		c.JSON(500, gin.H{"error": "failed to authenticate", "message": err.Error()})
		return
	}

	// Parse response
	var result map[string]interface{}
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		c.JSON(500, gin.H{"error": "invalid response from core service"})
		return
	}

	c.JSON(resp.StatusCode(), result)
}

// RegisterHandler handles user registration
func (cp *ControlPlane) RegisterHandler(c *gin.Context) {
	var req RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(400, gin.H{"error": "invalid request", "message": err.Error()})
		return
	}

	// Default to Developer role if not specified
	if req.Role == "" {
		req.Role = entities.RoleDeveloper
	}

	// Proxy to core service
	resp, err := cp.client.R().
		SetBody(req).
		Post("/api/v1/auth/register")

	if err != nil {
		c.JSON(500, gin.H{"error": "registration failed", "message": err.Error()})
		return
	}

	// Parse response
	var result map[string]interface{}
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		c.JSON(500, gin.H{"error": "invalid response from core service"})
		return
	}

	c.JSON(resp.StatusCode(), result)
}
