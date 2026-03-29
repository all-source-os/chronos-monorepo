package x402

import (
	"sync"
	"time"
)

// NonceTracker prevents replay attacks by tracking used nonces with TTL expiry.
// Thread-safe for concurrent use by HTTP handlers.
type NonceTracker struct {
	mu     sync.Mutex
	nonces map[string]time.Time
	ttl    time.Duration
}

// NewNonceTracker creates a NonceTracker with the given TTL for nonce expiry.
func NewNonceTracker(ttl time.Duration) *NonceTracker {
	return &NonceTracker{
		nonces: make(map[string]time.Time),
		ttl:    ttl,
	}
}

// Use attempts to mark a nonce as used. Returns false if already used (replay).
func (nt *NonceTracker) Use(nonce string) bool {
	nt.mu.Lock()
	defer nt.mu.Unlock()

	// Lazy cleanup of expired nonces
	now := time.Now()
	for k, expiry := range nt.nonces {
		if now.After(expiry) {
			delete(nt.nonces, k)
		}
	}

	if _, exists := nt.nonces[nonce]; exists {
		return false // replay
	}

	nt.nonces[nonce] = now.Add(nt.ttl)
	return true
}

// IsUsed checks if a nonce has been used without marking it.
func (nt *NonceTracker) IsUsed(nonce string) bool {
	nt.mu.Lock()
	defer nt.mu.Unlock()

	expiry, exists := nt.nonces[nonce]
	if !exists {
		return false
	}
	return time.Now().Before(expiry)
}

// Size returns the number of tracked nonces (including expired, before cleanup).
func (nt *NonceTracker) Size() int {
	nt.mu.Lock()
	defer nt.mu.Unlock()
	return len(nt.nonces)
}
