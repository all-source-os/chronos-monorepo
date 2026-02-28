pub mod adaptive_rate_limit;
/// Advanced Security Module
///
/// Comprehensive security features including:
/// - ML-based anomaly detection
/// - Field-level encryption
/// - Local KMS key management
/// - Adaptive rate limiting
/// - Security automation and CI/CD scanning
pub mod anomaly_detection;
pub mod automation;
#[cfg(feature = "server")]
pub mod encryption;
#[cfg(feature = "server")]
pub mod kms;

// Re-export main types
pub use anomaly_detection::{
    AnomalyDetectionConfig, AnomalyDetector, AnomalyResult, AnomalyType, DetectionStats,
    RecommendedAction,
};

#[cfg(feature = "server")]
pub use encryption::{
    Encryptable, EncryptedData, EncryptionAlgorithm, EncryptionConfig, EncryptionStats,
    FieldEncryption, decrypt_json_value, encrypt_json_value,
};

#[cfg(feature = "server")]
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
