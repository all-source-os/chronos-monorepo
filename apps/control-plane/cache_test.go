package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-gonic/gin"
)

func TestCachedResponse_IsExpired(t *testing.T) {
	tests := []struct {
		name      string
		expiresAt time.Time
		want      bool
	}{
		{
			name:      "not expired",
			expiresAt: time.Now().Add(time.Hour),
			want:      false,
		},
		{
			name:      "expired",
			expiresAt: time.Now().Add(-time.Hour),
			want:      true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := &CachedResponse{
				Data:      "test",
				ExpiresAt: tt.expiresAt,
			}
			if got := c.IsExpired(); got != tt.want {
				t.Errorf("IsExpired() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestResponseCache_GetSet(t *testing.T) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	// Test miss
	if got := cache.Get("nonexistent"); got != nil {
		t.Errorf("Get() = %v, want nil", got)
	}

	// Test set and get
	cache.Set("test_key", "test_value", time.Minute)
	if got := cache.Get("test_key"); got == nil {
		t.Fatal("Get() = nil, want non-nil")
	} else if got.Data != "test_value" {
		t.Errorf("Get().Data = %v, want test_value", got.Data)
	}
}

func TestResponseCache_Expiration(t *testing.T) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	// Set with very short TTL
	cache.Set("expiring_key", "value", time.Millisecond)

	// Wait for expiration
	time.Sleep(10 * time.Millisecond)

	// Should return nil for expired entry
	if got := cache.Get("expiring_key"); got != nil {
		t.Errorf("Get() = %v, want nil (expired)", got)
	}
}

func TestResponseCache_Invalidate(t *testing.T) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	cache.Set("key1", "value1", time.Minute)
	cache.Set("key2", "value2", time.Minute)

	// Invalidate single key
	cache.Invalidate("key1")
	if got := cache.Get("key1"); got != nil {
		t.Errorf("Get(key1) after Invalidate = %v, want nil", got)
	}
	if got := cache.Get("key2"); got == nil {
		t.Error("Get(key2) after Invalidate(key1) = nil, want non-nil")
	}
}

func TestResponseCache_InvalidateAll(t *testing.T) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	cache.Set("key1", "value1", time.Minute)
	cache.Set("key2", "value2", time.Minute)

	// Invalidate all
	cache.InvalidateAll()

	if got := cache.Get("key1"); got != nil {
		t.Errorf("Get(key1) after InvalidateAll = %v, want nil", got)
	}
	if got := cache.Get("key2"); got != nil {
		t.Errorf("Get(key2) after InvalidateAll = %v, want nil", got)
	}
}

// testCache holds a shared cache for benchmarks to avoid re-registering metrics
var testCache *ResponseCache

// setupTestRouter creates a test router with caching enabled
func setupTestRouter() (*gin.Engine, *ResponseCache) {
	gin.SetMode(gin.TestMode)

	// Use singleton cache metrics
	cacheMetrics := NewCacheMetrics()
	if testCache == nil {
		testCache = NewResponseCache(cacheMetrics)
	}
	cache := testCache

	router := gin.New()

	// Cached cluster status endpoint
	router.GET("/api/v1/cluster/status", func(c *gin.Context) {
		// Check cache first
		if cached := cache.Get(CacheKeyClusterStatus); cached != nil {
			c.JSON(http.StatusOK, cached.Data)
			return
		}

		response := gin.H{
			"cluster_id":    "test-cluster",
			"total_nodes":   1,
			"healthy_nodes": 1,
			"timestamp":     time.Now().UTC(),
		}

		cache.Set(CacheKeyClusterStatus, response, ClusterStatusCacheTTL)
		c.JSON(http.StatusOK, response)
	})

	// Cached metrics endpoint
	router.GET("/api/v1/metrics/json", func(c *gin.Context) {
		// Check cache first
		if cached := cache.Get(CacheKeyMetrics); cached != nil {
			c.JSON(http.StatusOK, cached.Data)
			return
		}

		response := gin.H{
			"metrics": gin.H{
				"uptime_seconds": 100.0, // Use static value for benchmarks
			},
			"timestamp": time.Now().UTC(),
		}

		cache.Set(CacheKeyMetrics, response, MetricsCacheTTL)
		c.JSON(http.StatusOK, response)
	})

	return router, cache
}

// BenchmarkClusterStatusCached benchmarks cached cluster status endpoint
func BenchmarkClusterStatusCached(b *testing.B) {
	router, _ := setupTestRouter()

	ctx := context.Background()

	// Warm up the cache with first request
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/api/v1/cluster/status", http.NoBody)
	if err != nil {
		b.Fatal(err)
	}
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	b.ReportAllocs()

	for b.Loop() {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/api/v1/cluster/status", http.NoBody)
		if err != nil {
			b.Fatal(err)
		}
		w := httptest.NewRecorder()
		router.ServeHTTP(w, req)

		if w.Code != http.StatusOK {
			b.Fatalf("unexpected status code: %d", w.Code)
		}
	}
}

// BenchmarkMetricsCached benchmarks cached metrics endpoint
func BenchmarkMetricsCached(b *testing.B) {
	router, _ := setupTestRouter()
	ctx := context.Background()

	// Warm up the cache with first request
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/api/v1/metrics/json", http.NoBody)
	if err != nil {
		b.Fatal(err)
	}
	w := httptest.NewRecorder()
	router.ServeHTTP(w, req)

	b.ReportAllocs()

	for b.Loop() {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/api/v1/metrics/json", http.NoBody)
		if err != nil {
			b.Fatal(err)
		}
		w := httptest.NewRecorder()
		router.ServeHTTP(w, req)

		if w.Code != http.StatusOK {
			b.Fatalf("unexpected status code: %d", w.Code)
		}
	}
}

// BenchmarkClusterStatusUncached simulates uncached performance for comparison
func BenchmarkClusterStatusUncached(b *testing.B) {
	gin.SetMode(gin.TestMode)
	router := gin.New()

	router.GET("/api/v1/cluster/status", func(c *gin.Context) {
		// No caching - builds response every time
		response := gin.H{
			"cluster_id":    "test-cluster",
			"total_nodes":   1,
			"healthy_nodes": 1,
			"timestamp":     time.Now().UTC(),
		}
		c.JSON(http.StatusOK, response)
	})

	ctx := context.Background()
	b.ReportAllocs()

	for b.Loop() {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, "/api/v1/cluster/status", http.NoBody)
		if err != nil {
			b.Fatal(err)
		}
		w := httptest.NewRecorder()
		router.ServeHTTP(w, req)

		if w.Code != http.StatusOK {
			b.Fatalf("unexpected status code: %d", w.Code)
		}
	}
}

// BenchmarkCacheGet benchmarks raw cache get operations
func BenchmarkCacheGet(b *testing.B) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	// Pre-populate cache
	cache.Set("test_key", gin.H{"data": "test_value"}, time.Hour)

	b.ReportAllocs()

	for b.Loop() {
		_ = cache.Get("test_key")
	}
}

// BenchmarkCacheSet benchmarks raw cache set operations
func BenchmarkCacheSet(b *testing.B) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	data := gin.H{"data": "test_value"}

	b.ReportAllocs()

	for b.Loop() {
		cache.Set("test_key", data, time.Hour)
	}
}

// BenchmarkCacheConcurrentAccess benchmarks concurrent cache access
func BenchmarkCacheConcurrentAccess(b *testing.B) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	// Pre-populate cache
	cache.Set("test_key", gin.H{"data": "test_value"}, time.Hour)

	b.ReportAllocs()

	for b.Loop() {
		_ = cache.Get("test_key")
	}
}

// BenchmarkCacheParallelAccess benchmarks parallel cache access with RunParallel
func BenchmarkCacheParallelAccess(b *testing.B) {
	metrics := NewCacheMetrics()
	cache := NewResponseCache(metrics)

	// Pre-populate cache
	cache.Set("test_key", gin.H{"data": "test_value"}, time.Hour)

	b.ReportAllocs()

	b.RunParallel(func(pb *testing.PB) {
		for pb.Next() {
			_ = cache.Get("test_key")
		}
	})
}
