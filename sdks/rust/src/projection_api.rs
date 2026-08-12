//! HTTP helpers on [`CoreClient`] for Core's projection-state and durable-consumer
//! endpoints. These power the [`ProjectionWorker`](crate::ws) infrastructure but
//! are also useful standalone for users who want to interact with Core's
//! projection KV or consumer registry directly.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{
    client::CoreClient,
    types::{ConsumerEvent, ConsumerState},
    Error,
};

impl CoreClient {
    /// Read a projection entity's state from Core.
    ///
    /// Returns `Ok(None)` when the projection or entity has no state.
    ///
    /// # Core behavior
    ///
    /// Core's `GET /api/v1/projections/:name/:entity_id/state` resolves in two
    /// steps: the registered projection's own state first, then it falls back
    /// to the projection state cache. So state written with
    /// [`Self::put_projection_state`] / [`Self::bulk_put_projection_state`] is
    /// readable back through this endpoint — and through the bulk and summary
    /// endpoints — with no projection registered in Core's manager. That is the
    /// "compute client-side, serve from Core" shape: one worker folds and
    /// pushes, any number of stateless readers read.
    ///
    /// Requires Core ≥ 0.19.1. Older Core versions had no cache fallback and
    /// returned `found: false` for unregistered projections.
    ///
    /// On the node running the worker,
    /// [`ProjectionHandle::get_state`](crate::projection_worker::ProjectionHandle::get_state)
    /// is cheaper (no round-trip) — but it is in-process, so it only serves
    /// that one node.
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

    /// Read every entity's state for a projection in one request.
    ///
    /// Uses `GET /api/v1/projections/:name/state`. Use this when you need to
    /// enumerate a projection without knowing the entity ids up front; when you
    /// do know them, prefer [`Self::bulk_get_projection_states`].
    ///
    /// # Core behavior
    ///
    /// The summary is served from the projection state cache — the states
    /// written via [`Self::put_projection_state`] /
    /// [`Self::bulk_put_projection_state`]. Registration in Core's projection
    /// manager is not required, and an unwritten projection returns an empty
    /// vec rather than an error.
    pub async fn get_projection_state_summary<T: DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<Vec<(String, T)>, Error> {
        let page = self
            .get_projection_state_summary_paged::<T>(name, &ProjectionStateSummaryParams::new())
            .await?;
        Ok(page.states)
    }

    /// Read a bounded page of a projection's entity states.
    ///
    /// The paginating counterpart of [`Self::get_projection_state_summary`],
    /// which returns every entity in one unbounded response. This is the only
    /// endpoint that can *enumerate* a projection — [`Self::bulk_get_projection_states`]
    /// needs the ids up front — so a projection with one entry per tenant needs
    /// this to be usable at all.
    ///
    /// Entities are ordered by `entity_id`, so `offset` paging is stable across
    /// requests. The returned [`ProjectionStateSummaryPage`] carries `total`
    /// (the full match set, ignoring `limit`/`offset`) and `has_more`, so a
    /// caller can drive a loop to completion.
    ///
    /// ```no_run
    /// # use allsource::{CoreClient, ProjectionStateSummaryParams};
    /// # async fn f(client: CoreClient) -> Result<(), allsource::Error> {
    /// let mut offset = 0;
    /// loop {
    ///     let page = client
    ///         .get_projection_state_summary_paged::<serde_json::Value>(
    ///             "tenant_usage",
    ///             &ProjectionStateSummaryParams::new().limit(500).offset(offset),
    ///         )
    ///         .await?;
    ///     offset += page.states.len();
    ///     if !page.has_more {
    ///         break;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Protocol`] when the server ignores `limit` and answers
    /// with more states than were asked for. Core versions before issue #249
    /// did not implement these parameters, and Core's extractor drops unknown
    /// query fields silently rather than rejecting them — so against an older
    /// Core the request looks accepted and quietly returns the whole
    /// projection. Failing loudly is the point: issue #250 was exactly this
    /// shape, an ignored `offset` that turned a paginator into an infinite
    /// loop of duplicate pages.
    pub async fn get_projection_state_summary_paged<T: DeserializeOwned>(
        &self,
        name: &str,
        params: &ProjectionStateSummaryParams,
    ) -> Result<ProjectionStateSummaryPage<T>, Error> {
        let path = format!("/api/v1/projections/{}/state", urlencode(name));
        let query = params.to_query();
        let resp: ProjectionStateSummaryResponse = if query.is_empty() {
            self.transport().get(&path).await?
        } else {
            self.transport().get_with_query(&path, &query).await?
        };

        if let Some(limit) = params.limit {
            if resp.states.len() > limit {
                return Err(Error::Protocol(format!(
                    "projection state summary for {name:?}: asked for at most {limit} states, \
                     got {}. The server ignored `limit` — it predates issue #249, and Core \
                     drops unknown query parameters silently, so paging cannot make progress.",
                    resp.states.len()
                )));
            }
        }

        let states = resp
            .states
            .into_iter()
            .map(|item| Ok((item.entity_id, serde_json::from_value(item.state)?)))
            .collect::<Result<Vec<_>, Error>>()?;

        // `total`/`has_more` are absent on a pre-#249 Core. With no `limit` set
        // the whole projection came back, so the honest fallback is "this is
        // everything": total = what we hold, nothing more to fetch.
        let total = resp.total.unwrap_or(states.len());
        let has_more = resp.has_more.unwrap_or(false);

        Ok(ProjectionStateSummaryPage {
            states,
            total,
            has_more,
        })
    }

    /// Read many entities' states in a single request.
    ///
    /// Uses `POST /api/v1/projections/:name/bulk`. Entities with no state come
    /// back as `None`, in the order they were requested — the read counterpart
    /// of [`Self::bulk_put_projection_state`].
    pub async fn bulk_get_projection_states<T: DeserializeOwned>(
        &self,
        name: &str,
        entity_ids: &[String],
    ) -> Result<Vec<(String, Option<T>)>, Error> {
        let path = format!("/api/v1/projections/{}/bulk", urlencode(name));
        let body = BulkGetRequest {
            entity_ids: entity_ids.to_vec(),
        };
        let resp: BulkGetResponse = self.transport().post(&path, &body).await?;
        resp.states
            .into_iter()
            .map(|item| {
                let state = match item.state {
                    Some(state) if item.found => Some(serde_json::from_value(state)?),
                    _ => None,
                };
                Ok((item.entity_id, state))
            })
            .collect()
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

    /// Poll a durable consumer for events since its last ack.
    ///
    /// Uses `GET /api/v1/consumers/:id/events`. This is the pull-based
    /// alternative to [`EventStreamClient`](crate::EventStreamClient) for
    /// environments where a long-lived WebSocket is awkward. `limit` caps the
    /// batch size (Core defaults to 100 when omitted); the cursor only moves
    /// once you [`Self::ack_consumer`] the highest [`ConsumerEvent::position`]
    /// you processed, so an unacked poll returns the same events again.
    pub async fn poll_consumer_events(
        &self,
        consumer_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<ConsumerEvent>, Error> {
        let path = format!("/api/v1/consumers/{}/events", urlencode(consumer_id));
        let query: Vec<(&str, String)> = limit
            .map(|limit| vec![("limit", limit.to_string())])
            .unwrap_or_default();
        let resp: ConsumerEventsResponse = self.transport().get_with_query(&path, &query).await?;
        Ok(resp.events)
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

/// Bounds for [`CoreClient::get_projection_state_summary_paged`].
///
/// All fields are optional; sending none is identical to the unbounded
/// [`CoreClient::get_projection_state_summary`].
#[derive(Debug, Clone, Default)]
pub struct ProjectionStateSummaryParams {
    /// Maximum number of entity states to return. Unbounded when absent.
    pub limit: Option<usize>,
    /// Number of matching states to skip before applying `limit`.
    pub offset: Option<usize>,
    /// Return only entities whose id starts with this prefix — lets a caller
    /// walk one shard of the keyspace without enumerating the whole projection.
    pub entity_id_prefix: Option<String>,
}

impl ProjectionStateSummaryParams {
    /// Unbounded params; add constraints with the builders.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return at most `limit` states.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip `offset` matching states before applying `limit`.
    #[must_use]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Restrict to entities whose id starts with `prefix`.
    #[must_use]
    pub fn entity_id_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.entity_id_prefix = Some(prefix.into());
        self
    }

    /// Only the fields that are set, so an empty params object sends a bare URL
    /// and stays byte-identical to the pre-pagination request.
    fn to_query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::new();
        if let Some(limit) = self.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = self.offset {
            query.push(("offset", offset.to_string()));
        }
        if let Some(prefix) = &self.entity_id_prefix {
            query.push(("entity_id_prefix", prefix.clone()));
        }
        query
    }
}

/// One page of a projection's entity states.
#[derive(Debug, Clone)]
pub struct ProjectionStateSummaryPage<T> {
    /// The states in this page, ordered by `entity_id`.
    pub states: Vec<(String, T)>,
    /// Total number of matching entities, ignoring `limit`/`offset`.
    pub total: usize,
    /// Whether further entities remain beyond this page.
    pub has_more: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ProjectionStateSummaryResponse {
    #[serde(default)]
    states: Vec<SummaryStateItem>,
    /// Absent on a Core predating issue #249.
    #[serde(default)]
    total: Option<usize>,
    /// Absent on a Core predating issue #249.
    #[serde(default)]
    has_more: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct SummaryStateItem {
    entity_id: String,
    state: Value,
}

#[derive(Debug, Serialize)]
struct BulkGetRequest {
    entity_ids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct BulkGetResponse {
    #[serde(default)]
    states: Vec<BulkGetStateItem>,
}

#[derive(Debug, serde::Deserialize)]
struct BulkGetStateItem {
    entity_id: String,
    #[serde(default)]
    state: Option<Value>,
    #[serde(default)]
    found: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ConsumerEventsResponse {
    #[serde(default)]
    events: Vec<ConsumerEvent>,
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
        matchers::{body_json, method, path, query_param},
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
    async fn get_projection_state_summary_returns_all_entities() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projections/assets/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projection": "assets",
                "states": [
                    {"entity_id": "BTC", "state": {"symbol": "BTC"}},
                    {"entity_id": "ETH", "state": {"symbol": "ETH"}}
                ],
                "total": 2
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Asset {
            symbol: String,
        }
        let states: Vec<(String, Asset)> =
            client.get_projection_state_summary("assets").await.unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].0, "BTC");
        assert_eq!(states[0].1.symbol, "BTC");
        assert_eq!(states[1].0, "ETH");
    }

    #[tokio::test]
    async fn bulk_get_projection_states_sends_entity_ids_and_maps_missing() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projections/assets/bulk"))
            .and(body_json(serde_json::json!({
                "entity_ids": ["BTC", "MISSING"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "projection": "assets",
                "states": [
                    {"entity_id": "BTC", "state": {"symbol": "BTC"}, "found": true},
                    {"entity_id": "MISSING", "state": null, "found": false}
                ],
                "total": 2
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Asset {
            symbol: String,
        }
        let ids = vec!["BTC".to_string(), "MISSING".to_string()];
        let states: Vec<(String, Option<Asset>)> = client
            .bulk_get_projection_states("assets", &ids)
            .await
            .unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].0, "BTC");
        assert_eq!(states[0].1.as_ref().unwrap().symbol, "BTC");
        assert_eq!(states[1].0, "MISSING");
        assert!(states[1].1.is_none());
    }

    #[tokio::test]
    async fn poll_consumer_events_decodes_flattened_events() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/consumers/w1/events"))
            .and(query_param("limit", "2"))
            // Core flattens the event next to `position` (ConsumerEventDto).
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "events": [
                    {
                        "position": 1,
                        "id": "11111111-1111-4111-8111-111111111111",
                        "event_type": "user.created",
                        "entity_id": "user-1",
                        "tenant_id": "default",
                        "payload": {"name": "Ada"},
                        "metadata": null,
                        "timestamp": "2026-01-01T00:00:00Z",
                        "version": 1
                    }
                ],
                "count": 1
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let events = client.poll_consumer_events("w1", Some(2)).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].position, 1);
        assert_eq!(events[0].event.event_type, "user.created");
        assert_eq!(events[0].event.entity_id, "user-1");
    }

    #[tokio::test]
    async fn poll_consumer_events_omits_limit_when_unset() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/consumers/w1/events"))
            .and(wiremock::matchers::query_param_is_missing("limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "events": [],
                "count": 0
            })))
            .mount(&server)
            .await;

        let client = make_client(&server).await;
        let events = client.poll_consumer_events("w1", None).await.unwrap();
        assert!(events.is_empty());
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
