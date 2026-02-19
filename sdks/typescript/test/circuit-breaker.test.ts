import { describe, expect, test } from "bun:test";
import { CircuitBreaker } from "../src/circuit-breaker";
import { CircuitOpenError } from "../src/types";

describe("CircuitBreaker", () => {
  test("starts in closed state", () => {
    const cb = new CircuitBreaker();
    expect(cb.state).toBe("closed");
  });

  test("stays closed below threshold", () => {
    const cb = new CircuitBreaker({ threshold: 5 });
    for (let i = 0; i < 4; i++) {
      cb.recordFailure();
    }
    expect(cb.state).toBe("closed");
    cb.check(); // should not throw
  });

  test("opens after threshold consecutive failures", () => {
    const cb = new CircuitBreaker({ threshold: 3 });
    for (let i = 0; i < 3; i++) {
      cb.recordFailure();
    }
    expect(cb.state).toBe("open");
    expect(() => cb.check()).toThrow(CircuitOpenError);
  });

  test("resets to closed on success", () => {
    const cb = new CircuitBreaker({ threshold: 3 });
    cb.recordFailure();
    cb.recordFailure();
    cb.recordSuccess();
    expect(cb.state).toBe("closed");

    // Should need 3 more failures to trip again
    cb.recordFailure();
    cb.recordFailure();
    expect(cb.state).toBe("closed");
    cb.recordFailure();
    expect(cb.state).toBe("open");
  });

  test("transitions to half-open after recovery timeout", () => {
    const cb = new CircuitBreaker({ threshold: 2, recoveryTimeout: 50 });
    cb.recordFailure();
    cb.recordFailure();
    expect(cb.state).toBe("open");

    // Manually wait a bit - we can't easily test timing so we use a workaround
    // by creating a breaker with a very short timeout and checking after a delay
    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(cb.state).toBe("half-open");
        cb.check(); // should not throw in half-open
        resolve();
      }, 60);
    });
  });

  test("half-open resets to closed on success", () => {
    const cb = new CircuitBreaker({ threshold: 2, recoveryTimeout: 10 });
    cb.recordFailure();
    cb.recordFailure();
    expect(cb.state).toBe("open");

    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(cb.state).toBe("half-open");
        cb.recordSuccess();
        expect(cb.state).toBe("closed");
        resolve();
      }, 20);
    });
  });

  test("half-open goes back to open on failure", () => {
    const cb = new CircuitBreaker({ threshold: 2, recoveryTimeout: 10 });
    cb.recordFailure();
    cb.recordFailure();
    expect(cb.state).toBe("open");

    return new Promise<void>((resolve) => {
      setTimeout(() => {
        expect(cb.state).toBe("half-open");
        cb.recordFailure();
        // After one more failure from half-open, the threshold is met again
        // since recordFailure increments consecutiveFailures (now 3 >= 2)
        expect(cb.state).toBe("open");
        resolve();
      }, 20);
    });
  });
});
