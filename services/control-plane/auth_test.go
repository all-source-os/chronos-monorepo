package main

import (
	"testing"
	"time"

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
		Role:     RoleDeveloper,
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
			Role:     RoleDeveloper,
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
		role       Role
		permission Permission
		expected   bool
	}{
		// Admin has all permissions
		{RoleAdmin, PermissionRead, true},
		{RoleAdmin, PermissionWrite, true},
		{RoleAdmin, PermissionAdmin, true},
		{RoleAdmin, PermissionMetrics, true},
		{RoleAdmin, PermissionManageSchemas, true},
		{RoleAdmin, PermissionManagePipelines, true},
		{RoleAdmin, PermissionManageTenants, true},

		// Developer permissions
		{RoleDeveloper, PermissionRead, true},
		{RoleDeveloper, PermissionWrite, true},
		{RoleDeveloper, PermissionAdmin, false},
		{RoleDeveloper, PermissionMetrics, true},
		{RoleDeveloper, PermissionManageSchemas, true},
		{RoleDeveloper, PermissionManagePipelines, true},
		{RoleDeveloper, PermissionManageTenants, false},

		// ReadOnly permissions
		{RoleReadOnly, PermissionRead, true},
		{RoleReadOnly, PermissionWrite, false},
		{RoleReadOnly, PermissionAdmin, false},
		{RoleReadOnly, PermissionMetrics, true},
		{RoleReadOnly, PermissionManageSchemas, false},
		{RoleReadOnly, PermissionManagePipelines, false},
		{RoleReadOnly, PermissionManageTenants, false},

		// ServiceAccount permissions
		{RoleServiceAccount, PermissionRead, true},
		{RoleServiceAccount, PermissionWrite, true},
		{RoleServiceAccount, PermissionAdmin, false},
		{RoleServiceAccount, PermissionMetrics, false},
		{RoleServiceAccount, PermissionManageSchemas, false},
		{RoleServiceAccount, PermissionManagePipelines, false},
		{RoleServiceAccount, PermissionManageTenants, false},
	}

	for _, tt := range tests {
		t.Run(string(tt.role)+"_"+string(tt.permission), func(t *testing.T) {
			result := tt.role.HasPermission(tt.permission)
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
