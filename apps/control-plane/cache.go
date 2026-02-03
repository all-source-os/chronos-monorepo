package main

import (
	"sync"
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// CachedResponse holds cached data with expiration time.
type CachedResponse struct {
	Data      any
	ExpiresAt time.Time
}

// IsExpired returns true if the cached response has expired.
func (c *CachedResponse) IsExpired() bool {
	return time.Now().After(c.ExpiresAt)
}

// CacheMetrics holds Prometheus metrics for the cache.
type CacheMetrics struct {
	CacheHitsTotal   *prometheus.CounterVec
	CacheMissesTotal *prometheus.CounterVec
	CacheSize        prometheus.Gauge
}

// cacheMetricsInstance holds the singleton cache metrics instance.
var cacheMetricsInstance *CacheMetrics

// NewCacheMetrics creates and registers cache metrics (singleton).
func NewCacheMetrics() *CacheMetrics {
	if cacheMetricsInstance != nil {
		return cacheMetricsInstance
	}
	cacheMetricsInstance = &CacheMetrics{
		CacheHitsTotal: promauto.NewCounterVec(prometheus.CounterOpts{
			Name: "control_plane_cache_hits_total",
			Help: "Total number of cache hits",
		}, []string{"endpoint"}),
		CacheMissesTotal: promauto.NewCounterVec(prometheus.CounterOpts{
			Name: "control_plane_cache_misses_total",
			Help: "Total number of cache misses",
		}, []string{"endpoint"}),
		CacheSize: promauto.NewGauge(prometheus.GaugeOpts{
			Name: "control_plane_cache_size",
			Help: "Current number of items in cache",
		}),
	}
	return cacheMetricsInstance
}

// Cache TTL constants for different endpoints.
const (
	// ClusterStatusCacheTTL is the TTL for cluster status cache entries.
	ClusterStatusCacheTTL = 30 * time.Second
	// MetricsCacheTTL is the TTL for metrics cache entries.
	MetricsCacheTTL = 10 * time.Second
)

// Cache keys for different endpoints.
const (
	CacheKeyClusterStatus = "cluster_status"
	CacheKeyMetrics       = "metrics"
)

// ResponseCache provides thread-safe caching for API responses.
type ResponseCache struct {
	store   sync.Map
	metrics *CacheMetrics
}

// NewResponseCache creates a new response cache with metrics.
func NewResponseCache(metrics *CacheMetrics) *ResponseCache {
	return &ResponseCache{
		metrics: metrics,
	}
}

// Get retrieves a cached response by key. Returns nil if not found or expired.
func (c *ResponseCache) Get(key string) *CachedResponse {
	value, ok := c.store.Load(key)
	if !ok {
		c.metrics.CacheMissesTotal.WithLabelValues(key).Inc()
		return nil
	}

	cached, ok := value.(*CachedResponse)
	if !ok || cached.IsExpired() {
		c.store.Delete(key)
		c.metrics.CacheMissesTotal.WithLabelValues(key).Inc()
		c.updateCacheSize()
		return nil
	}

	c.metrics.CacheHitsTotal.WithLabelValues(key).Inc()
	return cached
}

// Set stores a response in the cache with the specified TTL.
func (c *ResponseCache) Set(key string, data any, ttl time.Duration) {
	cached := &CachedResponse{
		Data:      data,
		ExpiresAt: time.Now().Add(ttl),
	}
	c.store.Store(key, cached)
	c.updateCacheSize()
}

// Invalidate removes a specific key from the cache.
func (c *ResponseCache) Invalidate(key string) {
	c.store.Delete(key)
	c.updateCacheSize()
}

// InvalidateAll removes all entries from the cache.
func (c *ResponseCache) InvalidateAll() {
	c.store.Range(func(key, _ any) bool {
		c.store.Delete(key)
		return true
	})
	c.updateCacheSize()
}

// updateCacheSize updates the cache size metric.
func (c *ResponseCache) updateCacheSize() {
	var count float64
	c.store.Range(func(_, _ any) bool {
		count++
		return true
	})
	c.metrics.CacheSize.Set(count)
}
