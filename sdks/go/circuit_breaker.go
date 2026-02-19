package allsource

import (
	"errors"
	"sync/atomic"
	"time"
)

// CircuitState represents the state of the circuit breaker.
type CircuitState int32

const (
	// CircuitClosed allows requests through normally.
	CircuitClosed CircuitState = iota
	// CircuitOpen rejects all requests immediately.
	CircuitOpen
	// CircuitHalfOpen allows a single probe request.
	CircuitHalfOpen
)

// ErrCircuitOpen is returned when the circuit breaker is open and rejecting requests.
var ErrCircuitOpen = errors.New("allsource: circuit breaker is open")

// CircuitBreaker implements a thread-safe circuit breaker using atomic operations.
type CircuitBreaker struct {
	state               atomic.Int32
	consecutiveFailures atomic.Int32
	threshold           int32
	recoveryTimeout     time.Duration
	lastFailureTime     atomic.Int64 // unix nano
}

// NewCircuitBreaker creates a circuit breaker with the given threshold and recovery timeout.
func NewCircuitBreaker(threshold int, recovery time.Duration) *CircuitBreaker {
	cb := &CircuitBreaker{
		threshold:       int32(threshold),
		recoveryTimeout: recovery,
	}
	cb.state.Store(int32(CircuitClosed))
	return cb
}

// Allow checks whether a request should be allowed through.
// Returns true if the request may proceed.
func (cb *CircuitBreaker) Allow() bool {
	state := CircuitState(cb.state.Load())

	switch state {
	case CircuitClosed:
		return true
	case CircuitOpen:
		lastFail := time.Unix(0, cb.lastFailureTime.Load())
		if time.Since(lastFail) >= cb.recoveryTimeout {
			// Attempt transition to half-open. Only one goroutine wins the CAS.
			if cb.state.CompareAndSwap(int32(CircuitOpen), int32(CircuitHalfOpen)) {
				return true
			}
			// Another goroutine already transitioned; re-check state.
			return CircuitState(cb.state.Load()) == CircuitHalfOpen
		}
		return false
	case CircuitHalfOpen:
		// Only one probe is allowed; subsequent calls are rejected until the probe completes.
		return false
	default:
		return true
	}
}

// RecordSuccess records a successful request. Resets the circuit breaker to Closed.
func (cb *CircuitBreaker) RecordSuccess() {
	cb.consecutiveFailures.Store(0)
	cb.state.Store(int32(CircuitClosed))
}

// RecordFailure records a failed request. May trip the breaker to Open.
func (cb *CircuitBreaker) RecordFailure() {
	failures := cb.consecutiveFailures.Add(1)
	cb.lastFailureTime.Store(time.Now().UnixNano())

	if failures >= cb.threshold {
		cb.state.Store(int32(CircuitOpen))
	}
}

// State returns the current circuit breaker state.
func (cb *CircuitBreaker) State() CircuitState {
	return CircuitState(cb.state.Load())
}

// ConsecutiveFailures returns the current consecutive failure count.
func (cb *CircuitBreaker) ConsecutiveFailures() int {
	return int(cb.consecutiveFailures.Load())
}
