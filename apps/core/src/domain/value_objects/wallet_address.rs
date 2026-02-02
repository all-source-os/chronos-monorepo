use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Value Object: WalletAddress
///
/// Represents a cryptocurrency wallet address in the paywall system.
/// Supports Solana addresses (base58 encoded, 32-44 characters).
///
/// Domain Rules:
/// - Cannot be empty
/// - Must be a valid base58 encoded string
/// - Must be between 32 and 44 characters (Solana address format)
/// - Case-sensitive
/// - Immutable once created
///
/// This is a Value Object:
/// - Defined by its value, not identity
/// - Immutable
/// - Self-validating
/// - Compared by value equality
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletAddress(String);

/// Base58 alphabet used by Solana
const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

impl WalletAddress {
    /// Create a new WalletAddress with validation
    ///
    /// # Errors
    /// Returns error if:
    /// - Address is empty
    /// - Address is not valid base58
    /// - Address is not between 32-44 characters
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::WalletAddress;
    ///
    /// // Valid Solana address format
    /// let wallet = WalletAddress::new("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string()).unwrap();
    /// assert_eq!(wallet.as_str(), "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");
    /// ```
    pub fn new(value: String) -> Result<Self> {
        Self::validate(&value)?;
        Ok(Self(value))
    }

    /// Create WalletAddress without validation (for internal use, e.g., from trusted storage)
    ///
    /// # Safety
    /// This bypasses validation. Only use when loading from trusted sources
    /// where validation has already occurred.
    pub(crate) fn new_unchecked(value: String) -> Self {
        Self(value)
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner String (consumes self)
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Get an anonymized version of the wallet address for display
    ///
    /// Returns the first 4 and last 4 characters with ellipsis in between.
    ///
    /// # Examples
    /// ```
    /// use allsource_core::domain::value_objects::WalletAddress;
    ///
    /// let wallet = WalletAddress::new("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string()).unwrap();
    /// assert_eq!(wallet.anonymized(), "9WzD...AWWM");
    /// ```
    pub fn anonymized(&self) -> String {
        if self.0.len() <= 8 {
            return self.0.clone();
        }
        format!("{}...{}", &self.0[..4], &self.0[self.0.len() - 4..])
    }

    /// Check if this is a valid Solana address format
    pub fn is_solana(&self) -> bool {
        // Solana addresses are 32-44 characters base58 encoded
        self.0.len() >= 32 && self.0.len() <= 44
    }

    /// Validate a wallet address string
    fn validate(value: &str) -> Result<()> {
        // Rule: Cannot be empty
        if value.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Wallet address cannot be empty".to_string(),
            ));
        }

        // Rule: Must be valid length (Solana: 32-44 characters)
        if value.len() < 32 || value.len() > 44 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Wallet address must be between 32 and 44 characters, got {}",
                value.len()
            )));
        }

        // Rule: Must be valid base58
        if !value.chars().all(|c| BASE58_ALPHABET.contains(c)) {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Wallet address contains invalid base58 characters".to_string(),
            ));
        }

        Ok(())
    }
}

impl fmt::Display for WalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for WalletAddress {
    type Error = crate::error::AllSourceError;

    fn try_from(value: &str) -> Result<Self> {
        WalletAddress::new(value.to_string())
    }
}

impl TryFrom<String> for WalletAddress {
    type Error = crate::error::AllSourceError;

    fn try_from(value: String) -> Result<Self> {
        WalletAddress::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A valid Solana address for testing (44 characters)
    const VALID_SOLANA_ADDRESS: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    // Another valid address (32 characters - minimum length)
    const VALID_SHORT_ADDRESS: &str = "11111111111111111111111111111111";

    #[test]
    fn test_create_valid_wallet_address() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string());
        assert!(wallet.is_ok());
        assert_eq!(wallet.unwrap().as_str(), VALID_SOLANA_ADDRESS);
    }

    #[test]
    fn test_create_short_valid_address() {
        let wallet = WalletAddress::new(VALID_SHORT_ADDRESS.to_string());
        assert!(wallet.is_ok());
    }

    #[test]
    fn test_reject_empty_address() {
        let result = WalletAddress::new("".to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn test_reject_too_short_address() {
        // 31 characters - too short
        let short_address = "1234567890123456789012345678901";
        let result = WalletAddress::new(short_address.to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("must be between 32 and 44"));
        }
    }

    #[test]
    fn test_reject_too_long_address() {
        // 45 characters - too long
        let long_address = "123456789012345678901234567890123456789012345";
        let result = WalletAddress::new(long_address.to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("must be between 32 and 44"));
        }
    }

    #[test]
    fn test_reject_invalid_base58_characters() {
        // Contains '0' which is not in base58
        let invalid = "0WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let result = WalletAddress::new(invalid.to_string());
        assert!(result.is_err());

        // Contains 'O' which is not in base58
        let invalid = "OWzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let result = WalletAddress::new(invalid.to_string());
        assert!(result.is_err());

        // Contains 'I' which is not in base58
        let invalid = "IWzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let result = WalletAddress::new(invalid.to_string());
        assert!(result.is_err());

        // Contains 'l' which is not in base58
        let invalid = "lWzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
        let result = WalletAddress::new(invalid.to_string());
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("invalid base58"));
        }
    }

    #[test]
    fn test_anonymized() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let anonymized = wallet.anonymized();
        assert_eq!(anonymized, "9WzD...AWWM");
    }

    #[test]
    fn test_is_solana() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        assert!(wallet.is_solana());

        let wallet = WalletAddress::new(VALID_SHORT_ADDRESS.to_string()).unwrap();
        assert!(wallet.is_solana());
    }

    #[test]
    fn test_display_trait() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        assert_eq!(format!("{}", wallet), VALID_SOLANA_ADDRESS);
    }

    #[test]
    fn test_try_from_str() {
        let wallet: Result<WalletAddress> = VALID_SOLANA_ADDRESS.try_into();
        assert!(wallet.is_ok());
        assert_eq!(wallet.unwrap().as_str(), VALID_SOLANA_ADDRESS);

        let invalid: Result<WalletAddress> = "".try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_try_from_string() {
        let wallet: Result<WalletAddress> = VALID_SOLANA_ADDRESS.to_string().try_into();
        assert!(wallet.is_ok());

        let invalid: Result<WalletAddress> = String::new().try_into();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_into_inner() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let inner = wallet.into_inner();
        assert_eq!(inner, VALID_SOLANA_ADDRESS);
    }

    #[test]
    fn test_equality() {
        let w1 = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let w2 = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let w3 = WalletAddress::new(VALID_SHORT_ADDRESS.to_string()).unwrap();

        // Value equality
        assert_eq!(w1, w2);
        assert_ne!(w1, w3);
    }

    #[test]
    fn test_cloning() {
        let w1 = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let w2 = w1.clone();
        assert_eq!(w1, w2);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::HashSet;

        let w1 = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();
        let w2 = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();

        let mut set = HashSet::new();
        set.insert(w1);

        // Should find the same value (value equality)
        assert!(set.contains(&w2));
    }

    #[test]
    fn test_serde_serialization() {
        let wallet = WalletAddress::new(VALID_SOLANA_ADDRESS.to_string()).unwrap();

        // Serialize
        let json = serde_json::to_string(&wallet).unwrap();
        assert_eq!(json, format!("\"{}\"", VALID_SOLANA_ADDRESS));

        // Deserialize
        let deserialized: WalletAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, wallet);
    }

    #[test]
    fn test_new_unchecked() {
        // Should create without validation (for internal use)
        let wallet = WalletAddress::new_unchecked("invalid".to_string());
        assert_eq!(wallet.as_str(), "invalid");
    }
}
