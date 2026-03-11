use crate::{
    application::dto::{
        CreatorDto, CreatorTierDto, ListCreatorsResponse, RegisterCreatorRequest,
        RegisterCreatorResponse, UpdateCreatorRequest, UpdateCreatorResponse,
    },
    domain::{
        entities::{Creator, CreatorSettings},
        repositories::CreatorRepository,
        value_objects::{TenantId, WalletAddress},
    },
    error::Result,
};
use std::sync::Arc;

/// Use Case: Register Creator
///
/// Registers a new content creator in the paywall system.
/// This creates a new creator with pending status, awaiting email verification.
///
/// Responsibilities:
/// - Validate input (DTO validation)
/// - Check for duplicate email/wallet
/// - Create domain Creator entity (with domain validation)
/// - Persist via repository
/// - Return response DTO
pub struct RegisterCreatorUseCase {
    repository: Arc<dyn CreatorRepository>,
}

impl RegisterCreatorUseCase {
    pub fn new(repository: Arc<dyn CreatorRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        request: RegisterCreatorRequest,
    ) -> Result<RegisterCreatorResponse> {
        // Check for duplicate email
        if self.repository.email_exists(&request.email).await? {
            return Err(crate::error::AllSourceError::ValidationError(
                "Email is already registered".to_string(),
            ));
        }

        // Validate and create tenant ID
        let tenant_id = TenantId::new(request.tenant_id)?;

        // Validate and create wallet address
        let wallet_address = WalletAddress::new(request.wallet_address)?;

        // Check for duplicate wallet
        if self
            .repository
            .find_by_wallet(&wallet_address)
            .await?
            .is_some()
        {
            return Err(crate::error::AllSourceError::ValidationError(
                "Wallet address is already registered".to_string(),
            ));
        }

        // Create domain creator entity
        let mut creator = Creator::new(tenant_id, request.email, wallet_address, request.blog_url)?;

        // Set name if provided
        if let Some(name) = request.name {
            creator.update_name(Some(name));
        }

        // Persist via repository
        let creator = self.repository.create(creator).await?;

        Ok(RegisterCreatorResponse {
            creator: CreatorDto::from(&creator),
        })
    }
}

/// Use Case: Update Creator
///
/// Updates creator information including name, wallet, blog URL, and settings.
pub struct UpdateCreatorUseCase {
    repository: Arc<dyn CreatorRepository>,
}

impl UpdateCreatorUseCase {
    pub fn new(repository: Arc<dyn CreatorRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        mut creator: Creator,
        request: UpdateCreatorRequest,
    ) -> Result<UpdateCreatorResponse> {
        // Update name if provided
        if let Some(name) = request.name {
            creator.update_name(Some(name));
        }

        // Update wallet address if provided
        if let Some(wallet) = request.wallet_address {
            let wallet_address = WalletAddress::new(wallet)?;

            // Check for duplicate wallet (if different from current)
            if creator.wallet_address() != &wallet_address
                && self
                    .repository
                    .find_by_wallet(&wallet_address)
                    .await?
                    .is_some()
            {
                return Err(crate::error::AllSourceError::ValidationError(
                    "Wallet address is already registered to another creator".to_string(),
                ));
            }

            creator.update_wallet_address(wallet_address);
        }

        // Update blog URL if provided
        if let Some(blog_url) = request.blog_url {
            creator.update_blog_url(Some(blog_url))?;
        }

        // Update settings if provided
        if let Some(settings_dto) = request.settings {
            let settings = CreatorSettings {
                default_price_cents: settings_dto.default_price_cents.unwrap_or(50),
                show_reading_time: settings_dto.show_reading_time.unwrap_or(true),
                brand_color: settings_dto.brand_color,
                unlock_button_text: settings_dto.unlock_button_text,
            };
            creator.update_settings(settings);
        }

        // Persist changes
        self.repository.save(&creator).await?;

        Ok(UpdateCreatorResponse {
            creator: CreatorDto::from(&creator),
        })
    }
}

/// Use Case: Verify Creator Email
///
/// Marks the creator's email as verified, which activates the creator account.
pub struct VerifyCreatorEmailUseCase;

impl VerifyCreatorEmailUseCase {
    pub fn execute(mut creator: Creator) -> Result<CreatorDto> {
        creator.verify_email();
        Ok(CreatorDto::from(&creator))
    }
}

/// Use Case: Upgrade Creator Tier
///
/// Upgrades a creator to a new tier level.
pub struct UpgradeCreatorTierUseCase;

impl UpgradeCreatorTierUseCase {
    pub fn execute(mut creator: Creator, tier: CreatorTierDto) -> Result<CreatorDto> {
        creator.upgrade_tier(tier.into());
        Ok(CreatorDto::from(&creator))
    }
}

/// Use Case: Suspend Creator
///
/// Suspends a creator, preventing them from receiving payments.
pub struct SuspendCreatorUseCase;

impl SuspendCreatorUseCase {
    pub fn execute(mut creator: Creator) -> Result<CreatorDto> {
        creator.suspend();
        Ok(CreatorDto::from(&creator))
    }
}

/// Use Case: Reactivate Creator
///
/// Reactivates a suspended creator.
pub struct ReactivateCreatorUseCase;

impl ReactivateCreatorUseCase {
    pub fn execute(mut creator: Creator) -> Result<CreatorDto> {
        creator.reactivate()?;
        Ok(CreatorDto::from(&creator))
    }
}

/// Use Case: Deactivate Creator
///
/// Permanently deactivates a creator.
pub struct DeactivateCreatorUseCase;

impl DeactivateCreatorUseCase {
    pub fn execute(mut creator: Creator) -> Result<CreatorDto> {
        creator.deactivate();
        Ok(CreatorDto::from(&creator))
    }
}

/// Use Case: List Creators
///
/// Returns a list of creators.
pub struct ListCreatorsUseCase;

impl ListCreatorsUseCase {
    pub fn execute(creators: &[Creator]) -> ListCreatorsResponse {
        let creator_dtos: Vec<CreatorDto> = creators.iter().map(CreatorDto::from).collect();
        let count = creator_dtos.len();

        ListCreatorsResponse {
            creators: creator_dtos,
            count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        entities::{CreatorStatus, CreatorTier},
        repositories::CreatorQuery,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    struct MockCreatorRepository {
        creators: Mutex<Vec<Creator>>,
    }

    impl MockCreatorRepository {
        fn new() -> Self {
            Self {
                creators: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl CreatorRepository for MockCreatorRepository {
        async fn create(&self, creator: Creator) -> Result<Creator> {
            let mut creators = self.creators.lock().unwrap();
            creators.push(creator.clone());
            Ok(creator)
        }

        async fn save(&self, _creator: &Creator) -> Result<()> {
            Ok(())
        }

        async fn find_by_id(
            &self,
            _id: &crate::domain::value_objects::CreatorId,
        ) -> Result<Option<Creator>> {
            Ok(None)
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<Creator>> {
            let creators = self.creators.lock().unwrap();
            Ok(creators.iter().find(|c| c.email() == email).cloned())
        }

        async fn find_by_wallet(&self, wallet: &WalletAddress) -> Result<Option<Creator>> {
            let creators = self.creators.lock().unwrap();
            Ok(creators
                .iter()
                .find(|c| c.wallet_address() == wallet)
                .cloned())
        }

        async fn find_by_tenant(
            &self,
            _tenant_id: &TenantId,
            _limit: usize,
            _offset: usize,
        ) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }

        async fn find_active(&self, _limit: usize, _offset: usize) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }

        async fn count(&self) -> Result<usize> {
            Ok(self.creators.lock().unwrap().len())
        }

        async fn count_by_status(&self, _status: CreatorStatus) -> Result<usize> {
            Ok(0)
        }

        async fn delete(&self, _id: &crate::domain::value_objects::CreatorId) -> Result<bool> {
            Ok(false)
        }

        async fn query(&self, _query: &CreatorQuery) -> Result<Vec<Creator>> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_register_creator() {
        let repo = Arc::new(MockCreatorRepository::new());
        let use_case = RegisterCreatorUseCase::new(repo.clone());

        let request = RegisterCreatorRequest {
            tenant_id: "test-tenant".to_string(),
            email: "creator@example.com".to_string(),
            wallet_address: VALID_WALLET.to_string(),
            blog_url: Some("https://blog.example.com".to_string()),
            name: Some("Test Creator".to_string()),
        };

        let response = use_case.execute(request).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.creator.email, "creator@example.com");
        assert_eq!(response.creator.name, Some("Test Creator".to_string()));
    }

    #[tokio::test]
    async fn test_register_creator_duplicate_email() {
        let repo = Arc::new(MockCreatorRepository::new());
        let use_case = RegisterCreatorUseCase::new(repo.clone());

        // First registration
        let request = RegisterCreatorRequest {
            tenant_id: "test-tenant".to_string(),
            email: "creator@example.com".to_string(),
            wallet_address: VALID_WALLET.to_string(),
            blog_url: None,
            name: None,
        };
        use_case.execute(request).await.unwrap();

        // Second registration with same email
        let request2 = RegisterCreatorRequest {
            tenant_id: "test-tenant".to_string(),
            email: "creator@example.com".to_string(),
            wallet_address: "11111111111111111111111111111111".to_string(),
            blog_url: None,
            name: None,
        };

        let result = use_case.execute(request2).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_creator_email() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let creator =
            Creator::new(tenant_id, "test@example.com".to_string(), wallet, None).unwrap();

        assert!(!creator.is_email_verified());

        let result = VerifyCreatorEmailUseCase::execute(creator);
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert!(dto.email_verified);
    }

    #[test]
    fn test_upgrade_creator_tier() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let creator =
            Creator::new(tenant_id, "test@example.com".to_string(), wallet, None).unwrap();

        assert_eq!(creator.tier(), CreatorTier::Free);

        let result = UpgradeCreatorTierUseCase::execute(creator, CreatorTierDto::Pro);
        assert!(result.is_ok());

        let dto = result.unwrap();
        assert_eq!(dto.tier, CreatorTierDto::Pro);
        assert_eq!(dto.fee_percentage, 5);
    }

    #[test]
    fn test_suspend_and_reactivate() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let mut creator =
            Creator::new(tenant_id, "test@example.com".to_string(), wallet, None).unwrap();

        creator.verify_email();

        // Suspend
        let suspended = SuspendCreatorUseCase::execute(creator).unwrap();
        assert_eq!(
            suspended.status,
            crate::application::dto::CreatorStatusDto::Suspended
        );

        // Reactivate (need to get the entity back - in real code this comes from repo)
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let mut creator =
            Creator::new(tenant_id, "test@example.com".to_string(), wallet, None).unwrap();
        creator.verify_email();
        creator.suspend();

        let reactivated = ReactivateCreatorUseCase::execute(creator).unwrap();
        assert_eq!(
            reactivated.status,
            crate::application::dto::CreatorStatusDto::Active
        );
    }

    #[test]
    fn test_list_creators() {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let wallet1 = WalletAddress::new(VALID_WALLET.to_string()).unwrap();
        let wallet2 = WalletAddress::new("11111111111111111111111111111111".to_string()).unwrap();

        let creators = vec![
            Creator::new(
                tenant_id.clone(),
                "creator1@example.com".to_string(),
                wallet1,
                None,
            )
            .unwrap(),
            Creator::new(tenant_id, "creator2@example.com".to_string(), wallet2, None).unwrap(),
        ];

        let response = ListCreatorsUseCase::execute(&creators);
        assert_eq!(response.count, 2);
        assert_eq!(response.creators.len(), 2);
    }
}
