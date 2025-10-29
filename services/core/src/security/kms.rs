/// Key Management System (KMS) Integration
///
/// Provides integration with external KMS/HSM for secure key storage:
/// - AWS KMS
/// - Google Cloud KMS
/// - Azure Key Vault
/// - HashiCorp Vault
/// - Local HSM via PKCS#11

use crate::error::{AllSourceError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// KMS provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KmsProvider {
    AwsKms,
    GoogleCloudKms,
    AzureKeyVault,
    HashicorpVault,
    Pkcs11,
    Local, // For development/testing only
}

/// KMS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmsConfig {
    /// KMS provider
    pub provider: KmsProvider,

    /// Provider-specific configuration
    pub config: HashMap<String, String>,

    /// Enable automatic key rotation
    pub auto_rotate: bool,

    /// Key rotation period in days
    pub rotation_period_days: u32,
}

impl Default for KmsConfig {
    fn default() -> Self {
        Self {
            provider: KmsProvider::Local,
            config: HashMap::new(),
            auto_rotate: true,
            rotation_period_days: 90,
        }
    }
}

/// Key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    /// Key ID
    pub key_id: String,

    /// Key alias/name
    pub alias: String,

    /// Key purpose
    pub purpose: KeyPurpose,

    /// Key algorithm
    pub algorithm: KeyAlgorithm,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last rotation timestamp
    pub last_rotated: Option<chrono::DateTime<chrono::Utc>>,

    /// Key status
    pub status: KeyStatus,

    /// Key version
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyPurpose {
    DataEncryption,
    JwtSigning,
    ApiKeySigning,
    DatabaseEncryption,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyAlgorithm {
    Aes256Gcm,
    Aes128Gcm,
    ChaCha20Poly1305,
    RsaOaep,
    EcdsaP256,
    Ed25519,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    Active,
    Rotating,
    Deprecated,
    Destroyed,
}

/// KMS client trait
#[async_trait::async_trait]
pub trait KmsClient: Send + Sync {
    /// Create a new key
    async fn create_key(&self, alias: String, purpose: KeyPurpose, algorithm: KeyAlgorithm) -> Result<KeyMetadata>;

    /// Get key metadata
    async fn get_key(&self, key_id: &str) -> Result<KeyMetadata>;

    /// List all keys
    async fn list_keys(&self) -> Result<Vec<KeyMetadata>>;

    /// Encrypt data using KMS
    async fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt data using KMS
    async fn decrypt(&self, key_id: &str, ciphertext: &[u8]) -> Result<Vec<u8>>;

    /// Rotate key
    async fn rotate_key(&self, key_id: &str) -> Result<KeyMetadata>;

    /// Disable key
    async fn disable_key(&self, key_id: &str) -> Result<()>;

    /// Enable key
    async fn enable_key(&self, key_id: &str) -> Result<()>;

    /// Generate data encryption key (for envelope encryption)
    async fn generate_data_key(&self, key_id: &str) -> Result<(Vec<u8>, Vec<u8>)>;
}

/// Local KMS implementation (for testing/development)
pub struct LocalKms {
    keys: Arc<RwLock<HashMap<String, StoredKey>>>,
    config: KmsConfig,
}

struct StoredKey {
    metadata: KeyMetadata,
    key_material: Vec<u8>,
}

impl LocalKms {
    pub fn new(config: KmsConfig) -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
}

#[async_trait::async_trait]
impl KmsClient for LocalKms {
    async fn create_key(&self, alias: String, purpose: KeyPurpose, algorithm: KeyAlgorithm) -> Result<KeyMetadata> {
        let key_id = uuid::Uuid::new_v4().to_string();

        // Generate key material based on algorithm
        let key_material = match algorithm {
            KeyAlgorithm::Aes256Gcm => {
                let mut key = vec![0u8; 32];
                use aes_gcm::aead::OsRng;
                use aes_gcm::aead::rand_core::RngCore;
                RngCore::fill_bytes(&mut OsRng, &mut key);
                key
            }
            KeyAlgorithm::Aes128Gcm => {
                let mut key = vec![0u8; 16];
                use aes_gcm::aead::OsRng;
                use aes_gcm::aead::rand_core::RngCore;
                RngCore::fill_bytes(&mut OsRng, &mut key);
                key
            }
            _ => {
                return Err(AllSourceError::ValidationError(
                    format!("Algorithm {:?} not supported in local KMS", algorithm)
                ));
            }
        };

        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            alias,
            purpose,
            algorithm,
            created_at: chrono::Utc::now(),
            last_rotated: None,
            status: KeyStatus::Active,
            version: 1,
        };

        let stored_key = StoredKey {
            metadata: metadata.clone(),
            key_material,
        };

        self.keys.write().insert(key_id, stored_key);

        Ok(metadata)
    }

    async fn get_key(&self, key_id: &str) -> Result<KeyMetadata> {
        self.keys.read()
            .get(key_id)
            .map(|k| k.metadata.clone())
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))
    }

    async fn list_keys(&self) -> Result<Vec<KeyMetadata>> {
        Ok(self.keys.read()
            .values()
            .map(|k| k.metadata.clone())
            .collect())
    }

    async fn encrypt(&self, key_id: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;

        let keys = self.keys.read();
        let stored_key = keys.get(key_id)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))?;

        if stored_key.metadata.status != KeyStatus::Active {
            return Err(AllSourceError::ValidationError("Key is not active".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&stored_key.key_material)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        // Generate nonce
        use aes_gcm::aead::OsRng;
        use aes_gcm::aead::rand_core::RngCore;
        let nonce_bytes = OsRng.next_u64().to_le_bytes();
        let mut nonce_array = [0u8; 12];
        nonce_array[..8].copy_from_slice(&nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_array);

        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| AllSourceError::ValidationError(format!("Encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    async fn decrypt(&self, key_id: &str, ciphertext_with_nonce: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;

        if ciphertext_with_nonce.len() < 12 {
            return Err(AllSourceError::ValidationError("Invalid ciphertext".to_string()));
        }

        let keys = self.keys.read();
        let stored_key = keys.get(key_id)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))?;

        let cipher = Aes256Gcm::new_from_slice(&stored_key.key_material)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&ciphertext_with_nonce[..12]);
        let ciphertext = &ciphertext_with_nonce[12..];

        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| AllSourceError::ValidationError(format!("Decryption failed: {}", e)))
    }

    async fn rotate_key(&self, key_id: &str) -> Result<KeyMetadata> {
        let mut keys = self.keys.write();
        let stored_key = keys.get_mut(key_id)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))?;

        // Generate new key material
        let new_key_material = {
            let mut key = vec![0u8; 32];
            use aes_gcm::aead::OsRng;
            use aes_gcm::aead::rand_core::RngCore;
            RngCore::fill_bytes(&mut OsRng, &mut key);
            key
        };

        stored_key.key_material = new_key_material;
        stored_key.metadata.version += 1;
        stored_key.metadata.last_rotated = Some(chrono::Utc::now());

        Ok(stored_key.metadata.clone())
    }

    async fn disable_key(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write();
        let stored_key = keys.get_mut(key_id)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))?;

        stored_key.metadata.status = KeyStatus::Deprecated;
        Ok(())
    }

    async fn enable_key(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write();
        let stored_key = keys.get_mut(key_id)
            .ok_or_else(|| AllSourceError::ValidationError(format!("Key {} not found", key_id)))?;

        stored_key.metadata.status = KeyStatus::Active;
        Ok(())
    }

    async fn generate_data_key(&self, key_id: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        // Generate data encryption key
        let mut dek = vec![0u8; 32];
        use aes_gcm::aead::OsRng;
        use aes_gcm::aead::rand_core::RngCore;
        RngCore::fill_bytes(&mut OsRng, &mut dek);

        // Encrypt DEK with master key
        let encrypted_dek = self.encrypt(key_id, &dek).await?;

        Ok((dek, encrypted_dek))
    }
}

/// KMS manager for handling multiple providers
pub struct KmsManager {
    client: Arc<dyn KmsClient>,
    config: KmsConfig,
}

impl KmsManager {
    /// Create new KMS manager
    pub fn new(config: KmsConfig) -> Result<Self> {
        let client: Arc<dyn KmsClient> = match config.provider {
            KmsProvider::Local => {
                Arc::new(LocalKms::new(config.clone()))
            }
            _ => {
                return Err(AllSourceError::ValidationError(
                    format!("KMS provider {:?} not yet implemented", config.provider)
                ));
            }
        };

        Ok(Self { client, config })
    }

    /// Get KMS client
    pub fn client(&self) -> &Arc<dyn KmsClient> {
        &self.client
    }

    /// Encrypt data using envelope encryption
    pub async fn envelope_encrypt(&self, master_key_id: &str, plaintext: &[u8]) -> Result<EnvelopeEncryptedData> {
        // Generate data encryption key
        let (dek, encrypted_dek) = self.client.generate_data_key(master_key_id).await?;

        // Encrypt data with DEK
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;

        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        use aes_gcm::aead::OsRng;
        use aes_gcm::aead::rand_core::RngCore;
        let nonce_bytes = OsRng.next_u64().to_le_bytes();
        let mut nonce_array = [0u8; 12];
        nonce_array[..8].copy_from_slice(&nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_array);

        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| AllSourceError::ValidationError(format!("Encryption failed: {}", e)))?;

        Ok(EnvelopeEncryptedData {
            ciphertext,
            nonce: nonce.to_vec(),
            encrypted_dek,
            master_key_id: master_key_id.to_string(),
        })
    }

    /// Decrypt data using envelope encryption
    pub async fn envelope_decrypt(&self, encrypted: &EnvelopeEncryptedData) -> Result<Vec<u8>> {
        // Decrypt DEK
        let dek = self.client.decrypt(&encrypted.master_key_id, &encrypted.encrypted_dek).await?;

        // Decrypt data with DEK
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
        use aes_gcm::aead::Aead;

        let cipher = Aes256Gcm::new_from_slice(&dek)
            .map_err(|e| AllSourceError::ValidationError(format!("Invalid key: {}", e)))?;

        let nonce = Nonce::from_slice(&encrypted.nonce);

        cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| AllSourceError::ValidationError(format!("Decryption failed: {}", e)))
    }
}

/// Envelope encrypted data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeEncryptedData {
    /// Encrypted data
    pub ciphertext: Vec<u8>,

    /// Nonce/IV
    pub nonce: Vec<u8>,

    /// Encrypted data encryption key
    pub encrypted_dek: Vec<u8>,

    /// Master key ID
    pub master_key_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_kms_create_key() {
        let config = KmsConfig::default();
        let kms = LocalKms::new(config);

        let metadata = kms.create_key(
            "test-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        ).await.unwrap();

        assert_eq!(metadata.alias, "test-key");
        assert_eq!(metadata.status, KeyStatus::Active);
        assert_eq!(metadata.version, 1);
    }

    #[tokio::test]
    async fn test_local_kms_encrypt_decrypt() {
        let config = KmsConfig::default();
        let kms = LocalKms::new(config);

        let key = kms.create_key(
            "test-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        ).await.unwrap();

        let plaintext = b"sensitive data";
        let ciphertext = kms.encrypt(&key.key_id, plaintext).await.unwrap();
        let decrypted = kms.decrypt(&key.key_id, &ciphertext).await.unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let config = KmsConfig::default();
        let kms = LocalKms::new(config);

        let key = kms.create_key(
            "test-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        ).await.unwrap();

        let rotated = kms.rotate_key(&key.key_id).await.unwrap();
        assert_eq!(rotated.version, 2);
        assert!(rotated.last_rotated.is_some());
    }

    #[tokio::test]
    async fn test_envelope_encryption() {
        let config = KmsConfig::default();
        let manager = KmsManager::new(config).unwrap();

        // Create master key
        let master_key = manager.client().create_key(
            "master-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        ).await.unwrap();

        // Encrypt data
        let plaintext = b"sensitive data for envelope encryption";
        let encrypted = manager.envelope_encrypt(&master_key.key_id, plaintext).await.unwrap();

        // Decrypt data
        let decrypted = manager.envelope_decrypt(&encrypted).await.unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_disable_enable_key() {
        let config = KmsConfig::default();
        let kms = LocalKms::new(config);

        let key = kms.create_key(
            "test-key".to_string(),
            KeyPurpose::DataEncryption,
            KeyAlgorithm::Aes256Gcm,
        ).await.unwrap();

        // Disable key
        kms.disable_key(&key.key_id).await.unwrap();
        let metadata = kms.get_key(&key.key_id).await.unwrap();
        assert_eq!(metadata.status, KeyStatus::Deprecated);

        // Try to encrypt with disabled key (should fail)
        let result = kms.encrypt(&key.key_id, b"test").await;
        assert!(result.is_err());

        // Enable key
        kms.enable_key(&key.key_id).await.unwrap();
        let metadata = kms.get_key(&key.key_id).await.unwrap();
        assert_eq!(metadata.status, KeyStatus::Active);

        // Should work now
        let result = kms.encrypt(&key.key_id, b"test").await;
        assert!(result.is_ok());
    }
}
