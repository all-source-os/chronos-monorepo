pub mod adaptive_rate_limit;
/// Advanced Security Module
///
/// Comprehensive security features including:
/// - ML-based anomaly detection
/// - Field-level encryption
/// - HSM/KMS integration
/// - Adaptive rate limiting
/// - Security automation and CI/CD scanning
pub mod anomaly_detection;
pub mod automation;
pub mod encryption;
pub mod kms;

// Re-export main types
pub use anomaly_detection::{
    AnomalyDetectionConfig, AnomalyDetector, AnomalyResult, AnomalyType, DetectionStats,
    RecommendedAction,
};

pub use encryption::{
    decrypt_json_value, encrypt_json_value, Encryptable, EncryptedData, EncryptionAlgorithm,
    EncryptionConfig, EncryptionStats, FieldEncryption,
};

pub use kms::{
    EnvelopeEncryptedData, KeyAlgorithm, KeyMetadata, KeyPurpose, KeyStatus, KmsClient, KmsConfig,
    KmsManager, KmsProvider, LocalKms,
};

pub use adaptive_rate_limit::{
    AdaptiveLimitStats, AdaptiveRateLimitConfig, AdaptiveRateLimiter, AdaptiveRateLimiterStats,
    SystemLoad,
};

pub use automation::{
    CiCdIntegration, FindingCategory, ScanStatus, ScanSummary, SecurityFinding, SecurityScanConfig,
    SecurityScanResult, SecurityScanner, Severity,
};
