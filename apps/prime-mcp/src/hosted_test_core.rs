//! A **stateful** fake `AllSource` Core for hosted-path tests.
//!
//! `wiremock`'s static responses can't model write-then-read-back: a `POST
//! /api/v1/events` followed by a `GET /api/v1/events/query` should see the event
//! that was just written, the way a real Core does. Since commit `3c07e6b`
//! (t-d90426) a *cold*-tenant write no longer updates the warm cache in place —
//! it relies on the next read re-hydrating from Core — so the old always-empty
//! mock made those round-trips fail. This helper stores posted events and returns
//! them on query, restoring a faithful round-trip without an embedded store.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

#[derive(Clone, Default)]
struct CoreEvents {
    events: Arc<Mutex<Vec<Value>>>,
}

/// `POST /api/v1/events` — wrap the ingest body `{event_type, entity_id, payload,
/// metadata?}` into a queryable `EventView` (adding id/timestamp/version) and store it.
struct Ingest(CoreEvents);
impl Respond for Ingest {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        if let Ok(body) = serde_json::from_slice::<Value>(&req.body) {
            let mut events = self.0.events.lock().unwrap();
            let n = events.len();
            events.push(json!({
                "id": format!("00000000-0000-0000-0000-{:012}", n + 1),
                "event_type": body.get("event_type").cloned().unwrap_or(Value::Null),
                "entity_id": body.get("entity_id").cloned().unwrap_or(Value::Null),
                "tenant_id": body.get("tenant_id").cloned().unwrap_or(Value::Null),
                "payload": body.get("payload").cloned().unwrap_or_else(|| json!({})),
                "metadata": body.get("metadata").cloned().unwrap_or(Value::Null),
                // Strictly increasing so Core's (timestamp, version) order is stable.
                "timestamp": format!("2026-06-06T00:{:02}:{:02}Z", n / 60, n % 60),
                "version": 1,
            }));
        }
        ResponseTemplate::new(200).set_body_json(json!({ "ok": true }))
    }
}

/// `GET /api/v1/events/query` — return every stored event. Tests are single-tenant
/// and write a couple of events (< any batch limit), so one query suffices.
struct Query(CoreEvents);
impl Respond for Query {
    fn respond(&self, _req: &Request) -> ResponseTemplate {
        let events = self.0.events.lock().unwrap().clone();
        let count = events.len();
        ResponseTemplate::new(200).set_body_json(json!({ "events": events, "count": count }))
    }
}

/// Mount a stateful Core on `server`: posted events are stored and returned by the
/// query endpoint, so hosted write→read-back round-trips like a real Core.
pub async fn mount_stateful_core(server: &MockServer) {
    let events = CoreEvents::default();
    Mock::given(method("POST"))
        .and(path("/api/v1/events"))
        .respond_with(Ingest(events.clone()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/events/query"))
        .respond_with(Query(events))
        .mount(server)
        .await;
}
