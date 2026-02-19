import { type CircuitBreakerConfig, CircuitOpenError } from "./types";

export type CircuitState = "closed" | "open" | "half-open";

const DEFAULT_CONFIG: CircuitBreakerConfig = {
  threshold: 5,
  recoveryTimeout: 30_000,
};

export class CircuitBreaker {
  private readonly threshold: number;
  private readonly recoveryTimeout: number;
  private consecutiveFailures = 0;
  private lastFailureTime = 0;
  private currentState: CircuitState = "closed";

  constructor(config: Partial<CircuitBreakerConfig> = {}) {
    this.threshold = config.threshold ?? DEFAULT_CONFIG.threshold;
    this.recoveryTimeout = config.recoveryTimeout ?? DEFAULT_CONFIG.recoveryTimeout;
  }

  /** Returns the current circuit state. */
  get state(): CircuitState {
    if (this.currentState === "open") {
      const elapsed = Date.now() - this.lastFailureTime;
      if (elapsed >= this.recoveryTimeout) {
        return "half-open";
      }
    }
    return this.currentState;
  }

  /** Check if a request is allowed. Throws CircuitOpenError if the circuit is open. */
  check(): void {
    const s = this.state;
    if (s === "open") {
      throw new CircuitOpenError();
    }
    // closed or half-open: allow the request through
  }

  /** Record a successful request. Resets the circuit to closed. */
  recordSuccess(): void {
    this.consecutiveFailures = 0;
    this.currentState = "closed";
  }

  /** Record a failed request. Opens the circuit after threshold consecutive failures. */
  recordFailure(): void {
    this.consecutiveFailures++;
    this.lastFailureTime = Date.now();
    if (this.consecutiveFailures >= this.threshold) {
      this.currentState = "open";
    }
  }
}
