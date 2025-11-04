package main

import (
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/dgrijalva/jwt-go"
)

func TestAuthClient_ValidateToken(t *testing.T) {
	secret := "test-secret-key"
	authClient := NewAuthClient(secret)

	// Create a valid token
	claims := &Claims{
		UserID:   "user-123",
		Username: "testuser",
		TenantID: "default",
		Role:     entities.RoleDeveloper,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: time.Now().Add(time.Hour).Unix(),
			IssuedAt:  time.Now().Unix(),
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenString, err := token.SignedString([]byte(secret))
	if err != nil {
		t.Fatalf("Failed to create token: %v", err)
	}

	// Test: Valid token should be accepted
	t.Run("ValidToken", func(t *testing.T) {
		validatedClaims, err := authClient.ValidateToken(tokenString)
		if err != nil {
			t.Errorf("Valid token was rejected: %v", err)
		}
		if validatedClaims.UserID != claims.UserID {
			t.Errorf("UserID mismatch: expected %s, got %s", claims.UserID, validatedClaims.UserID)
		}
		if validatedClaims.TenantID != claims.TenantID {
			t.Errorf("TenantID mismatch: expected %s, got %s", claims.TenantID, validatedClaims.TenantID)
		}
		if validatedClaims.Role != claims.Role {
			t.Errorf("Role mismatch: expected %s, got %s", claims.Role, validatedClaims.Role)
		}
	})

	// Test: Expired token should be rejected
	t.Run("ExpiredToken", func(t *testing.T) {
		expiredClaims := &Claims{
			UserID:   "user-123",
			Username: "testuser",
			TenantID: "default",
			Role:     entities.RoleDeveloper,
			StandardClaims: jwt.StandardClaims{
				ExpiresAt: time.Now().Add(-time.Hour).Unix(), // Already expired
				IssuedAt:  time.Now().Unix(),
			},
		}

		expiredToken := jwt.NewWithClaims(jwt.SigningMethodHS256, expiredClaims)
		expiredTokenString, _ := expiredToken.SignedString([]byte(secret))

		_, err := authClient.ValidateToken(expiredTokenString)
		if err == nil {
			t.Error("Expired token should have been rejected")
		}
	})

	// Test: Invalid signature should be rejected
	t.Run("InvalidSignature", func(t *testing.T) {
		wrongSecret := "wrong-secret"
		wrongToken := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
		wrongTokenString, _ := wrongToken.SignedString([]byte(wrongSecret))

		_, err := authClient.ValidateToken(wrongTokenString)
		if err == nil {
			t.Error("Token with wrong signature should have been rejected")
		}
	})

	// Test: Malformed token should be rejected
	t.Run("MalformedToken", func(t *testing.T) {
		_, err := authClient.ValidateToken("not-a-valid-token")
		if err == nil {
			t.Error("Malformed token should have been rejected")
		}
	})
}

func TestRole_HasPermission(t *testing.T) {
	tests := []struct {
		role       entities.Role
		permission entities.Permission
		expected   bool
	}{
		// Admin has all permissions
		{entities.RoleAdmin, entities.PermissionRead, true},
		{entities.RoleAdmin, entities.PermissionWrite, true},
		{entities.RoleAdmin, entities.PermissionAdmin, true},
		{entities.RoleAdmin, entities.PermissionMetrics, true},
		{entities.RoleAdmin, entities.PermissionManageSchemas, true},
		{entities.RoleAdmin, entities.PermissionManagePipelines, true},
		{entities.RoleAdmin, entities.PermissionManageTenants, true},

		// Developer permissions
		{entities.RoleDeveloper, entities.PermissionRead, true},
		{entities.RoleDeveloper, entities.PermissionWrite, true},
		{entities.RoleDeveloper, entities.PermissionAdmin, false},
		{entities.RoleDeveloper, entities.PermissionMetrics, true},
		{entities.RoleDeveloper, entities.PermissionManageSchemas, true},
		{entities.RoleDeveloper, entities.PermissionManagePipelines, true},
		{entities.RoleDeveloper, entities.PermissionManageTenants, false},

		// ReadOnly permissions
		{entities.RoleReadOnly, entities.PermissionRead, true},
		{entities.RoleReadOnly, entities.PermissionWrite, false},
		{entities.RoleReadOnly, entities.PermissionAdmin, false},
		{entities.RoleReadOnly, entities.PermissionMetrics, true},
		{entities.RoleReadOnly, entities.PermissionManageSchemas, false},
		{entities.RoleReadOnly, entities.PermissionManagePipelines, false},
		{entities.RoleReadOnly, entities.PermissionManageTenants, false},

		// ServiceAccount permissions
		{entities.RoleServiceAccount, entities.PermissionRead, true},
		{entities.RoleServiceAccount, entities.PermissionWrite, true},
		{entities.RoleServiceAccount, entities.PermissionAdmin, false},
		{entities.RoleServiceAccount, entities.PermissionMetrics, false},
		{entities.RoleServiceAccount, entities.PermissionManageSchemas, false},
		{entities.RoleServiceAccount, entities.PermissionManagePipelines, false},
		{entities.RoleServiceAccount, entities.PermissionManageTenants, false},
	}

	for _, tt := range tests {
		t.Run(string(tt.role)+"_"+string(tt.permission), func(t *testing.T) {
			result := RoleHasPermission(tt.role, tt.permission)
			if result != tt.expected {
				t.Errorf("Role %s permission %s: expected %v, got %v",
					tt.role, tt.permission, tt.expected, result)
			}
		})
	}
}

func TestExtractToken(t *testing.T) {
	// This test would require creating a gin.Context which is complex
	// Skipping for now, but this is a good candidate for integration tests
	t.Skip("Requires gin.Context setup")
}

func TestGetAuthContext(t *testing.T) {
	// This test would require creating a gin.Context
	// Skipping for now, but this is a good candidate for integration tests
	t.Skip("Requires gin.Context setup")
}
