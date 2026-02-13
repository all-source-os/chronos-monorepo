use crate::{
    domain::value_objects::{CreatorId, TenantId, WalletAddress},
    error::Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Creator status in the paywall system
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatorStatus {
    /// Creator has signed up but not verified email
    #[default]
    Pending,
    /// Creator is active and can receive payments
    Active,
    /// Creator has been temporarily suspended
    Suspended,
    /// Creator has been permanently deactivated
    Deactivated,
}

/// Creator tier determining fees and features
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatorTier {
    /// Free tier: 10% platform fee
    #[default]
    Free,
    /// Creator tier ($29/month): 7% platform fee
    Creator,
    /// Pro tier ($99/month): 5% platform fee
    Pro,
    /// Enterprise tier (custom pricing): Custom fee
    Enterprise,
}

impl CreatorTier {
    /// Get the platform fee percentage for this tier
    pub fn fee_percentage(&self) -> u64 {
        match self {
            CreatorTier::Free => 10,
            CreatorTier::Creator => 7,
            CreatorTier::Pro => 5,
            CreatorTier::Enterprise => 3,
        }
    }
}

/// Creator settings for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorSettings {
    /// Default price for new articles (in cents)
    pub default_price_cents: u64,
    /// Whether to show reading time estimates
    pub show_reading_time: bool,
    /// Custom branding color (hex)
    pub brand_color: Option<String>,
    /// Custom CTA text
    pub unlock_button_text: Option<String>,
}

impl Default for CreatorSettings {
    fn default() -> Self {
        Self {
            default_price_cents: 50, // $0.50 default
            show_reading_time: true,
            brand_color: None,
            unlock_button_text: None,
        }
    }
}

/// Domain Entity: Creator
///
/// Represents a content creator in the paywall system.
/// Creators sign up, configure their paywall settings, and receive payments.
///
/// Domain Rules:
/// - Creator must have a valid email
/// - Creator must have a valid wallet address for receiving payments
/// - Only active creators can receive payments
/// - Creator tier determines platform fee
/// - API key must be kept secure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    id: CreatorId,
    tenant_id: TenantId,
    email: String,
    name: Option<String>,
    wallet_address: WalletAddress,
    blog_url: Option<String>,
    status: CreatorStatus,
    tier: CreatorTier,
    settings: CreatorSettings,
    api_key_hash: Option<String>,
    email_verified: bool,
    total_revenue_cents: u64,
    total_articles: u32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    metadata: serde_json::Value,
}

impl Creator {
    /// Create a new creator with validation
    pub fn new(
        tenant_id: TenantId,
        email: String,
        wallet_address: WalletAddress,
        blog_url: Option<String>,
    ) -> Result<Self> {
        Self::validate_email(&email)?;

        if let Some(ref url) = blog_url {
            Self::validate_url(url)?;
        }

        let now = Utc::now();
        Ok(Self {
            id: CreatorId::new(),
            tenant_id,
            email,
            name: None,
            wallet_address,
            blog_url,
            status: CreatorStatus::Pending,
            tier: CreatorTier::Free,
            settings: CreatorSettings::default(),
            api_key_hash: None,
            email_verified: false,
            total_revenue_cents: 0,
            total_articles: 0,
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
        })
    }

    /// Reconstruct creator from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: CreatorId,
        tenant_id: TenantId,
        email: String,
        name: Option<String>,
        wallet_address: WalletAddress,
        blog_url: Option<String>,
        status: CreatorStatus,
        tier: CreatorTier,
        settings: CreatorSettings,
        api_key_hash: Option<String>,
        email_verified: bool,
        total_revenue_cents: u64,
        total_articles: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            email,
            name,
            wallet_address,
            blog_url,
            status,
            tier,
            settings,
            api_key_hash,
            email_verified,
            total_revenue_cents,
            total_articles,
            created_at,
            updated_at,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> &CreatorId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn wallet_address(&self) -> &WalletAddress {
        &self.wallet_address
    }

    pub fn blog_url(&self) -> Option<&str> {
        self.blog_url.as_deref()
    }

    pub fn status(&self) -> CreatorStatus {
        self.status
    }

    pub fn tier(&self) -> CreatorTier {
        self.tier
    }

    pub fn settings(&self) -> &CreatorSettings {
        &self.settings
    }

    pub fn api_key_hash(&self) -> Option<&str> {
        self.api_key_hash.as_deref()
    }

    pub fn is_email_verified(&self) -> bool {
        self.email_verified
    }

    pub fn total_revenue_cents(&self) -> u64 {
        self.total_revenue_cents
    }

    pub fn total_articles(&self) -> u32 {
        self.total_articles
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Check if creator is active and can receive payments
    pub fn is_active(&self) -> bool {
        self.status == CreatorStatus::Active && self.email_verified
    }

    /// Check if creator can receive payments
    pub fn can_receive_payments(&self) -> Result<()> {
        if !self.email_verified {
            return Err(crate::error::AllSourceError::ValidationError(
                "Email not verified".to_string(),
            ));
        }

        if self.status != CreatorStatus::Active {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Creator is not active (status: {:?})",
                self.status
            )));
        }

        Ok(())
    }

    /// Get the platform fee percentage for this creator
    pub fn fee_percentage(&self) -> u64 {
        self.tier.fee_percentage()
    }

    /// Verify the email address
    pub fn verify_email(&mut self) {
        self.email_verified = true;
        if self.status == CreatorStatus::Pending {
            self.status = CreatorStatus::Active;
        }
        self.updated_at = Utc::now();
    }

    /// Update the name
    pub fn update_name(&mut self, name: Option<String>) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    /// Update the wallet address
    pub fn update_wallet_address(&mut self, wallet_address: WalletAddress) {
        self.wallet_address = wallet_address;
        self.updated_at = Utc::now();
    }

    /// Update the blog URL
    pub fn update_blog_url(&mut self, blog_url: Option<String>) -> Result<()> {
        if let Some(ref url) = blog_url {
            Self::validate_url(url)?;
        }
        self.blog_url = blog_url;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update settings
    pub fn update_settings(&mut self, settings: CreatorSettings) {
        self.settings = settings;
        self.updated_at = Utc::now();
    }

    /// Set the API key hash
    pub fn set_api_key_hash(&mut self, hash: String) {
        self.api_key_hash = Some(hash);
        self.updated_at = Utc::now();
    }

    /// Upgrade tier
    pub fn upgrade_tier(&mut self, tier: CreatorTier) {
        self.tier = tier;
        self.updated_at = Utc::now();
    }

    /// Suspend the creator
    pub fn suspend(&mut self) {
        self.status = CreatorStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Reactivate a suspended creator
    pub fn reactivate(&mut self) -> Result<()> {
        if self.status == CreatorStatus::Deactivated {
            return Err(crate::error::AllSourceError::ValidationError(
                "Cannot reactivate a deactivated creator".to_string(),
            ));
        }

        if self.email_verified {
            self.status = CreatorStatus::Active;
        } else {
            self.status = CreatorStatus::Pending;
        }
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Permanently deactivate the creator
    pub fn deactivate(&mut self) {
        self.status = CreatorStatus::Deactivated;
        self.updated_at = Utc::now();
    }

    /// Record revenue from a transaction
    pub fn record_revenue(&mut self, amount_cents: u64) {
        self.total_revenue_cents += amount_cents;
        self.updated_at = Utc::now();
    }

    /// Increment article count
    pub fn increment_articles(&mut self) {
        self.total_articles += 1;
        self.updated_at = Utc::now();
    }

    /// Decrement article count
    pub fn decrement_articles(&mut self) {
        self.total_articles = self.total_articles.saturating_sub(1);
        self.updated_at = Utc::now();
    }

    /// Update metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = Utc::now();
    }

    // Validation

    fn validate_email(email: &str) -> Result<()> {
        if email.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Email cannot be empty".to_string(),
            ));
        }

        if !email.contains('@') || !email.contains('.') {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Invalid email format".to_string(),
            ));
        }

        if email.len() > 254 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Email cannot exceed 254 characters".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "URL cannot be empty".to_string(),
            ));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(crate::error::AllSourceError::InvalidInput(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        if url.len() > 2048 {
            return Err(crate::error::AllSourceError::InvalidInput(
                "URL cannot exceed 2048 characters".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_wallet() -> WalletAddress {
        WalletAddress::new(VALID_WALLET.to_string()).unwrap()
    }

    #[test]
    fn test_create_creator() {
        let creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            Some("https://blog.example.com".to_string()),
        );

        assert!(creator.is_ok());
        let creator = creator.unwrap();
        assert_eq!(creator.email(), "test@example.com");
        assert_eq!(creator.status(), CreatorStatus::Pending);
        assert!(!creator.is_email_verified());
    }

    #[test]
    fn test_reject_invalid_email() {
        let result = Creator::new(
            test_tenant_id(),
            "invalid-email".to_string(),
            test_wallet(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_empty_email() {
        let result = Creator::new(test_tenant_id(), "".to_string(), test_wallet(), None);

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_invalid_url() {
        let result = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            Some("not-a-url".to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_email_activates_creator() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        assert_eq!(creator.status(), CreatorStatus::Pending);
        assert!(!creator.is_active());

        creator.verify_email();

        assert_eq!(creator.status(), CreatorStatus::Active);
        assert!(creator.is_active());
    }

    #[test]
    fn test_can_receive_payments() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        // Cannot receive payments when not verified
        assert!(creator.can_receive_payments().is_err());

        // Verify and check again
        creator.verify_email();
        assert!(creator.can_receive_payments().is_ok());

        // Suspend and check again
        creator.suspend();
        assert!(creator.can_receive_payments().is_err());
    }

    #[test]
    fn test_fee_percentage() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        // Free tier: 10%
        assert_eq!(creator.fee_percentage(), 10);

        // Upgrade to Creator tier: 7%
        creator.upgrade_tier(CreatorTier::Creator);
        assert_eq!(creator.fee_percentage(), 7);

        // Upgrade to Pro tier: 5%
        creator.upgrade_tier(CreatorTier::Pro);
        assert_eq!(creator.fee_percentage(), 5);
    }

    #[test]
    fn test_suspend_and_reactivate() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        creator.verify_email();
        assert!(creator.is_active());

        creator.suspend();
        assert_eq!(creator.status(), CreatorStatus::Suspended);
        assert!(!creator.is_active());

        creator.reactivate().unwrap();
        assert_eq!(creator.status(), CreatorStatus::Active);
        assert!(creator.is_active());
    }

    #[test]
    fn test_cannot_reactivate_deactivated() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        creator.deactivate();
        assert!(creator.reactivate().is_err());
    }

    #[test]
    fn test_record_revenue() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        assert_eq!(creator.total_revenue_cents(), 0);

        creator.record_revenue(1000);
        assert_eq!(creator.total_revenue_cents(), 1000);

        creator.record_revenue(500);
        assert_eq!(creator.total_revenue_cents(), 1500);
    }

    #[test]
    fn test_article_count() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        assert_eq!(creator.total_articles(), 0);

        creator.increment_articles();
        assert_eq!(creator.total_articles(), 1);

        creator.increment_articles();
        assert_eq!(creator.total_articles(), 2);

        creator.decrement_articles();
        assert_eq!(creator.total_articles(), 1);
    }

    #[test]
    fn test_update_settings() {
        let mut creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        let new_settings = CreatorSettings {
            default_price_cents: 100,
            show_reading_time: false,
            brand_color: Some("#FF0000".to_string()),
            unlock_button_text: Some("Read Now".to_string()),
        };

        creator.update_settings(new_settings);

        assert_eq!(creator.settings().default_price_cents, 100);
        assert!(!creator.settings().show_reading_time);
        assert_eq!(creator.settings().brand_color, Some("#FF0000".to_string()));
    }

    #[test]
    fn test_serde_serialization() {
        let creator = Creator::new(
            test_tenant_id(),
            "test@example.com".to_string(),
            test_wallet(),
            None,
        )
        .unwrap();

        let json = serde_json::to_string(&creator);
        assert!(json.is_ok());

        let deserialized: Creator = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(deserialized.email(), "test@example.com");
    }
}
