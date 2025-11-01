# AllSource Core - Examples

This directory contains comprehensive examples demonstrating AllSource Core's advanced security features.

## Available Examples

### 1. Advanced Security Demo (`advanced_security_demo.rs`)

An interactive demo showcasing all enterprise-grade security features:

- **ML-Based Anomaly Detection**: Real-time threat detection with behavioral analysis
- **Field-Level Encryption**: AES-256-GCM encryption with key rotation
- **HSM/KMS Integration**: Enterprise key management (AWS, GCP, Azure, Vault)
- **Adaptive Rate Limiting**: ML-based automatic limit adjustment
- **Security Automation**: CI/CD security scanning integration

#### Running the Demo

```bash
# Run the interactive demo
cargo run --example advanced_security_demo

# The demo will walk you through:
# 1. Detecting brute force attacks and data exfiltration
# 2. Encrypting sensitive data with automatic key rotation
# 3. Using KMS for envelope encryption
# 4. Adaptive rate limiting with system load awareness
# 5. Automated security scanning for CI/CD
```

#### Demo Scenarios

**Scenario 1: Brute Force Detection**
- Simulates 6 failed login attempts
- Demonstrates ML-based detection
- Shows recommended actions (Block, Alert, Monitor)

**Scenario 2: Data Exfiltration**
- Simulates unusual high-volume queries
- Detects 5x above normal rate
- Triggers automatic alerts

**Scenario 3: Field Encryption**
- Encrypts SSN, credit cards, API keys
- Demonstrates key rotation
- Shows backward compatibility

**Scenario 4: KMS Integration**
- Creates master encryption keys
- Performs envelope encryption
- Demonstrates key rotation

**Scenario 5: Adaptive Rate Limiting**
- Normal usage pattern learning
- System load-based adjustment
- Traffic spike detection

**Scenario 6: Security Automation**
- Dependency vulnerability scanning
- Secret detection in code
- SAST with clippy
- CI/CD workflow generation

## Demo Output Example

```
╔═══════════════════════════════════════════════════════════════╗
║      AllSource Core - Advanced Security Features Demo        ║
║                                                               ║
║  Enterprise-Grade Security:                                  ║
║  ✓ ML-Based Anomaly Detection                                ║
║  ✓ Field-Level Encryption (AES-256-GCM)                     ║
║  ✓ HSM/KMS Integration (AWS, GCP, Azure, Vault)             ║
║  ✓ Adaptive Rate Limiting (ML-Based)                        ║
║  ✓ Security Automation (CI/CD Scanning)                     ║
╚═══════════════════════════════════════════════════════════════╝

🔍 Initializing anomaly detector with ML-based behavioral analysis...
✓ Anomaly detector initialized

📊 Scenario 1: Detecting Brute Force Attack
   Simulating 6 failed login attempts in 2 minutes...
   └─ Failed login attempt #1
   └─ Failed login attempt #2
   ...

   ⚠️  ANOMALY DETECTED!
   ├─ Type: BruteForceAttack
   ├─ Confidence Score: 60.0%
   ├─ Reason: Anomalous BruteForceAttack detected with score 0.60
   └─ Recommended Action: Alert
   📢 Action: SENDING ALERT
```

## Integration Examples

### Using in Your Application

```rust
use allsource_core::security::{
    AnomalyDetector, FieldEncryption, KmsManager,
    AdaptiveRateLimiter, SecurityScanner,
};

// Initialize all security components
let anomaly_detector = AnomalyDetector::new(config);
let encryption = FieldEncryption::new(enc_config)?;
let kms = KmsManager::new(kms_config)?;
let rate_limiter = AdaptiveRateLimiter::new(rate_config);
let scanner = SecurityScanner::new(scan_config);

// Use in request handling
async fn handle_request(request: Request) -> Response {
    // Check rate limit
    let rate_result = rate_limiter.check_adaptive_limit(&tenant_id)?;
    if !rate_result.allowed {
        return Response::rate_limited();
    }

    // Analyze for anomalies
    let anomaly_result = anomaly_detector.analyze_event(&audit_event)?;
    if anomaly_result.is_anomalous {
        alert_security_team(&anomaly_result);
    }

    // Encrypt sensitive response data
    let encrypted = encryption.encrypt_string(&sensitive_data, "field_name")?;

    Response::ok(encrypted)
}
```

## Requirements

- Rust 1.70+
- Tokio async runtime
- Optional: cargo-audit for dependency scanning

## Next Steps

After running the demo:

1. **Review the source code** to understand implementation details
2. **Check SECURITY.md** for comprehensive security documentation
3. **Run tests** with `cargo test --lib security::`
4. **Integrate** security features into your application
5. **Configure** for production environment

## Support

For questions or issues:
- Open an issue on GitHub
- Check documentation in `docs/`
- Review SECURITY.md for security guidelines
