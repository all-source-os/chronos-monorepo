use crate::error::AllsourceAuthError;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Convert a `snake_case` field name to `camelCase`. Idempotent for inputs
/// that have no underscores (e.g. `email`, `slug`, `credentialID`).
///
/// Used to bridge the gap between adapter callers, which pass snake_case
/// field names, and serialized payloads, whose keys are camelCase because
/// `better_auth_core`'s structs carry `#[serde(rename = "userId")]` etc.
fn to_camel_case(field: &str) -> String {
    if !field.contains('_') {
        return field.to_string();
    }
    let mut out = String::with_capacity(field.len());
    let mut upper_next = false;
    for ch in field.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Look up `field` on `payload`, trying the literal key first and then
/// its camelCase form. Returns `None` if neither resolves to a string.
fn payload_field_str<'a>(payload: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    payload
        .get(field)
        .and_then(|v| v.as_str())
        .or_else(|| {
            let camel = to_camel_case(field);
            if camel == field {
                None
            } else {
                payload.get(&camel).and_then(|v| v.as_str())
            }
        })
}

/// Build the `payload_filter` query value Core expects: a flat
/// `{"<field>": "<value>"}` map. Core's filter parser
/// (`apps/core/src/store.rs::matches_filter`) decodes the string as a
/// `Map<String, Value>` and requires `payload[k] == v` for every entry.
///
/// An earlier version of this client sent a structured
/// `{"field": "payload.<field>", "op": "eq", "value": <value>}` body. Core
/// interpreted that as "payload must contain literal keys `field`, `op`,
/// `value`", which never matches a real event, so every server-side filter
/// query returned an empty result. The adapter's scan fallback only fires on
/// HTTP 400/422, not on 200-with-empty, so `find_by_field` always returned
/// `None` — duplicating users on every signin (issue #187).
fn build_payload_filter(field: &str, value: &str) -> String {
    serde_json::Value::Object(
        [(field.to_string(), serde_json::Value::String(value.to_string()))]
            .into_iter()
            .collect(),
    )
    .to_string()
}

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
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        // `order=desc` so `limit=1` returns the NEWEST event. Without it Core
        // defaults to oldest-first and this returns the entity's first-ever
        // event — a stale payload missing later schema fields (issue #177).
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("entity_id", entity_id),
                ("limit", "1"),
                ("order", "desc"),
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", &limit.to_string()),
                ("order", "desc"),
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

        // Group by entity_id, take latest per entity. The `order=desc` param
        // above makes Core return newest-first, so the first event seen for
        // each entity_id is its current state.
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
        // Try the literal field name first, then the camelCase form if it
        // differs. Payloads are stored with `#[serde(rename = "userId")]`
        // style keys, so callers passing `user_id` need the camel retry.
        match self
            .find_by_field_filtered::<T>(event_type_prefix, field, value)
            .await?
        {
            Some(entity) => Ok(Some(entity)),
            None => {
                let camel = to_camel_case(field);
                if camel != field {
                    self.find_by_field_filtered::<T>(event_type_prefix, &camel, value)
                        .await
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Inner helper: issues exactly one server-side `payload_filter` query for
    /// `field`. On `400`/`422` (filter unsupported) it falls back to the
    /// case-tolerant in-memory scan.
    async fn find_by_field_filtered<T: DeserializeOwned>(
        &self,
        event_type_prefix: &str,
        field: &str,
        value: &str,
    ) -> Result<Option<T>, AllsourceAuthError> {
        let url = format!("{}/api/v1/events/query", self.query_url);

        let filter = build_payload_filter(field, value);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("payload_filter", &filter),
                ("limit", "1"),
                ("order", "desc"),
            ])
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            if status.as_u16() == 400 || status.as_u16() == 422 {
                return self
                    .find_by_field_scan(event_type_prefix, field, value)
                    .await;
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
                match serde_json::from_value::<T>(event.payload.clone()) {
                    Ok(entity) => Ok(Some(entity)),
                    Err(_) => {
                        // event_type_prefix filter may be broken, returning wrong event types.
                        // Fall back to in-memory scan which skips non-matching types gracefully.
                        self.find_by_field_scan(event_type_prefix, field, value)
                            .await
                    }
                }
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", "10000"),
                ("order", "desc"),
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

            let field_val = payload_field_str(&event.payload, field);
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
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("event_type_prefix", event_type_prefix),
                ("limit", "10000"),
                ("order", "desc"),
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

            let field_val = payload_field_str(&event.payload, field);
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

#[cfg(test)]
mod tests {
    use super::*;
    use better_auth_core::types::User;
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    #[test]
    fn to_camel_case_basic() {
        assert_eq!(to_camel_case("user_id"), "userId");
        assert_eq!(to_camel_case("organization_id"), "organizationId");
        assert_eq!(to_camel_case("created_at"), "createdAt");
        assert_eq!(to_camel_case("two_factor_id"), "twoFactorId");
    }

    #[test]
    fn to_camel_case_idempotent_for_camel_or_flat() {
        assert_eq!(to_camel_case("email"), "email");
        assert_eq!(to_camel_case("slug"), "slug");
        assert_eq!(to_camel_case("credentialID"), "credentialID");
        assert_eq!(to_camel_case("userId"), "userId");
    }

    #[test]
    fn payload_field_str_prefers_literal_then_camel() {
        // Camel-keyed payload (what better_auth_core actually serializes).
        let camel = serde_json::json!({
            "userId": "uid-1",
            "organizationId": "org-1",
            "email": "a@b",
        });
        // Snake caller keys resolve via the camel fallback.
        assert_eq!(payload_field_str(&camel, "user_id"), Some("uid-1"));
        assert_eq!(payload_field_str(&camel, "organization_id"), Some("org-1"));
        // Non-renamed keys still work.
        assert_eq!(payload_field_str(&camel, "email"), Some("a@b"));
        // Missing key returns None.
        assert_eq!(payload_field_str(&camel, "missing_field"), None);

        // Snake-keyed payload (hypothetical legacy data) still resolves.
        let snake = serde_json::json!({ "user_id": "uid-2" });
        assert_eq!(payload_field_str(&snake, "user_id"), Some("uid-2"));
    }

    /// Regression for issue #187. Core's `payload_filter` parser expects a
    /// flat `{field: value}` JSON map. The previous structured shape
    /// `{"field": "payload.email", "op": "eq", ...}` matched no events and
    /// made `find_by_field` return `None` for every signin lookup.
    #[test]
    fn build_payload_filter_emits_flat_map() {
        let raw = build_payload_filter("email", "alias+test1@example.com");
        let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!({ "email": "alias+test1@example.com" })
        );
    }

    #[test]
    fn build_payload_filter_preserves_special_chars() {
        // Make sure `+` and `@` survive serialization; URL encoding is the
        // HTTP client's job, not ours.
        let raw = build_payload_filter("email", "a+b@c.io");
        assert!(raw.contains("a+b@c.io"), "got {raw}");
    }

    /// End-to-end check that `find_by_field("email", ...)` sends Core a
    /// `payload_filter` whose JSON decodes to `{"email": "<value>"}`. This
    /// asserts the wire contract directly; if a future change re-wraps the
    /// filter we'll catch it here.
    #[tokio::test]
    async fn find_by_field_sends_flat_payload_filter() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Capture the inbound request line so we can assert on the query
        // string after the handler returns.
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let captured_clone = captured.clone();

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, mut write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await.unwrap();
            *captured_clone.lock().await = Some(request_line.clone());

            // Drain headers
            loop {
                let mut buf = String::new();
                reader.read_line(&mut buf).await.unwrap();
                if buf == "\r\n" || buf.is_empty() {
                    break;
                }
            }

            let body = r#"{"events":[]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            write_half.write_all(response.as_bytes()).await.unwrap();
            write_half.shutdown().await.ok();
        });

        let core_url = format!("http://{addr}");
        let client = AllsourceClient::new(&core_url, &core_url, "test-key");
        let _: Option<User> = client
            .find_by_field("auth.user.created", "email", "alias+test1@example.com")
            .await
            .unwrap();

        server.await.unwrap();
        let line = captured.lock().await.clone().expect("request not captured");

        // Extract the URL portion of the request line: `GET <url> HTTP/1.1`.
        let path_and_query = line
            .split_whitespace()
            .nth(1)
            .expect("request line malformed");
        let qs = path_and_query
            .split_once('?')
            .map(|(_, q)| q)
            .expect("query string missing");

        // Find `payload_filter=<value>` and decode it.
        let filter_raw = qs
            .split('&')
            .find_map(|pair| pair.strip_prefix("payload_filter="))
            .expect("payload_filter param missing");
        // `reqwest` percent-encodes the JSON; decode minimally for assertion.
        let decoded = percent_decode(filter_raw);
        let parsed: serde_json::Value = serde_json::from_str(&decoded)
            .unwrap_or_else(|e| panic!("payload_filter not valid JSON: {decoded:?} ({e})"));

        assert_eq!(
            parsed,
            serde_json::json!({ "email": "alias+test1@example.com" }),
            "regression for #187: filter must be a flat key/value map Core can match"
        );
    }

    /// Minimal percent-decoder for the assertion above; we only need to
    /// reverse what `reqwest`'s query encoder produced.
    fn percent_decode(input: &str) -> String {
        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
                    out.push(u8::from_str_radix(hex, 16).unwrap());
                    i += 3;
                }
                b => {
                    out.push(b);
                    i += 1;
                }
            }
        }
        String::from_utf8(out).unwrap()
    }
}
