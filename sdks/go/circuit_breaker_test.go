package allsource

import (
	"testing"
	"time"
)

func TestCircuitBreakerStartsClosed(t *testing.T) {
	cb := NewCircuitBreaker(5, 30*time.Second)
	if cb.State() != CircuitClosed {
		t.Errorf("expected CircuitClosed, got %v", cb.State())
	}
	if !cb.Allow() {
		t.Error("expected Allow() to return true in Closed state")
	}
}

func TestCircuitBreakerTripsAfterThreshold(t *testing.T) {
	cb := NewCircuitBreaker(3, 30*time.Second)

	// Two failures: still closed.
	cb.RecordFailure()
	cb.RecordFailure()
	if cb.State() != CircuitClosed {
		t.Errorf("expected CircuitClosed after 2 failures, got %v", cb.State())
	}
	if !cb.Allow() {
		t.Error("expected Allow() after 2 failures")
	}

	// Third failure: trips to open.
	cb.RecordFailure()
	if cb.State() != CircuitOpen {
		t.Errorf("expected CircuitOpen after 3 failures, got %v", cb.State())
	}
	if cb.Allow() {
		t.Error("expected Allow() to return false in Open state")
	}
}

func TestCircuitBreakerResetsOnSuccess(t *testing.T) {
	cb := NewCircuitBreaker(3, 30*time.Second)

	cb.RecordFailure()
	cb.RecordFailure()
	if cb.ConsecutiveFailures() != 2 {
		t.Errorf("expected 2 failures, got %d", cb.ConsecutiveFailures())
	}

	cb.RecordSuccess()
	if cb.ConsecutiveFailures() != 0 {
		t.Errorf("expected 0 failures after success, got %d", cb.ConsecutiveFailures())
	}
	if cb.State() != CircuitClosed {
		t.Errorf("expected CircuitClosed after success, got %v", cb.State())
	}
}

func TestCircuitBreakerHalfOpenAfterTimeout(t *testing.T) {
	cb := NewCircuitBreaker(2, 50*time.Millisecond)

	// Trip the breaker.
	cb.RecordFailure()
	cb.RecordFailure()
	if cb.State() != CircuitOpen {
		t.Fatal("expected CircuitOpen")
	}
	if cb.Allow() {
		t.Error("expected Allow()=false immediately after opening")
	}

	// Wait for recovery timeout.
	time.Sleep(60 * time.Millisecond)

	// Should transition to HalfOpen and allow one probe.
	if !cb.Allow() {
		t.Error("expected Allow()=true after recovery timeout")
	}
	if cb.State() != CircuitHalfOpen {
		t.Errorf("expected CircuitHalfOpen, got %v", cb.State())
	}

	// Second call in HalfOpen should be rejected (only one probe allowed).
	if cb.Allow() {
		t.Error("expected Allow()=false for second call in HalfOpen")
	}
}

func TestCircuitBreakerHalfOpenSuccessCloses(t *testing.T) {
	cb := NewCircuitBreaker(2, 50*time.Millisecond)

	cb.RecordFailure()
	cb.RecordFailure()
	time.Sleep(60 * time.Millisecond)

	cb.Allow() // transitions to HalfOpen
	cb.RecordSuccess()

	if cb.State() != CircuitClosed {
		t.Errorf("expected CircuitClosed after HalfOpen success, got %v", cb.State())
	}
	if !cb.Allow() {
		t.Error("expected Allow()=true after close")
	}
}

func TestCircuitBreakerHalfOpenFailureReopens(t *testing.T) {
	cb := NewCircuitBreaker(2, 50*time.Millisecond)

	cb.RecordFailure()
	cb.RecordFailure()
	time.Sleep(60 * time.Millisecond)

	cb.Allow() // transitions to HalfOpen
	cb.RecordFailure()

	if cb.State() != CircuitOpen {
		t.Errorf("expected CircuitOpen after HalfOpen failure, got %v", cb.State())
	}
}
