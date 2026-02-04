use crate::application::dto::{
    ArticleStatusDto, BlockchainDto, CreatorStatusDto, CreatorTierDto, ForkStatusDto,
    PaginationRequest, SortRequest, TimeRangeFilter, TransactionStatusDto,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Base filter trait for common filter operations
pub trait FilterRequest {
    fn pagination(&self) -> &PaginationRequest;
    fn sort(&self) -> Option<&SortRequest>;
    fn time_range(&self) -> Option<&TimeRangeFilter>;
}

/// Filter for querying articles
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArticleFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by creator
    pub creator_id: Option<String>,

    /// Filter by status
    pub status: Option<ArticleStatusDto>,

    /// Filter by price range (min)
    pub min_price_cents: Option<i64>,

    /// Filter by price range (max)
    pub max_price_cents: Option<i64>,

    /// Search in title
    pub title_contains: Option<String>,

    /// Filter purchasable only
    pub purchasable_only: Option<bool>,

    /// Time range filter
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl ArticleFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_creator(mut self, creator_id: impl Into<String>) -> Self {
        self.creator_id = Some(creator_id.into());
        self
    }

    pub fn by_status(mut self, status: ArticleStatusDto) -> Self {
        self.status = Some(status);
        self
    }

    pub fn purchasable(mut self) -> Self {
        self.purchasable_only = Some(true);
        self
    }

    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.pagination = PaginationRequest::new(limit, offset);
        self
    }
}

impl FilterRequest for ArticleFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Filter for querying creators
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatorFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by status
    pub status: Option<CreatorStatusDto>,

    /// Filter by tier
    pub tier: Option<CreatorTierDto>,

    /// Filter by email verified status
    pub email_verified: Option<bool>,

    /// Search in email
    pub email_contains: Option<String>,

    /// Search in name
    pub name_contains: Option<String>,

    /// Minimum total revenue
    pub min_revenue_cents: Option<i64>,

    /// Time range filter
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl CreatorFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_status(mut self, status: CreatorStatusDto) -> Self {
        self.status = Some(status);
        self
    }

    pub fn by_tier(mut self, tier: CreatorTierDto) -> Self {
        self.tier = Some(tier);
        self
    }

    pub fn verified_only(mut self) -> Self {
        self.email_verified = Some(true);
        self
    }
}

impl FilterRequest for CreatorFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Filter for querying transactions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactionFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by article
    pub article_id: Option<String>,

    /// Filter by creator
    pub creator_id: Option<String>,

    /// Filter by reader wallet
    pub reader_wallet: Option<String>,

    /// Filter by status
    pub status: Option<TransactionStatusDto>,

    /// Filter by blockchain
    pub blockchain: Option<BlockchainDto>,

    /// Minimum amount
    pub min_amount_cents: Option<i64>,

    /// Maximum amount
    pub max_amount_cents: Option<i64>,

    /// Time range filter
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl TransactionFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_article(mut self, article_id: impl Into<String>) -> Self {
        self.article_id = Some(article_id.into());
        self
    }

    pub fn by_creator(mut self, creator_id: impl Into<String>) -> Self {
        self.creator_id = Some(creator_id.into());
        self
    }

    pub fn by_status(mut self, status: TransactionStatusDto) -> Self {
        self.status = Some(status);
        self
    }

    pub fn by_blockchain(mut self, blockchain: BlockchainDto) -> Self {
        self.blockchain = Some(blockchain);
        self
    }

    pub fn in_date_range(mut self, since: DateTime<Utc>, until: DateTime<Utc>) -> Self {
        self.time_range = Some(TimeRangeFilter::new().since(since).until(until));
        self
    }
}

impl FilterRequest for TransactionFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Filter for querying access tokens
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessTokenFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by article
    pub article_id: Option<String>,

    /// Filter by creator
    pub creator_id: Option<String>,

    /// Filter by reader wallet
    pub reader_wallet: Option<String>,

    /// Filter by valid status
    pub is_valid: Option<bool>,

    /// Filter by expired status
    pub is_expired: Option<bool>,

    /// Filter by revoked status
    pub is_revoked: Option<bool>,

    /// Time range filter (by issued_at)
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl AccessTokenFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_article(mut self, article_id: impl Into<String>) -> Self {
        self.article_id = Some(article_id.into());
        self
    }

    pub fn by_reader(mut self, wallet: impl Into<String>) -> Self {
        self.reader_wallet = Some(wallet.into());
        self
    }

    pub fn valid_only(mut self) -> Self {
        self.is_valid = Some(true);
        self.is_expired = Some(false);
        self.is_revoked = Some(false);
        self
    }

    pub fn expired_only(mut self) -> Self {
        self.is_expired = Some(true);
        self
    }
}

impl FilterRequest for AccessTokenFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Filter for querying forks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForkFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by status
    pub status: Option<ForkStatusDto>,

    /// Filter by parent fork
    pub parent_fork_id: Option<String>,

    /// Filter by creating agent
    pub created_by_agent: Option<String>,

    /// Filter by name containing
    pub name_contains: Option<String>,

    /// Include expired forks
    pub include_expired: Option<bool>,

    /// Time range filter (by created_at)
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl ForkFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn by_status(mut self, status: ForkStatusDto) -> Self {
        self.status = Some(status);
        self
    }

    pub fn by_agent(mut self, agent: impl Into<String>) -> Self {
        self.created_by_agent = Some(agent.into());
        self
    }

    pub fn active_only(mut self) -> Self {
        self.status = Some(ForkStatusDto::Active);
        self.include_expired = Some(false);
        self
    }
}

impl FilterRequest for ForkFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Filter for querying events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    /// Filter by tenant
    pub tenant_id: Option<String>,

    /// Filter by entity ID
    pub entity_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<String>,

    /// Filter by multiple event types
    pub event_types: Option<Vec<String>>,

    /// Filter by minimum version
    pub min_version: Option<i64>,

    /// Filter by maximum version
    pub max_version: Option<i64>,

    /// Time-travel: get events as of this timestamp
    pub as_of: Option<DateTime<Utc>>,

    /// Time range filter
    #[serde(flatten)]
    pub time_range: Option<TimeRangeFilter>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,

    /// Sort options
    pub sort: Option<SortRequest>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by_entity(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }

    pub fn by_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn by_types(mut self, types: Vec<String>) -> Self {
        self.event_types = Some(types);
        self
    }

    pub fn at_time(mut self, as_of: DateTime<Utc>) -> Self {
        self.as_of = Some(as_of);
        self
    }
}

impl FilterRequest for EventFilter {
    fn pagination(&self) -> &PaginationRequest {
        &self.pagination
    }

    fn sort(&self) -> Option<&SortRequest> {
        self.sort.as_ref()
    }

    fn time_range(&self) -> Option<&TimeRangeFilter> {
        self.time_range.as_ref()
    }
}

/// Full-text search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Search query string
    pub query: String,

    /// Entity types to search (articles, creators, etc.)
    pub entity_types: Option<Vec<String>>,

    /// Tenant to search within
    pub tenant_id: Option<String>,

    /// Pagination
    #[serde(flatten)]
    pub pagination: PaginationRequest,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            entity_types: None,
            tenant_id: None,
            pagination: PaginationRequest::default(),
        }
    }

    pub fn in_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn of_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = Some(types);
        self
    }
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Entity type (article, creator, etc.)
    pub entity_type: String,

    /// Entity ID
    pub entity_id: String,

    /// Display title/name
    pub title: String,

    /// Snippet showing matched content
    pub snippet: Option<String>,

    /// Relevance score (0-1)
    pub score: f64,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search results
    pub results: Vec<SearchResultItem>,

    /// Total matching results
    pub total: usize,

    /// Query that was executed
    pub query: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_filter_builder() {
        let filter = ArticleFilter::new()
            .by_creator("creator-1")
            .by_status(ArticleStatusDto::Active)
            .purchasable()
            .with_pagination(10, 0);

        assert_eq!(filter.creator_id, Some("creator-1".to_string()));
        assert_eq!(filter.status, Some(ArticleStatusDto::Active));
        assert_eq!(filter.purchasable_only, Some(true));
        assert_eq!(filter.pagination.limit, 10);
    }

    #[test]
    fn test_transaction_filter_date_range() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);

        let filter = TransactionFilter::new()
            .by_status(TransactionStatusDto::Confirmed)
            .in_date_range(yesterday, now);

        assert_eq!(filter.status, Some(TransactionStatusDto::Confirmed));
        assert!(filter.time_range.is_some());
    }

    #[test]
    fn test_search_request_builder() {
        let search = SearchRequest::new("test query")
            .in_tenant("tenant-1")
            .of_types(vec!["article".to_string(), "creator".to_string()]);

        assert_eq!(search.query, "test query");
        assert_eq!(search.tenant_id, Some("tenant-1".to_string()));
        assert_eq!(search.entity_types.unwrap().len(), 2);
    }

    #[test]
    fn test_access_token_filter_valid_only() {
        let filter = AccessTokenFilter::new()
            .by_article("article-1")
            .valid_only();

        assert_eq!(filter.article_id, Some("article-1".to_string()));
        assert_eq!(filter.is_valid, Some(true));
        assert_eq!(filter.is_expired, Some(false));
        assert_eq!(filter.is_revoked, Some(false));
    }

    #[test]
    fn test_fork_filter_active_only() {
        let filter = ForkFilter::new().by_tenant("tenant-1").active_only();

        assert_eq!(filter.status, Some(ForkStatusDto::Active));
        assert_eq!(filter.include_expired, Some(false));
    }
}
