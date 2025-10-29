/// Advanced Security Module
///
/// Comprehensive security features including:
/// - ML-based anomaly detection
/// - Field-level encryption
/// - HSM/KMS integration
/// - Adaptive rate limiting
/// - Security automation and CI/CD scanning

pub mod anomaly_detection;
pub mod encryption;
pub mod kms;
pub mod adaptive_rate_limit;
pub mod automation;

// Re-export main types
pub use anomaly_detection::{
    AnomalyDetector, AnomalyDetectionConfig, AnomalyResult, AnomalyType,
    RecommendedAction, DetectionStats,
};

pub use encryption::{
    FieldEncryption, EncryptionConfig, EncryptedData, EncryptionAlgorithm,
    Encryptable, encrypt_json_value, decrypt_json_value, EncryptionStats,
};

pub use kms::{
    KmsManager, KmsConfig, KmsProvider, KmsClient, LocalKms,
    KeyMetadata, KeyPurpose, KeyAlgorithm, KeyStatus,
    EnvelopeEncryptedData,
};

pub use adaptive_rate_limit::{
    AdaptiveRateLimiter, AdaptiveRateLimitConfig, SystemLoad,
    AdaptiveLimitStats, AdaptiveRateLimiterStats,
};

pub use automation::{
    SecurityScanner, SecurityScanConfig, SecurityScanResult,
    ScanStatus, SecurityFinding, Severity, FindingCategory,
    ScanSummary, CiCdIntegration,
};
