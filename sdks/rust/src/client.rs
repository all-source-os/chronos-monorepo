use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use crate::{
    circuit_breaker::CircuitBreaker, error::Error, fold::EventFolder,
    normalize::normalize_event_type,
    paginate::{EntityPaginator, EventPaginator, DEFAULT_PAGE_SIZE},
    types::*,
};

/// Retry configuration for transient failures.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts. Default: 3.
    pub max_retries: u32,
    /// Base delay between retries. Default: 200ms.
    pub base_delay: Duration,
    /// Backoff multiplier. Default: 2.0.
    pub backoff_factor: f64,
    /// Maximum delay between retries. Default: 10s.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(200),
            backoff_factor: 2.0,
            max_delay: Duration::from_secs(10),
        }
    }
}

/// Full configuration for building a client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the service.
    pub base_url: String,
    /// API key for authentication.
    pub api_key: String,
    /// HTTP request timeout. Default: 30s.
    pub timeout: Duration,
    /// Retry configuration.
    pub retry: RetryConfig,
    /// Circuit breaker failure threshold. Default: 5.
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker recovery timeout. Default: 30s.
    pub circuit_breaker_recovery: Duration,
}

impl ClientConfig {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            timeout: Duration::from_secs(30),
            retry: RetryConfig::default(),
            circuit_breaker_threshold: 5,
            circuit_breaker_recovery: Duration::from_secs(30),
        }
    }
}

/// Shared HTTP transport with retry and circuit breaker.
#[derive(Debug)]
pub(crate) struct HttpTransport {
    pub(crate) base_url: String,
    http: reqwest::Client,
    retry: RetryConfig,
    circuit_breaker: CircuitBreaker,
}

impl HttpTransport {
    fn new(config: &ClientConfig) -> Result<Self, Error> {
        if config.base_url.is_empty() {
            return Err(Error::Config("base_url is required".into()));
        }
        if config.api_key.is_empty() {
            return Err(Error::Config("api_key is required".into()));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|e| Error::Config(format!("invalid API key: {e}")))?,
        );
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()?;

        Ok(Self {
            base_url: config.base_url.trim_end_matches('/').to_string(),
            http,
            retry: config.retry.clone(),
            circuit_breaker: CircuitBreaker::new(
                config.circuit_breaker_threshold,
                config.circuit_breaker_recovery,
            ),
        })
    }

    pub(crate) async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, Error> {
        self.with_retry(|| async {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.http.get(&url).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    async fn get_with_query<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, Error> {
        self.with_retry(|| async {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.http.get(&url).query(query).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    pub(crate) async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.with_retry(|| async {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.http.post(&url).json(body).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    pub(crate) async fn put<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, Error> {
        self.with_retry(|| async {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.http.put(&url).json(body).send().await?;
            self.handle_response(resp).await
        })
        .await
    }

    /// GET returning `None` on 404. Useful for "does this key exist" lookups
    /// where absence is a normal case, not an error.
    pub(crate) async fn get_optional<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<Option<T>, Error> {
        self.with_retry(|| async {
            let url = format!("{}{}", self.base_url, path);
            let resp = self.http.get(&url).send().await?;
            if resp.status().as_u16() == 404 {
                return Ok(None);
            }
            let value: T = self.handle_response(resp).await?;
            Ok(Some(value))
        })
        .await
    }

    async fn with_retry<T, F, Fut>(&self, f: F) -> Result<T, Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, Error>>,
    {
        // Check circuit breaker
        if let Err(retry_after) = self.circuit_breaker.check() {
            return Err(Error::CircuitOpen {
                retry_after_secs: retry_after,
            });
        }

        let mut last_error = None;
        for attempt in 0..=self.retry.max_retries {
            if attempt > 0 {
                let delay = self
                    .retry
                    .base_delay
                    .mul_f64(self.retry.backoff_factor.powi(attempt as i32 - 1));
                let delay = delay.min(self.retry.max_delay);
                tracing::debug!(attempt, delay_ms = delay.as_millis(), "retrying request");
                tokio::time::sleep(delay).await;
            }

            match f().await {
                Ok(result) => {
                    self.circuit_breaker.record_success();
                    return Ok(result);
                }
                Err(e) if e.is_retryable() && attempt < self.retry.max_retries => {
                    tracing::warn!(attempt, error = %e, "transient error, will retry");
                    self.circuit_breaker.record_failure();
                    last_error = Some(e);
                }
                Err(e) => {
                    if e.is_server_error() || matches!(e, Error::Http(_)) {
                        self.circuit_breaker.record_failure();
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Config("retry exhausted".into())))
    }

    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, Error> {
        let status = resp.status().as_u16();
        if !(200..300).contains(&status) {
            let body: Option<serde_json::Value> = resp.json().await.ok();
            let message = body
                .as_ref()
                .and_then(|b| b.get("error").and_then(|e| e.get("message")))
                .and_then(|m| m.as_str())
                .or_else(|| {
                    body.as_ref()
                        .and_then(|b| b.get("error"))
                        .and_then(|e| e.as_str())
                })
                .unwrap_or("Unknown error")
                .to_string();
            return Err(Error::Api {
                status,
                message,
                body,
            });
        }
        Ok(resp.json().await?)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QueryClient — reads from Query Service
// ═══════════════════════════════════════════════════════════════════════════

/// Client for the AllSource Query Service (port 3902).
///
/// Use this when your traffic goes through the gateway: multi-tenant, external
/// clients, per-tenant rate limits, or billing/quota enforcement. QS validates
/// the API key (120s ETS cache, then delegates to Core), applies tenant
/// context, and forwards to Core.
///
/// For co-located services where you control both sides and don't need those
/// gateway features, prefer [`CoreClient`] — one hop, no cache layer.
///
/// Cloning is cheap; the underlying `reqwest::Client` and its connection pool
/// are shared via `Arc`.
#[derive(Debug, Clone)]
pub struct QueryClient {
    transport: Arc<HttpTransport>,
}

impl QueryClient {
    /// Create a new Query Service client.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Error> {
        Self::with_config(ClientConfig::new(base_url, api_key))
    }

    /// Create with full configuration.
    #[allow(clippy::needless_pass_by_value)] // public API — taking ownership is idiomatic
    pub fn with_config(config: ClientConfig) -> Result<Self, Error> {
        Ok(Self {
            transport: Arc::new(HttpTransport::new(&config)?),
        })
    }

    /// Query events with filters.
    ///
    /// Uses Core's `/api/v1/events/query` endpoint. Results are ordered by
    /// `(timestamp, version)` — ascending (oldest first) by default, or set
    /// [`QueryEventsParams::order`] / [`QueryEventsParams::order_desc`] for
    /// newest-first.
    pub async fn query_events(
        &self,
        params: QueryEventsParams,
    ) -> Result<QueryEventsResponse, Error> {
        let pairs = params.to_query_pairs();
        self.transport
            .get_with_query("/api/v1/events/query", &pairs)
            .await
    }

    /// Get all events for a specific entity.
    pub async fn get_entity_events(&self, entity_id: &str) -> Result<QueryEventsResponse, Error> {
        self.query_events(QueryEventsParams::new().entity_id(entity_id))
            .await
    }

    /// Get all events of a specific type.
    pub async fn get_events_by_type(&self, event_type: &str) -> Result<QueryEventsResponse, Error> {
        self.query_events(QueryEventsParams::new().event_type(event_type))
            .await
    }

    /// List projections.
    pub async fn list_projections(&self) -> Result<ProjectionsResponse, Error> {
        self.transport.get("/api/v1/projections").await
    }

    /// Check health.
    pub async fn health(&self) -> Result<HealthResponse, Error> {
        self.transport.get("/health").await
    }

    /// List distinct entities, optionally filtered by event type prefix.
    ///
    /// Uses Core's `/api/v1/entities` endpoint. Entities are sorted by
    /// last-event time — most recently active first by default, or set
    /// [`ListEntitiesParams::order`] / [`ListEntitiesParams::order_asc`] for the
    /// opposite. The order is total (ties broken by `entity_id`), so
    /// `limit`/`offset` pagination is stable; see [`Self::list_entities_paged`].
    pub async fn list_entities(
        &self,
        params: ListEntitiesParams,
    ) -> Result<ListEntitiesResponse, Error> {
        let pairs = params.to_query_pairs();
        self.transport
            .get_with_query("/api/v1/entities", &pairs)
            .await
    }

    /// Auto-paginating cursor over [`Self::query_events`].
    ///
    /// Drives `limit`/`offset` paging for you: each [`EventPaginator::next_page`]
    /// fetches the next batch until the server reports no more. The `limit` set
    /// on `params` (default [`DEFAULT_PAGE_SIZE`]) becomes the per-page size;
    /// any `offset` on `params` is the starting offset.
    pub fn query_events_paged(&self, params: QueryEventsParams) -> EventPaginator {
        let page_size = params.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        EventPaginator::new(self.clone(), params, page_size)
    }

    /// Auto-paginating cursor over [`Self::list_entities`].
    ///
    /// Drives `limit`/`offset` paging for you: each [`EntityPaginator::next_page`]
    /// fetches the next batch until the server reports no more. The `limit` set
    /// on `params` (default [`DEFAULT_PAGE_SIZE`]) becomes the per-page size;
    /// any `offset` on `params` is the starting offset.
    pub fn list_entities_paged(&self, params: ListEntitiesParams) -> EntityPaginator {
        let page_size = params.limit.unwrap_or(DEFAULT_PAGE_SIZE);
        EntityPaginator::new(self.clone(), params, page_size)
    }

    /// Detect duplicate entities by grouping on payload field values.
    ///
    /// Uses Core's `/api/v1/entities/duplicates` endpoint.
    /// Requires `event_type_prefix` to scope the search (no full-store scans).
    pub async fn detect_duplicates(
        &self,
        event_type_prefix: &str,
        group_by: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<DetectDuplicatesResponse, Error> {
        let mut pairs: Vec<(&str, String)> = vec![
            ("event_type_prefix", event_type_prefix.to_string()),
            ("group_by", group_by.to_string()),
        ];
        if let Some(limit) = limit {
            pairs.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            pairs.push(("offset", offset.to_string()));
        }
        self.transport
            .get_with_query("/api/v1/entities/duplicates", &pairs)
            .await
    }

    /// Query events and fold them into domain state in one call.
    ///
    /// Convenience method that combines [`Self::query_events`] + [`crate::fold::fold_events`].
    pub async fn query_and_fold<F: EventFolder>(
        &self,
        params: QueryEventsParams,
    ) -> Result<Option<F::State>, Error> {
        let resp = self.query_events(params).await?;
        Ok(crate::fold::fold_events::<F>(&resp.events))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CoreClient — writes to Core
// ═══════════════════════════════════════════════════════════════════════════

/// Client for AllSource Core (port 3900).
///
/// Low-latency path: one hop, in-process DashMap key validation (~μs), no rate
/// limit or quota layer. Use for co-located services doing writes,
/// projection work, or anything latency-sensitive. Powers
/// [`ProjectionWorker`](crate::ProjectionWorker) internally.
///
/// For multi-tenant / external / billing-gated traffic, use [`QueryClient`]
/// instead — that goes through the Query Service which owns rate limiting
/// and quota enforcement.
///
/// Cloning is cheap; the underlying `reqwest::Client` and its connection pool
/// are shared via `Arc`. Constructing a fresh `CoreClient` per request is an
/// anti-pattern; construct once and clone as needed.
///
/// # Example
///
/// ```no_run
/// use allsource::{CoreClient, IngestEventInput};
/// use serde_json::json;
///
/// # async fn run() -> Result<(), allsource::Error> {
/// let core = CoreClient::new("http://localhost:3900", "ask_...")?;
/// core.ingest_event(IngestEventInput {
///     event_type: "order.placed".into(),
///     entity_id: "order-42".into(),
///     payload: json!({"total_usd": 19.99}),
///     metadata: None,
/// }).await?;
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct CoreClient {
    transport: Arc<HttpTransport>,
}

impl CoreClient {
    /// Create a new Core client.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Error> {
        Self::with_config(ClientConfig::new(base_url, api_key))
    }

    /// Create with full configuration.
    #[allow(clippy::needless_pass_by_value)] // public API — taking ownership is idiomatic
    pub fn with_config(config: ClientConfig) -> Result<Self, Error> {
        Ok(Self {
            transport: Arc::new(HttpTransport::new(&config)?),
        })
    }

    /// Shared transport handle, for in-crate modules that add more HTTP methods.
    pub(crate) fn transport(&self) -> &HttpTransport {
        &self.transport
    }

    /// Ingest a single event.
    ///
    /// Event types are automatically normalized to AllSource's `lowercase.dot.separated`
    /// convention. For example, `VerificationCreated` becomes `verification.created`.
    pub async fn ingest_event(&self, mut input: IngestEventInput) -> Result<IngestResponse, Error> {
        input.event_type = normalize_event_type(&input.event_type);
        self.transport.post("/api/v1/events", &input).await
    }

    /// Ingest a batch of events.
    ///
    /// Event types are automatically normalized to AllSource's `lowercase.dot.separated`
    /// convention. For example, `VerificationCreated` becomes `verification.created`.
    pub async fn ingest_batch(
        &self,
        mut events: Vec<IngestEventInput>,
    ) -> Result<BatchIngestResponse, Error> {
        for event in &mut events {
            event.event_type = normalize_event_type(&event.event_type);
        }
        #[derive(Serialize)]
        struct BatchRequest {
            events: Vec<IngestEventInput>,
        }
        self.transport
            .post("/api/v1/events/batch", &BatchRequest { events })
            .await
    }

    /// Check Core health.
    pub async fn health(&self) -> Result<HealthResponse, Error> {
        self.transport.get("/health").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        assert!(QueryClient::new("", "key").is_err());
        assert!(QueryClient::new("http://localhost:3902", "").is_err());
        assert!(QueryClient::new("http://localhost:3902", "test-key").is_ok());
    }

    #[test]
    fn test_core_client_config_validation() {
        assert!(CoreClient::new("", "key").is_err());
        assert!(CoreClient::new("http://localhost:3900", "").is_err());
        assert!(CoreClient::new("http://localhost:3900", "test-key").is_ok());
    }

    #[test]
    fn test_base_url_trailing_slash() {
        let client = QueryClient::new("http://localhost:3902/", "key").unwrap();
        assert_eq!(client.transport.base_url, "http://localhost:3902");
    }

    #[test]
    fn test_retry_config_defaults() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.base_delay, Duration::from_millis(200));
        assert_eq!(cfg.backoff_factor, 2.0);
    }
}
