package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	"github.com/dgrijalva/jwt-go"

	"github.com/allsource/control-plane/internal/domain/entities"
)

func TestAuthClient_ValidateToken(t *testing.T) {
	secret := "test-secret-key"
	authClient := NewAuthClient(secret, "")

	// Create a valid token
	claims := &Claims{
		UserID:   "user-123",
		Username: "testuser",
		TenantID: "default",
		Role:     entities.RoleDeveloper,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: time.Now().Add(time.Hour).Unix(),
			IssuedAt:  time.Now().Unix(),
			Issuer:    "allsource",
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
				Issuer:    "allsource",
			},
		}

		expiredToken := jwt.NewWithClaims(jwt.SigningMethodHS256, expiredClaims)
		expiredTokenString, _ := expiredToken.SignedString([]byte(secret)) //nolint:errcheck // test code

		_, err := authClient.ValidateToken(expiredTokenString)
		if err == nil {
			t.Error("Expired token should have been rejected")
		}
	})

	// Test: Invalid signature should be rejected
	t.Run("InvalidSignature", func(t *testing.T) {
		wrongSecret := "wrong-secret"
		wrongToken := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
		wrongTokenString, _ := wrongToken.SignedString([]byte(wrongSecret)) //nolint:errcheck // test code

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

func TestAuthClient_ValidateAPIKey(t *testing.T) {
	// Spin up a fake Core that impersonates /api/v1/auth/me.
	var hits int64
	core := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		atomic.AddInt64(&hits, 1)
		auth := r.Header.Get("Authorization")
		if auth != "Bearer ask_good-key" {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"id":"00000000-0000-0000-0000-000000000000","username":"alice","email":"alice@example.com","role":"developer","tenant_id":"tenant-alice"}`)) //nolint:errcheck // test response
	}))
	defer core.Close()

	client := NewAuthClient("secret", core.URL)

	t.Run("valid ask_ key returns claims", func(t *testing.T) {
		claims, err := client.ValidateAPIKey(context.Background(), "ask_good-key")
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if claims.TenantID != "tenant-alice" {
			t.Errorf("tenant_id: got %q, want %q", claims.TenantID, "tenant-alice")
		}
		if claims.Role != entities.RoleDeveloper {
			t.Errorf("role: got %q, want %q", claims.Role, entities.RoleDeveloper)
		}
		if !claims.IsAPIKey {
			t.Errorf("IsAPIKey: got false, want true")
		}
	})

	t.Run("second call hits cache, not Core", func(t *testing.T) {
		before := atomic.LoadInt64(&hits)
		if _, err := client.ValidateAPIKey(context.Background(), "ask_good-key"); err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if after := atomic.LoadInt64(&hits); after != before {
			t.Errorf("expected cache hit, but Core was called (hits %d -> %d)", before, after)
		}
	})

	t.Run("bad key surfaces Core error", func(t *testing.T) {
		if _, err := client.ValidateAPIKey(context.Background(), "ask_bogus"); err == nil {
			t.Error("expected error for bogus key, got nil")
		}
	})

	t.Run("coreURL unset returns error without HTTP call", func(t *testing.T) {
		unconfigured := NewAuthClient("secret", "")
		_, err := unconfigured.ValidateAPIKey(context.Background(), "ask_whatever")
		if err == nil {
			t.Error("expected error when coreURL unset, got nil")
		}
	})
}

func TestAuthClient_RememberAPIKey_BypassesCoreMe(t *testing.T) {
	// Fake Core that would panic if called — proving the cached key path
	// never round-trips to /me.
	coreCalled := int64(0)
	core := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		atomic.AddInt64(&coreCalled, 1)
		http.Error(w, "should not be called", http.StatusTeapot)
	}))
	defer core.Close()

	client := NewAuthClient("secret", core.URL)

	// Mint a key via RememberAPIKey (simulates provisionCoreAPIKey's
	// post-CreateCoreAPIKey step).
	remembered := &Claims{
		UserID:   "user-xyz",
		TenantID: "tenant-xyz",
		Role:     entities.RoleServiceAccount,
		IsAPIKey: true,
	}
	client.RememberAPIKey("ask_freshly_minted", remembered)

	t.Run("validate returns cached claims without hitting Core", func(t *testing.T) {
		claims, err := client.ValidateAPIKey(context.Background(), "ask_freshly_minted")
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		if claims.TenantID != "tenant-xyz" {
			t.Errorf("tenant_id: got %q, want tenant-xyz", claims.TenantID)
		}
		if atomic.LoadInt64(&coreCalled) != 0 {
			t.Errorf("Core /me was called %d times — should be zero for remembered keys", coreCalled)
		}
	})

	t.Run("repeated calls never expire remembered entry", func(t *testing.T) {
		// Pre-existing /me-resolved entries expire after 120s; remembered
		// entries should not. We can't time-travel in a unit test, but we
		// can assert expiresAt.IsZero() semantics hold by re-calling many
		// times and confirming Core is never touched.
		for i := 0; i < 10; i++ {
			if _, err := client.ValidateAPIKey(context.Background(), "ask_freshly_minted"); err != nil {
				t.Fatalf("call %d: %v", i, err)
			}
		}
		if atomic.LoadInt64(&coreCalled) != 0 {
			t.Errorf("Core /me was called %d times during 10 cached calls", coreCalled)
		}
	})

	t.Run("unknown keys still fall back to Core /me", func(t *testing.T) {
		// Fresh client so the cached entry doesn't interfere.
		otherCalled := int64(0)
		other := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			atomic.AddInt64(&otherCalled, 1)
			w.Header().Set("Content-Type", "application/json")
			_, _ = w.Write([]byte(`{"id":"00000000-0000-0000-0000-000000000000","username":"legacy","email":"","role":"developer","tenant_id":"legacy-tenant"}`)) //nolint:errcheck // test response
		}))
		defer other.Close()

		legacyClient := NewAuthClient("secret", other.URL)
		claims, err := legacyClient.ValidateAPIKey(context.Background(), "ask_legacy_not_cached")
		if err != nil {
			t.Fatalf("legacy key path broken: %v", err)
		}
		if claims.TenantID != "legacy-tenant" {
			t.Errorf("tenant_id: got %q, want legacy-tenant", claims.TenantID)
		}
		if atomic.LoadInt64(&otherCalled) != 1 {
			t.Errorf("Core /me called %d times, want 1", otherCalled)
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
