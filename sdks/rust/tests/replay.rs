use allsource::QueryClient;
use serde_json::json;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn run_json() -> serde_json::Value {
    json!({
        "replay_id": "replay-1",
        "projection_name": "event-count",
        "status": "running",
        "started_at": "2026-08-14T10:00:00Z",
        "updated_at": "2026-08-14T10:00:01Z",
        "completed_at": null,
        "total_events": 42,
        "processed_events": 12,
        "failed_events": 0,
        "progress_percentage": 28.6,
        "events_per_second": 120.0,
        "error_message": null
    })
}

#[tokio::test]
async fn projection_replay_workflow_uses_tenant_scoped_query_service_routes() {
    let server = MockServer::start().await;
    let client = QueryClient::new(&server.uri(), "test-key").unwrap();

    Mock::given(method("POST"))
        .and(path("/api/replay/preview"))
        .and(body_json(json!({"projection_name": "event-count"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "projection_name": "event-count",
                "projection_title": "Event Count",
                "projection_kind": "counter",
                "projection_status": "ready",
                "current_entity_count": 1,
                "total_events": 42,
                "sampled_events": 42,
                "analysis_scope": "full",
                "event_type_distribution": [
                    {"event_type": "order.created", "count": 42, "share": 100.0}
                ],
                "sampled_entity_count": 7,
                "sampled_entities": [{"entity_id": "order-1", "event_count": 8}],
                "first_event_at": "2026-08-01T00:00:00Z",
                "last_event_at": "2026-08-14T00:00:00Z",
                "analyzed_at": "2026-08-14T10:00:00Z",
                "ready_to_replay": true,
                "checks": [],
                "warnings": []
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/replay"))
        .and(body_json(json!({"projection_name": "event-count"})))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({"data": run_json()})))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/replay"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [run_json()]})))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/replay/replay-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": run_json()})))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/replay/replay-1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": run_json()})))
        .mount(&server)
        .await;

    let analysis = client
        .analyze_projection_replay("event-count")
        .await
        .unwrap();
    assert_eq!(analysis.total_events, 42);
    assert!(analysis.ready_to_replay);

    let started = client.start_projection_replay("event-count").await.unwrap();
    assert_eq!(started.replay_id, "replay-1");

    assert_eq!(client.list_projection_replays().await.unwrap().len(), 1);
    assert_eq!(
        client
            .get_projection_replay("replay-1")
            .await
            .unwrap()
            .projection_name,
        "event-count"
    );
    assert_eq!(
        client
            .cancel_projection_replay("replay-1")
            .await
            .unwrap()
            .status,
        "running"
    );
}
