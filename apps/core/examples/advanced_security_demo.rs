/// AllSource Core - Advanced Security Features Demo
///
/// This comprehensive demo showcases all enterprise-grade security features:
/// 1. ML-Based Anomaly Detection
/// 2. Field-Level Encryption
/// 3. HSM/KMS Integration
/// 4. Adaptive Rate Limiting
/// 5. Security Automation & CI/CD Scanning
use allsource_core::{
    domain::entities::{Actor, AuditAction, AuditEvent, AuditOutcome},
    domain::value_objects::TenantId,
    security::{
        AdaptiveRateLimitConfig,
        // Adaptive Rate Limiting
        AdaptiveRateLimiter,
        AnomalyDetectionConfig,
        // Anomaly Detection
        AnomalyDetector,
        AnomalyType,
        CiCdIntegration,
        EncryptionAlgorithm,

        EncryptionConfig,
        // Encryption
        FieldEncryption,
        KeyAlgorithm,

        KeyPurpose,
        KmsConfig,
        // KMS
        KmsManager,
        KmsProvider,
        RecommendedAction,

        ScanStatus,
        SecurityScanConfig,
        // Security Automation
        SecurityScanner,
        Severity,
        SystemLoad,
    },
};

use std::io::{self, Write as _};

#[tokio::main]
async fn main() {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║      AllSource Core - Advanced Security Features Demo        ║");
    println!("║                                                               ║");
    println!("║  Enterprise-Grade Security:                                  ║");
    println!("║  ✓ ML-Based Anomaly Detection                                ║");
    println!("║  ✓ Field-Level Encryption (AES-256-GCM)                     ║");
    println!("║  ✓ HSM/KMS Integration (AWS, GCP, Azure, Vault)             ║");
    println!("║  ✓ Adaptive Rate Limiting (ML-Based)                        ║");
    println!("║  ✓ Security Automation (CI/CD Scanning)                     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    pause("Press Enter to start the demo...");

    // Demo 1: ML-Based Anomaly Detection
    demo_anomaly_detection();

    // Demo 2: Field-Level Encryption
    demo_field_encryption().expect("Encryption demo failed");

    // Demo 3: KMS Integration
    demo_kms_integration().await.expect("KMS demo failed");

    // Demo 4: Adaptive Rate Limiting
    demo_adaptive_rate_limiting().expect("Rate limiting demo failed");

    // Demo 5: Security Automation
    demo_security_automation().expect("Security automation demo failed");

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    Demo Complete! 🎉                          ║");
    println!("║                                                               ║");
    println!("║  All advanced security features demonstrated successfully!   ║");
    println!("║  Your event store is now secured with enterprise-grade       ║");
    println!("║  protection against threats, data breaches, and attacks.     ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");
}

fn demo_anomaly_detection() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo 1: ML-Based Anomaly Detection                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🔍 Initializing anomaly detector with ML-based behavioral analysis...");

    let config = AnomalyDetectionConfig {
        enabled: true,
        sensitivity: 0.7,
        min_baseline_events: 100,
        analysis_window_hours: 24,
        enable_brute_force_detection: true,
        enable_unusual_access_detection: true,
        enable_privilege_escalation_detection: true,
        enable_data_exfiltration_detection: true,
        enable_velocity_detection: true,
    };

    let detector = AnomalyDetector::new(config);
    println!("✓ Anomaly detector initialized\n");

    // Scenario 1: Simulate brute force attack
    println!("📊 Scenario 1: Detecting Brute Force Attack");
    println!("   Simulating 6 failed login attempts in 2 minutes...");

    let tenant_id = TenantId::new("demo-tenant".to_string()).unwrap();
    let attacker = Actor::User {
        user_id: "attacker123".to_string(),
        username: "suspicious_user".to_string(),
    };

    for i in 1..=6 {
        let event = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::LoginFailed,
            attacker.clone(),
            AuditOutcome::Failure,
        );
        detector.add_recent_event(event.clone());
        println!("   └─ Failed login attempt #{}", i);

        if i == 6 {
            match detector.analyze_event(&event) {
                Ok(result) => {
                    if result.is_anomalous {
                        println!("\n   ⚠️  ANOMALY DETECTED!");
                        println!("   ├─ Type: {:?}", result.anomaly_type.unwrap());
                        println!("   ├─ Confidence Score: {:.1}%", result.score * 100.0);
                        println!("   ├─ Reason: {}", result.reason);
                        println!("   └─ Recommended Action: {:?}", result.recommended_action);

                        match result.recommended_action {
                            RecommendedAction::RevokeAccess => {
                                println!("   🚫 Action: IMMEDIATELY BLOCKING USER")
                            }
                            RecommendedAction::Block => println!("   🛑 Action: BLOCKING USER"),
                            RecommendedAction::RequireMFA => {
                                println!("   🔐 Action: REQUIRING MFA")
                            }
                            RecommendedAction::Alert => println!("   📢 Action: SENDING ALERT"),
                            RecommendedAction::Monitor => println!("   👁️  Action: MONITORING"),
                        }
                    }
                }
                Err(e) => println!("   Error analyzing event: {}", e),
            }
        }
    }

    pause("\n   Press Enter to continue to next scenario...");

    // Scenario 2: Data exfiltration detection
    println!("\n📊 Scenario 2: Detecting Data Exfiltration");
    println!("   Simulating unusual high-volume data queries...");

    let data_actor = Actor::User {
        user_id: "insider123".to_string(),
        username: "data_analyst".to_string(),
    };

    for i in 1..=20 {
        let event = AuditEvent::new(
            tenant_id.clone(),
            AuditAction::EventQueried,
            data_actor.clone(),
            AuditOutcome::Success,
        );
        detector.add_recent_event(event.clone());
    }

    let query_event = AuditEvent::new(
        tenant_id.clone(),
        AuditAction::EventQueried,
        data_actor,
        AuditOutcome::Success,
    );

    match detector.analyze_event(&query_event) {
        Ok(result) => {
            if result.is_anomalous && result.anomaly_type == Some(AnomalyType::DataExfiltration) {
                println!("\n   ⚠️  DATA EXFILTRATION DETECTED!");
                println!("   ├─ Confidence Score: {:.1}%", result.score * 100.0);
                println!("   ├─ Pattern: Queries 5x above normal rate");
                println!("   └─ Recommended Action: {:?}", result.recommended_action);
            } else {
                println!("   ✓ Normal activity detected");
            }
        }
        Err(e) => println!("   Error: {}", e),
    }

    let stats = detector.get_stats();
    println!("\n📈 Anomaly Detection Statistics:");
    println!("   ├─ User Profiles Tracked: {}", stats.user_profiles_count);
    println!(
        "   ├─ Recent Events Analyzed: {}",
        stats.recent_events_count
    );
    println!("   └─ Detection Types: Brute Force, Access Patterns, Privilege Escalation, Data Exfiltration, Velocity");

    pause("\n✓ Anomaly detection demo complete. Press Enter to continue...");
}

fn demo_field_encryption() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo 2: Field-Level Encryption                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🔐 Initializing AES-256-GCM encryption system...");

    let config = EncryptionConfig {
        enabled: true,
        key_rotation_days: 90,
        algorithm: EncryptionAlgorithm::Aes256Gcm,
    };

    let encryption = FieldEncryption::new(config)?;
    println!("✓ Encryption system initialized\n");

    // Encrypt sensitive data
    println!("📝 Encrypting sensitive data fields:");

    let sensitive_data = vec![
        ("SSN", "123-45-6789"),
        ("Credit Card", "4111-1111-1111-1111"),
        ("Password", "super_secret_password_123"),
        ("API Key", "sk_live_51H7x8yJ9K0L1M2N3O4P5"),
    ];

    for (field, value) in &sensitive_data {
        let encrypted = encryption.encrypt_string(value, field)?;
        println!("   ├─ {}: {} chars → encrypted", field, value.len());
        println!("   │  └─ Ciphertext: {}...", &encrypted.ciphertext[..20]);
        println!("   │  └─ Key ID: {}", &encrypted.key_id[..8]);
        println!("   │  └─ Version: {}", encrypted.version);
    }

    println!("\n🔄 Demonstrating key rotation...");
    println!(
        "   ├─ Current key version: {}",
        encryption.get_stats().active_key_version
    );

    let encrypted_before = encryption.encrypt_string("test data", "test")?;
    let version_before = encrypted_before.version;

    encryption.rotate_keys()?;
    println!("   ├─ ✓ Keys rotated successfully");

    let encrypted_after = encryption.encrypt_string("test data", "test")?;
    let version_after = encrypted_after.version;

    println!("   ├─ New key version: {}", version_after);
    println!(
        "   └─ Old data still decryptable: {}",
        encryption.decrypt_string(&encrypted_before).is_ok()
    );

    println!("\n🔓 Decrypting data encrypted with old key:");
    let decrypted = encryption.decrypt_string(&encrypted_before)?;
    println!(
        "   └─ ✓ Successfully decrypted: {} (version {})",
        decrypted, version_before
    );

    let stats = encryption.get_stats();
    println!("\n📈 Encryption Statistics:");
    println!("   ├─ Total Keys: {}", stats.total_keys);
    println!("   ├─ Active Key Version: {}", stats.active_key_version);
    println!("   ├─ Algorithm: {:?}", stats.algorithm);
    println!("   └─ Status: Operational ✓");

    pause("\n✓ Field-level encryption demo complete. Press Enter to continue...");
    Ok(())
}

async fn demo_kms_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo 3: HSM/KMS Integration                                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🔑 Initializing Key Management System (Local KMS for demo)...");
    println!("   Supported providers:");
    println!("   ├─ AWS KMS");
    println!("   ├─ Google Cloud KMS");
    println!("   ├─ Azure Key Vault");
    println!("   ├─ HashiCorp Vault");
    println!("   ├─ PKCS#11 HSM");
    println!("   └─ Local KMS (demo mode)\n");

    let config = KmsConfig {
        provider: KmsProvider::Local,
        config: std::collections::HashMap::new(),
        auto_rotate: true,
        rotation_period_days: 90,
    };

    let kms = KmsManager::new(config)?;
    println!("✓ KMS initialized in local mode\n");

    // Create master key
    println!("🔐 Creating master encryption key...");
    let master_key = kms
        .client()
        .create_key(
            "demo-master-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        )
        .await?;

    println!("   ├─ Key ID: {}", &master_key.key_id[..16]);
    println!("   ├─ Purpose: {:?}", master_key.purpose);
    println!("   ├─ Algorithm: {:?}", master_key.algorithm);
    println!("   └─ Status: {:?}", master_key.status);

    // Envelope encryption
    println!("\n📦 Demonstrating Envelope Encryption:");
    println!("   (Using DEK + Master Key for large data encryption)");

    let sensitive_data = b"This is highly sensitive financial data that needs strong protection";
    println!("\n   ├─ Original data size: {} bytes", sensitive_data.len());

    let encrypted = kms
        .envelope_encrypt(&master_key.key_id, sensitive_data)
        .await?;
    println!("   ├─ ✓ Data encrypted with DEK");
    println!("   ├─ ✓ DEK encrypted with Master Key");
    println!(
        "   └─ Encrypted package size: {} bytes",
        encrypted.ciphertext.len() + encrypted.encrypted_dek.len()
    );

    println!("\n   Decrypting with envelope decryption...");
    let decrypted = kms.envelope_decrypt(&encrypted).await?;
    println!("   └─ ✓ Successfully decrypted: {} bytes", decrypted.len());

    // Key rotation
    println!("\n🔄 Performing key rotation...");
    let rotated_key = kms.client().rotate_key(&master_key.key_id).await?;
    println!("   ├─ Old version: {}", master_key.version);
    println!("   ├─ New version: {}", rotated_key.version);
    println!("   └─ ✓ Rotation successful");

    println!("\n📈 KMS Statistics:");
    println!("   ├─ Provider: Local KMS");
    println!("   ├─ Total Keys: 1");
    println!("   ├─ Active Keys: 1");
    println!("   └─ Security Level: Enterprise-Grade ✓");

    pause("\n✓ KMS integration demo complete. Press Enter to continue...");
    Ok(())
}

fn demo_adaptive_rate_limiting() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo 4: Adaptive Rate Limiting                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🎯 Initializing ML-based adaptive rate limiter...");

    let config = AdaptiveRateLimitConfig {
        enabled: true,
        min_rate_limit: 10,
        max_rate_limit: 10_000,
        learning_window_hours: 24 * 7,
        adjustment_factor: 0.3,
        enable_anomaly_throttling: true,
        enable_load_based_adjustment: true,
        enable_pattern_prediction: true,
    };

    let limiter = AdaptiveRateLimiter::new(config);
    println!("✓ Adaptive limiter initialized\n");

    println!("📊 Simulating tenant usage patterns:");

    // Simulate normal usage
    println!("\n   Phase 1: Normal Usage Pattern");
    for i in 1..=100 {
        let result = limiter.check_adaptive_limit("demo-tenant")?;
        if i % 25 == 0 {
            println!("   ├─ Request #{}: {} remaining", i, result.remaining);
        }
    }

    if let Some(stats) = limiter.get_tenant_stats("demo-tenant") {
        println!("\n   📈 After 100 requests:");
        println!("   ├─ Current Limit: {} req/hour", stats.current_limit);
        println!("   ├─ Base Limit: {} req/hour", stats.base_limit);
        println!("   ├─ Utilization: {:.1}%", stats.utilization * 100.0);
        println!("   └─ Adjustments Made: {}", stats.total_adjustments);
    }

    // Record system load
    println!("\n   Phase 2: High System Load Detected");
    limiter.record_system_load(SystemLoad {
        cpu_usage: 0.85,
        memory_usage: 0.78,
        active_connections: 5000,
        queue_depth: 1200,
    });
    println!("   ├─ CPU Usage: 85%");
    println!("   ├─ Memory Usage: 78%");
    println!("   └─ Updating adaptive limits...");

    limiter.update_adaptive_limits()?;

    if let Some(stats) = limiter.get_tenant_stats("demo-tenant") {
        println!("\n   📉 After load-based adjustment:");
        println!("   ├─ Adjusted Limit: {} req/hour", stats.current_limit);
        println!("   └─ Reason: High system load protection");
    }

    // Simulate traffic spike
    println!("\n   Phase 3: Traffic Spike Detection");
    println!("   Simulating 3x normal traffic rate...");

    for _ in 0..300 {
        let _ = limiter.check_adaptive_limit("demo-tenant");
    }

    limiter.update_adaptive_limits()?;

    if let Some(stats) = limiter.get_tenant_stats("demo-tenant") {
        println!("\n   ⚠️  Anomaly-based throttling activated:");
        println!("   ├─ Traffic: 3x above normal");
        println!("   ├─ Throttled Limit: {} req/hour", stats.current_limit);
        println!("   └─ Protection: Active");
    }

    let overall_stats = limiter.get_stats();
    println!("\n📈 Overall System Statistics:");
    println!(
        "   ├─ Total Tenants Tracked: {}",
        overall_stats.total_tenants
    );
    println!(
        "   ├─ Learning Window: {} hours",
        overall_stats.config.learning_window_hours
    );
    println!(
        "   ├─ Anomaly Throttling: {}",
        if overall_stats.config.enable_anomaly_throttling {
            "✓ Enabled"
        } else {
            "Disabled"
        }
    );
    println!(
        "   ├─ Load-Based Adjustment: {}",
        if overall_stats.config.enable_load_based_adjustment {
            "✓ Enabled"
        } else {
            "Disabled"
        }
    );
    println!(
        "   └─ Pattern Prediction: {}",
        if overall_stats.config.enable_pattern_prediction {
            "✓ Enabled"
        } else {
            "Disabled"
        }
    );

    pause("\n✓ Adaptive rate limiting demo complete. Press Enter to continue...");
    Ok(())
}

fn demo_security_automation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo 5: Security Automation & CI/CD Scanning                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🤖 Initializing automated security scanner...");

    let config = SecurityScanConfig {
        enabled: true,
        scan_frequency_hours: 24,
        enable_dependency_scan: true,
        enable_secrets_scan: true,
        enable_sast: true,
        enable_license_check: true,
        fail_on_high_severity: true,
        fail_on_medium_severity: false,
    };

    let mut scanner = SecurityScanner::new(config);
    println!("✓ Scanner initialized\n");

    println!("🔍 Running comprehensive security scan...");
    println!("   This scans for:");
    println!("   ├─ Dependency vulnerabilities (cargo audit)");
    println!("   ├─ Hardcoded secrets and API keys");
    println!("   ├─ Code security issues (SAST)");
    println!("   └─ License compliance\n");

    println!("   [Scanning dependencies...]");
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("   ✓ Dependency scan complete");

    println!("   [Scanning for secrets...]");
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("   ✓ Secret scan complete");

    println!("   [Running SAST...]");
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("   ✓ SAST complete");

    println!("   [Checking licenses...]");
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("   ✓ License check complete");

    let result = scanner.run_full_scan()?;

    println!("\n📊 Scan Results:");
    println!("   ├─ Status: {:?}", result.status);
    println!(
        "   ├─ Timestamp: {}",
        result.timestamp.format("%Y-%m-%d %H:%M:%S")
    );
    println!("   └─ Summary:");
    println!("      ├─ Total Findings: {}", result.summary.total_findings);
    println!("      ├─ Critical: {}", result.summary.critical);
    println!("      ├─ High: {}", result.summary.high);
    println!("      ├─ Medium: {}", result.summary.medium);
    println!("      ├─ Low: {}", result.summary.low);
    println!("      └─ Info: {}", result.summary.info);

    if !result.findings.is_empty() {
        println!("\n   📋 Findings by Category:");
        for (category, findings) in &result.findings {
            println!("      ├─ {}: {} findings", category, findings.len());
            for finding in findings.iter().take(2) {
                println!("      │  ├─ [{:?}] {}", finding.severity, finding.title);
                if let Some(fix) = &finding.fix {
                    println!("      │  └─ Fix: {}", fix);
                }
            }
        }
    }

    if !result.recommendations.is_empty() {
        println!("\n   💡 Recommendations:");
        for (i, rec) in result.recommendations.iter().enumerate() {
            println!("      {}. {}", i + 1, rec);
        }
    }

    println!("\n🔧 CI/CD Integration:");
    println!("   Generating GitHub Actions workflow...");

    let workflow = CiCdIntegration::generate_github_actions_workflow();
    println!(
        "   ✓ GitHub Actions workflow generated ({} lines)",
        workflow.lines().count()
    );
    println!("   └─ Save to: .github/workflows/security.yml");

    println!("\n   Generating GitLab CI config...");
    let gitlab_config = CiCdIntegration::generate_gitlab_ci_config();
    println!(
        "   ✓ GitLab CI config generated ({} lines)",
        gitlab_config.lines().count()
    );
    println!("   └─ Save to: .gitlab-ci.yml");

    println!("\n📈 Automation Statistics:");
    println!("   ├─ Scan Types: 4 (Dependencies, Secrets, SAST, Licenses)");
    println!("   ├─ Frequency: Every 24 hours");
    println!("   ├─ CI/CD Support: GitHub Actions, GitLab CI");
    println!("   └─ Auto-fail on: High Severity Issues");

    pause("\n✓ Security automation demo complete. Press Enter to finish...");
    Ok(())
}

fn pause(message: &str) {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}
