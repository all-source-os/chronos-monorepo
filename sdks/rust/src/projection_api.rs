//! HTTP helpers on [`CoreClient`] for Core's projection-state and durable-consumer
//! endpoints. These power the [`ProjectionWorker`](crate::ws) infrastructure but
//! are also useful standalone for users who want to interact with Core's
//! projection KV or consumer registry directly.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{client::CoreClient, types::ConsumerState, Error};

impl CoreClient {
    /// Read a projection entity's state from Core.
    ///
    /// Returns `Ok(None)` when the projection or entity has no state.
    ///
    /// # Core behavior
    ///
    /// Core's `GET /api/v1/projections/:name/:entity_id/state` requires the
    /// projection to be registered in Core's projection manager. State written
    /// purely via [`Self::put_projection_state`] (without a matching registered
    /// projection) is not readable through this endpoint — it lives in a
    /// separate cache. For the common case (ProjectionWorker managing its own
    /// in-memory state), prefer reading through
    /// [`ProjectionHandle::get_state`](crate::projection_worker::ProjectionHandle::get_state).
    pub async fn get_projection_state<T: DeserializeOwned>(
        &self,
        name: &str,
        entity_id: &str,
    ) -> Result<Option<T>, Error> {
        let path = format!(
            "/api/v1/projections/{}/{}/state",
            urlencode(name),
            urlencode(entity_id)
        );
        let resp: Option<ProjectionStateResponse> = self.transport().get_optional(&path).await?;
        let Some(resp) = resp else {
            return Ok(None);
        };
        if !resp.found {
            return Ok(None);
        }
        match resp.state {
            Some(state) => Ok(Some(serde_json::from_value(state)?)),
            None => Ok(None),
        }
    }

    /// Write a projection entity's state to Core.
    ///
    /// Uses `PUT /api/v1/projections/:name/:entity_id/state`. The state is
    /// serialized as `{"state": <user_state>}` per Core's contract.
    pub async fn put_projection_state<T: Serialize>(
        &self,
        name: &str,
        entity_id: &str,
        state: &T,
    ) -> Result<(), Error> {
        let path = format!(
            "/api/v1/projections/{}/{}/state",
            urlencode(name),
            urlencode(entity_id)
        );
        let body = SaveStateRequest {
            state: serde_json::to_value(state)?,
        };
        let _: Value = self.transport().put(&path, &body).await?;
        Ok(())
    }

    /// Write multiple entity states in a single request.
    ///
    /// Uses `POST /api/v1/projections/:name/bulk/save`. Prefer this over looping
    /// [`Self::put_projection_state`] when flushing many entities at once —
    /// single request, single round-trip.
    pub async fn bulk_put_projection_state<T: Serialize>(
        &self,
        name: &str,
        entries: &[(String, T)],
    ) -> Result<(), Error> {
        let path = format!("/api/v1/projections/{}/bulk/save", urlencode(name));
        let states: Vec<BulkStateItem> = entries
            .iter()
            .map(|(entity_id, state)| {
                Ok(BulkStateItem {
                    entity_id: entity_id.clone(),
                    state: serde_json::to_value(state)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let body = BulkSaveRequest { states };
        let _: Value = self.transport().post(&path, &body).await?;
        Ok(())
    }

    /// Register a durable consumer with Core.
    ///
    /// Durable consumers track their own WAL position server-side; reconnecting
    /// to `/api/v1/events/stream?consumer_id=<name>` auto-replays all events
    /// since the last ack before switching to live delivery.
    pub async fn register_consumer(
        &self,
        consumer_id: &str,
        event_type_filters: &[String],
    ) -> Result<ConsumerState, Error> {
        let body = RegisterConsumerRequest {
            consumer_id: consumer_id.to_string(),
            event_type_filters: event_type_filters.to_vec(),
        };
        self.transport().post("/api/v1/consumers", &body).await
    }

    /// Fetch a consumer's current state (cursor position + filters).
    pub async fn get_consumer(&self, consumer_id: &str) -> Result<ConsumerState, Error> {
        let path = format!("/api/v1/consumers/{}", urlencode(consumer_id));
        self.transport().get(&path).await
    }

    /// Acknowledge events up to `position`, advancing the consumer's cursor.
    ///
    /// After ack, subsequent reconnections replay from `position + 1`.
    pub async fn ack_consumer(&self, consumer_id: &str, position: u64) -> Result<(), Error> {
        let path = format!("/api/v1/consumers/{}/ack", urlencode(consumer_id));
        let body = AckRequest { position };
        let _: Value = self.transport().post(&path, &body).await?;
        Ok(())
    }

    /// Persist a projection worker's checkpoint.
    ///
    /// Thin wrapper over [`Self::ack_consumer`] — the two concepts are the
    /// same primitive (a WAL offset we've committed to), and sharing the
    /// implementation keeps the SDK from reinventing position tracking.
    pub async fn save_checkpoint(&self, worker_name: &str, position: u64) -> Result<(), Error> {
        self.ack_consumer(worker_name, position).await
    }

    /// Load a projection worker's last checkpoint (returns `None` for fresh workers).
    pub async fn load_checkpoint(&self, worker_name: &str) -> Result<Option<u64>, Error> {
        let state = self.get_consumer(worker_name).await?;
        Ok(state.cursor_position)
    }
}

// --- Internal wire types ---

#[derive(Debug, serde::Deserialize)]
struct ProjectionStateResponse {
    #[serde(default)]
    state: Option<Value>,
    #[serde(default)]
    found: bool,
}

#[derive(Debug, Serialize)]
struct SaveStateRequest {
    state: Value,
}

#[derive(Debug, Serialize)]
struct BulkSaveRequest {
    states: Vec<BulkStateItem>,
}

#[derive(Debug, Serialize)]
struct BulkStateItem {
    entity_id: String,
    state: Value,
}

#[derive(Debug, Serialize)]
struct RegisterConsumerRequest {
    consumer_id: String,
    event_type_filters: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AckRequest {
    position: u64,
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    async fn make_client(server: &MockServer) -> CoreClient {
        CoreClient::new(&server.uri(), "test-key").unwrap()
    }

    #[tokio::test]
    async fn get_projection_state_returns_deserialized() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projections/assets/BTC/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projection": "assets",
                "entity_id": "BTC",
                "state": {"symbol": "BTC", "altname": "Bitcoin"},
                "found": true
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Asset {
            symbol: String,
            altname: String,
        }
        let result: Option<Asset> = client.get_projection_state("assets", "BTC").await.unwrap();
        assert_eq!(
            result,
            Some(Asset {
                symbol: "BTC".into(),
                altname: "Bitcoin".into()
            })
        );
    }

    #[tokio::test]
    async fn get_projection_state_not_found_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projections/assets/UNKNOWN/state"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": "not found"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let result: Option<serde_json::Value> = client
            .get_projection_state("assets", "UNKNOWN")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_projection_state_found_false_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projections/assets/BTC/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projection": "assets",
                "entity_id": "BTC",
                "state": null,
                "found": false
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let result: Option<serde_json::Value> =
            client.get_projection_state("assets", "BTC").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn put_projection_state_sends_state_wrapped() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/projections/assets/BTC/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projection": "assets",
                "entity_id": "BTC",
                "saved": true
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        #[derive(serde::Serialize)]
        struct Asset {
            symbol: String,
        }
        client
            .put_projection_state(
                "assets",
                "BTC",
                &Asset {
                    symbol: "BTC".into(),
                },
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn bulk_put_projection_state_sends_array() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projections/assets/bulk/save"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "saved": 2
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let entries = vec![
            ("BTC".to_string(), serde_json::json!({"sym": "BTC"})),
            ("ETH".to_string(), serde_json::json!({"sym": "ETH"})),
        ];
        client
            .bulk_put_projection_state("assets", &entries)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn register_consumer_returns_state() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/consumers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "consumer_id": "w1",
                "event_type_filters": ["asset.*"],
                "cursor_position": null
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let state = client
            .register_consumer("w1", &["asset.*".to_string()])
            .await
            .unwrap();
        assert_eq!(state.consumer_id, "w1");
        assert_eq!(state.cursor_position, None);
    }

    #[tokio::test]
    async fn ack_consumer_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/consumers/w1/ack"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok",
                "consumer_id": "w1",
                "position": 42
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        client.ack_consumer("w1", 42).await.unwrap();
    }

    #[tokio::test]
    async fn save_checkpoint_is_ack_alias() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/consumers/w1/ack"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        client.save_checkpoint("w1", 100).await.unwrap();
    }

    #[tokio::test]
    async fn load_checkpoint_returns_cursor_position() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/consumers/w1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "consumer_id": "w1",
                "event_type_filters": [],
                "cursor_position": 57
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let pos = client.load_checkpoint("w1").await.unwrap();
        assert_eq!(pos, Some(57));
    }

    #[tokio::test]
    async fn server_error_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/consumers"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": "internal"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        // Use a short-retry config so the test doesn't spin for the default retry budget.
        let err = client.register_consumer("w1", &[]).await.unwrap_err();
        assert!(matches!(
            err,
            Error::Api { status: 500, .. } | Error::CircuitOpen { .. }
        ));
    }
}
