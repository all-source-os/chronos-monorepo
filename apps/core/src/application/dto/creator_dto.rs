use crate::domain::entities::{Creator, CreatorSettings, CreatorStatus, CreatorTier};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO for registering a new creator
#[derive(Debug, Deserialize)]
pub struct RegisterCreatorRequest {
    pub tenant_id: String,
    pub email: String,
    pub wallet_address: String,
    pub blog_url: Option<String>,
    pub name: Option<String>,
}

/// DTO for creator registration response
#[derive(Debug, Serialize)]
pub struct RegisterCreatorResponse {
    pub creator: CreatorDto,
}

/// DTO for updating a creator
#[derive(Debug, Deserialize)]
pub struct UpdateCreatorRequest {
    pub name: Option<String>,
    pub wallet_address: Option<String>,
    pub blog_url: Option<String>,
    pub settings: Option<CreatorSettingsDto>,
}

/// DTO for updating creator response
#[derive(Debug, Serialize)]
pub struct UpdateCreatorResponse {
    pub creator: CreatorDto,
}

/// DTO for upgrading creator tier
#[derive(Debug, Deserialize)]
pub struct UpgradeCreatorTierRequest {
    pub tier: CreatorTierDto,
}

/// DTO for listing creators response
#[derive(Debug, Serialize)]
pub struct ListCreatorsResponse {
    pub creators: Vec<CreatorDto>,
    pub count: usize,
}

/// DTO for creator settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorSettingsDto {
    pub default_price_cents: Option<u64>,
    pub show_reading_time: Option<bool>,
    pub brand_color: Option<String>,
    pub unlock_button_text: Option<String>,
}

impl From<&CreatorSettings> for CreatorSettingsDto {
    fn from(settings: &CreatorSettings) -> Self {
        Self {
            default_price_cents: Some(settings.default_price_cents),
            show_reading_time: Some(settings.show_reading_time),
            brand_color: settings.brand_color.clone(),
            unlock_button_text: settings.unlock_button_text.clone(),
        }
    }
}

impl From<CreatorSettingsDto> for CreatorSettings {
    fn from(dto: CreatorSettingsDto) -> Self {
        Self {
            default_price_cents: dto.default_price_cents.unwrap_or(50),
            show_reading_time: dto.show_reading_time.unwrap_or(true),
            brand_color: dto.brand_color,
            unlock_button_text: dto.unlock_button_text,
        }
    }
}

/// DTO for creator status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreatorStatusDto {
    Pending,
    Active,
    Suspended,
    Deactivated,
}

impl From<CreatorStatus> for CreatorStatusDto {
    fn from(status: CreatorStatus) -> Self {
        match status {
            CreatorStatus::Pending => CreatorStatusDto::Pending,
            CreatorStatus::Active => CreatorStatusDto::Active,
            CreatorStatus::Suspended => CreatorStatusDto::Suspended,
            CreatorStatus::Deactivated => CreatorStatusDto::Deactivated,
        }
    }
}

/// DTO for creator tier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreatorTierDto {
    Free,
    Creator,
    Pro,
    Enterprise,
}

impl From<CreatorTier> for CreatorTierDto {
    fn from(tier: CreatorTier) -> Self {
        match tier {
            CreatorTier::Free => CreatorTierDto::Free,
            CreatorTier::Creator => CreatorTierDto::Creator,
            CreatorTier::Pro => CreatorTierDto::Pro,
            CreatorTier::Enterprise => CreatorTierDto::Enterprise,
        }
    }
}

impl From<CreatorTierDto> for CreatorTier {
    fn from(dto: CreatorTierDto) -> Self {
        match dto {
            CreatorTierDto::Free => CreatorTier::Free,
            CreatorTierDto::Creator => CreatorTier::Creator,
            CreatorTierDto::Pro => CreatorTier::Pro,
            CreatorTierDto::Enterprise => CreatorTier::Enterprise,
        }
    }
}

/// DTO for a creator in responses
#[derive(Debug, Clone, Serialize)]
pub struct CreatorDto {
    pub id: Uuid,
    pub tenant_id: String,
    pub email: String,
    pub name: Option<String>,
    pub wallet_address: String,
    pub blog_url: Option<String>,
    pub status: CreatorStatusDto,
    pub tier: CreatorTierDto,
    pub settings: CreatorSettingsDto,
    pub email_verified: bool,
    pub total_revenue_cents: u64,
    pub total_articles: u32,
    pub fee_percentage: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Creator> for CreatorDto {
    fn from(creator: &Creator) -> Self {
        Self {
            id: creator.id().as_uuid(),
            tenant_id: creator.tenant_id().to_string(),
            email: creator.email().to_string(),
            name: creator.name().map(std::string::ToString::to_string),
            wallet_address: creator.wallet_address().to_string(),
            blog_url: creator.blog_url().map(std::string::ToString::to_string),
            status: creator.status().into(),
            tier: creator.tier().into(),
            settings: creator.settings().into(),
            email_verified: creator.is_email_verified(),
            total_revenue_cents: creator.total_revenue_cents(),
            total_articles: creator.total_articles(),
            fee_percentage: creator.fee_percentage(),
            created_at: creator.created_at(),
            updated_at: creator.updated_at(),
        }
    }
}

impl From<Creator> for CreatorDto {
    fn from(creator: Creator) -> Self {
        CreatorDto::from(&creator)
    }
}
