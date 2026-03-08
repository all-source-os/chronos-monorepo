use crate::error::AllsourceAuthError;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Low-level HTTP client for Allsource Core and Query Service.
#[derive(Clone)]
pub struct AllsourceClient {
    http: Client,
    core_url: String,
    query_url: String,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct IngestEvent<'a> {
    entity_id: &'a str,
    event_type: &'a str,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    events: Vec<StoredEvent>,
}

#[derive(Debug, Deserialize)]
pub struct StoredEvent {
    pub payload: serde_json::Value,
}

impl AllsourceClient {
    pub fn new(core_url: &str, query_url: &str, api_key: &str) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            core_url: core_url.trim_end_matches('/').to_string(),
            query_url: query_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// Append an event to Allsource Core.
    pub async fn append_event(
        &self,
        entity_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), AllsourceAuthError> {
        let url = format!("{}/api/v1/events", self.core_url);
        let event = IngestEvent {
            entity_id,
            event_type,
            payload,
        };

        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&event)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        Ok(())
    }

    /// Query the latest event for an entity and deserialize the payload.
    pub async fn get_latest<T: DeserializeOwned>(
        &self,
        entity_id: &str,
    ) -> Result<Option<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("entity_id", entity_id),
                ("limit", "1"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let query_resp: QueryResponse = resp.json().await?;

        match query_resp.events.first() {
            Some(event) => {
                // Check if this is a deletion event
                if let Some(deleted) = event.payload.get("_deleted") {
                    if deleted.as_bool().unwrap_or(false) {
                        return Ok(None);
                    }
                }
                let entity: T = serde_json::from_value(event.payload.clone())?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    /// Query all non-deleted entities of a given type by event_type prefix.
    /// Scans events, groups by entity_id, takes the latest per entity.
    pub async fn query_all<T: DeserializeOwned>(
        &self,
        event_type_prefix: &str,
        limit: usize,
    ) -> Result<Vec<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let query_resp: QueryResponse = resp.json().await?;
        let mut results = Vec::new();

        // Group by entity_id, take latest per entity (events come sorted by time desc)
        let mut seen = std::collections::HashSet::new();
        for event in &query_resp.events {
            let entity_id = event
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if seen.contains(entity_id) {
                continue;
            }
            seen.insert(entity_id.to_string());

            // Skip deleted entities
            if event
                .payload
                .get("_deleted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }

            if let Ok(entity) = serde_json::from_value::<T>(event.payload.clone()) {
                results.push(entity);
            }
        }

        Ok(results)
    }

    /// Search for entities matching a field value using payload filtering.
    pub async fn find_by_field<T: DeserializeOwned>(
        &self,
        event_type_prefix: &str,
        field: &str,
        value: &str,
    ) -> Result<Option<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let filter = serde_json::json!({
            "field": format!("payload.{}", field),
            "op": "eq",
            "value": value
        });

        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("payload_filter", &filter.to_string()),
                ("limit", "1"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            // If payload_filter is not supported, fall back to scanning
            if status.as_u16() == 400 || status.as_u16() == 422 {
                return self.find_by_field_scan(event_type_prefix, field, value).await;
            }
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let query_resp: QueryResponse = resp.json().await?;
        match query_resp.events.first() {
            Some(event) => {
                if event
                    .payload
                    .get("_deleted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    return Ok(None);
                }
                let entity: T = serde_json::from_value(event.payload.clone())?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    /// Fallback: scan all entities and filter in-memory.
    async fn find_by_field_scan<T: DeserializeOwned>(
        &self,
        event_type_prefix: &str,
        field: &str,
        value: &str,
    ) -> Result<Option<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", "10000"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let query_resp: QueryResponse = resp.json().await?;
        let mut seen = std::collections::HashSet::new();

        for event in &query_resp.events {
            let entity_id = event
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if seen.contains(entity_id) {
                continue;
            }
            seen.insert(entity_id.to_string());

            if event
                .payload
                .get("_deleted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }

            let field_val = event.payload.get(field).and_then(|v| v.as_str());
            if field_val == Some(value) {
                if let Ok(entity) = serde_json::from_value::<T>(event.payload.clone()) {
                    return Ok(Some(entity));
                }
            }
        }

        Ok(None)
    }

    /// Find all entities matching a field value.
    pub async fn find_all_by_field<T: DeserializeOwned>(
        &self,
        event_type_prefix: &str,
        field: &str,
        value: &str,
    ) -> Result<Vec<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", "10000"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let message = extract_error_message(resp).await;
            return Err(AllsourceAuthError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let query_resp: QueryResponse = resp.json().await?;
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for event in &query_resp.events {
            let entity_id = event
                .payload
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if seen.contains(entity_id) {
                continue;
            }
            seen.insert(entity_id.to_string());

            if event
                .payload
                .get("_deleted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }

            let field_val = event.payload.get(field).and_then(|v| v.as_str());
            if field_val == Some(value) {
                if let Ok(entity) = serde_json::from_value::<T>(event.payload.clone()) {
                    results.push(entity);
                }
            }
        }

        Ok(results)
    }

    /// Append a deletion marker event.
    pub async fn append_delete(
        &self,
        entity_id: &str,
        event_type: &str,
    ) -> Result<(), AllsourceAuthError> {
        self.append_event(
            entity_id,
            event_type,
            serde_json::json!({ "_deleted": true, "id": entity_id }),
        )
        .await
    }
}

/// Extract a human-readable error message from an API error response.
///
/// Core returns `{"error": "..."}`, so we parse the JSON and extract the
/// `error` field. Falls back to the raw response body if parsing fails.
async fn extract_error_message(resp: reqwest::Response) -> String {
    let body = resp.text().await.unwrap_or_default();
    // Try to extract structured error from Core's JSON response
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(msg) = json.get("error").and_then(|e| e.as_str()) {
            return msg.to_string();
        }
    }
    body
}
