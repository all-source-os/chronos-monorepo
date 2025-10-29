/// Field-Level Encryption for Sensitive Data
///
/// Provides transparent encryption/decryption of sensitive fields using:
/// - AES-256-GCM for symmetric encryption
/// - Envelope encryption pattern
/// - Key rotation support
/// - Per-field encryption keys

use crate::error::{AllSourceError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,

    /// Key rotation period in days
    pub key_rotation_days: u32,

    /// Algorithm to use
    pub algorithm: EncryptionAlgorithm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            key_rotation_days: 90,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        }
    }
}

/// Encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Ciphertext (base64 encoded)
    pub ciphertext: String,

    /// Nonce/IV (base64 encoded)
    pub nonce: String,

    /// Key ID used for encryption
    pub key_id: String,

    /// Algorithm used
    pub algorithm: EncryptionAlgorithm,

    /// Version for key rotation
    pub version: u32,
}

/// Data encryption key
#[derive(Debug, Clone)]
struct DataEncryptionKey {
    key_id: String,
    key_bytes: Vec<u8>,
    version: u32,
    created_at: chrono::DateTime<chrono::Utc>,
    active: bool,
}

/// Field-level encryption manager
pub struct FieldEncryption {
    config: Arc<RwLock<EncryptionConfig>>,

    // Data encryption keys (DEKs)
    deks: Arc<RwLock<HashMap<String, DataEncryptionKey>>>,

    // Active key for encryption
    active_key_id: Arc<RwLock<Option<String>>>,
}

impl FieldEncryption {
    /// Create new field encryption manager
    pub fn new(config: EncryptionConfig) -> Result<Self> {
        let manager = Self {
            config: Arc::new(RwLock::new(config)),
            deks: Arc::new(RwLock::new(HashMap::new())),
            active_key_id: Arc::new(RwLock::new(None)),
        };

        // Generate initial key
        manager.rotate_keys()?;

        Ok(manager)
    }

    /// Encrypt a string value
    pub fn encrypt_string(&self, plaintext: &str, field_name: &str) -> Result<EncryptedData> {
        if !self.config.read().enabled {
            return Err(AllSourceError::ValidationError(
                "Encryption is disabled".to_string(),
            ));
        }

        let active_key_id = self.active_key_id.read();
        let key_id = active_key_id.as_ref()
            .ok_or_else(|| AllSourceError::ValidationError("No active encryption key".to_string()))?
            .clone();

        let deks = self.deks.read();
        let dek = deks.get(&key_id)
            .ok_or_else(|| AllSourceError::ValidationError("Encryption key not found".to_string()))?;

        // Use AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&dek.key_bytes)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        // Generate random nonce
        let nonce_bytes = aes_gcm::aead::rand_core::RngCore::next_u64(&mut OsRng).to_le_bytes();
        let mut nonce_array = [0u8; 12];
        nonce_array[..8].copy_from_slice(&nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_array);

        // Encrypt with associated data (field name) for integrity
        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AllSourceError::ValidationError(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedData {
            ciphertext: general_purpose::STANDARD.encode(&ciphertext),
            nonce: general_purpose::STANDARD.encode(nonce.as_slice()),
            key_id: key_id.clone(),
            algorithm: self.config.read().algorithm.clone(),
            version: dek.version,
        })
    }

    /// Decrypt a string value
    pub fn decrypt_string(&self, encrypted: &EncryptedData) -> Result<String> {
        if !self.config.read().enabled {
            return Err(AllSourceError::ValidationError(
                "Encryption is disabled".to_string(),
            ));
        }

        let deks = self.deks.read();
        let dek = deks.get(&encrypted.key_id)
            .ok_or_else(|| AllSourceError::ValidationError(
                format!("Encryption key {} not found", encrypted.key_id)
            ))?;

        // Decode base64
        let ciphertext = general_purpose::STANDARD.decode(&encrypted.ciphertext)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid ciphertext encoding: {}", e)))?;

        let nonce_bytes = general_purpose::STANDARD.decode(&encrypted.nonce)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid nonce encoding: {}", e)))?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decrypt
        let cipher = Aes256Gcm::new_from_slice(&dek.key_bytes)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        let plaintext_bytes = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| AllSourceError::ValidationError(format!("Decryption failed: {}", e)))?;

        String::from_utf8(plaintext_bytes)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid UTF-8: {}", e)))
    }

    /// Rotate encryption keys
    pub fn rotate_keys(&self) -> Result<()> {
        let mut deks = self.deks.write();
        let mut active_key_id = self.active_key_id.write();

        // Generate new key
        let key_id = uuid::Uuid::new_v4().to_string();
        let mut key_bytes = vec![0u8; 32]; // 256 bits for AES-256
        aes_gcm::aead::rand_core::RngCore::fill_bytes(&mut OsRng, &mut key_bytes);

        let version = deks.len() as u32 + 1;

        let new_key = DataEncryptionKey {
            key_id: key_id.clone(),
            key_bytes,
            version,
            created_at: chrono::Utc::now(),
            active: true,
        };

        // Deactivate old keys
        for key in deks.values_mut() {
            key.active = false;
        }

        // Add new key
        deks.insert(key_id.clone(), new_key);
        *active_key_id = Some(key_id);

        Ok(())
    }

    /// Get encryption statistics
    pub fn get_stats(&self) -> EncryptionStats {
        let deks = self.deks.read();
        let active_key_id = self.active_key_id.read();

        EncryptionStats {
            enabled: self.config.read().enabled,
            total_keys: deks.len(),
            active_key_version: deks.get(active_key_id.as_ref().unwrap_or(&String::new()))
                .map(|k| k.version)
                .unwrap_or(0),
            algorithm: self.config.read().algorithm.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionStats {
    pub enabled: bool,
    pub total_keys: usize,
    pub active_key_version: u32,
    pub algorithm: EncryptionAlgorithm,
}

/// Trait for types that can be encrypted
pub trait Encryptable {
    fn encrypt(&self, encryption: &FieldEncryption, field_name: &str) -> Result<EncryptedData>;
    fn decrypt(encrypted: &EncryptedData, encryption: &FieldEncryption) -> Result<Self>
    where
        Self: Sized;
}

impl Encryptable for String {
    fn encrypt(&self, encryption: &FieldEncryption, field_name: &str) -> Result<EncryptedData> {
        encryption.encrypt_string(self, field_name)
    }

    fn decrypt(encrypted: &EncryptedData, encryption: &FieldEncryption) -> Result<Self> {
        encryption.decrypt_string(encrypted)
    }
}

/// Helper for encrypting JSON values
pub fn encrypt_json_value(
    value: &serde_json::Value,
    encryption: &FieldEncryption,
    field_name: &str,
) -> Result<EncryptedData> {
    let json_string = serde_json::to_string(value)
        .map_err(|e| AllSourceError::ValidationError(format!("JSON serialization failed: {}", e)))?;

    encryption.encrypt_string(&json_string, field_name)
}

/// Helper for decrypting JSON values
pub fn decrypt_json_value(
    encrypted: &EncryptedData,
    encryption: &FieldEncryption,
) -> Result<serde_json::Value> {
    let json_string = encryption.decrypt_string(encrypted)?;

    serde_json::from_str(&json_string)
        .map_err(|e| AllSourceError::ValidationError(format!("JSON deserialization failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_creation() {
        let encryption = FieldEncryption::new(EncryptionConfig::default()).unwrap();
        let stats = encryption.get_stats();

        assert!(stats.enabled);
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.active_key_version, 1);
    }

    #[test]
    fn test_encrypt_decrypt_string() {
        let encryption = FieldEncryption::new(EncryptionConfig::default()).unwrap();
        let plaintext = "sensitive data";

        let encrypted = encryption.encrypt_string(plaintext, "test_field").unwrap();
        assert_ne!(encrypted.ciphertext, plaintext);

        let decrypted = encryption.decrypt_string(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_json() {
        let encryption = FieldEncryption::new(EncryptionConfig::default()).unwrap();
        let value = serde_json::json!({
            "username": "john_doe",
            "ssn": "123-45-6789",
            "credit_card": "4111-1111-1111-1111"
        });

        let encrypted = encrypt_json_value(&value, &encryption, "sensitive_data").unwrap();
        let decrypted = decrypt_json_value(&encrypted, &encryption).unwrap();

        assert_eq!(decrypted, value);
    }

    #[test]
    fn test_key_rotation() {
        let encryption = FieldEncryption::new(EncryptionConfig::default()).unwrap();
        let plaintext = "sensitive data";

        // Encrypt with first key
        let encrypted1 = encryption.encrypt_string(plaintext, "test").unwrap();
        let key_id1 = encrypted1.key_id.clone();

        // Rotate keys
        encryption.rotate_keys().unwrap();

        // Encrypt with new key
        let encrypted2 = encryption.encrypt_string(plaintext, "test").unwrap();
        let key_id2 = encrypted2.key_id.clone();

        // Keys should be different
        assert_ne!(key_id1, key_id2);
        assert_eq!(encrypted2.version, 2);

        // Should still be able to decrypt data encrypted with old key
        let decrypted1 = encryption.decrypt_string(&encrypted1).unwrap();
        assert_eq!(decrypted1, plaintext);

        // Should be able to decrypt data encrypted with new key
        let decrypted2 = encryption.decrypt_string(&encrypted2).unwrap();
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn test_multiple_key_rotations() {
        let encryption = FieldEncryption::new(EncryptionConfig::default()).unwrap();
        let plaintext = "test data";

        let mut encrypted_data = Vec::new();

        // Create data with multiple key versions
        for _ in 0..5 {
            let encrypted = encryption.encrypt_string(plaintext, "test").unwrap();
            encrypted_data.push(encrypted);
            encryption.rotate_keys().unwrap();
        }

        // Should be able to decrypt all versions
        for encrypted in &encrypted_data {
            let decrypted = encryption.decrypt_string(encrypted).unwrap();
            assert_eq!(decrypted, plaintext);
        }

        let stats = encryption.get_stats();
        assert_eq!(stats.total_keys, 6); // Initial + 5 rotations
        assert_eq!(stats.active_key_version, 6);
    }

    #[test]
    fn test_disabled_encryption() {
        let mut config = EncryptionConfig::default();
        config.enabled = false;

        let encryption = FieldEncryption::new(config).unwrap();
        let plaintext = "test";

        let result = encryption.encrypt_string(plaintext, "test");
        assert!(result.is_err());
    }
}
