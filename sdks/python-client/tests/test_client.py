"""Tests for the synchronous AllSource client."""

import json

import httpx
import pytest
from pytest_httpx import HTTPXMock

from allsource_client import AllSourceClient, AllSourceError


BASE_URL = "http://localhost:3902"


@pytest.fixture
def client() -> AllSourceClient:
    return AllSourceClient(api_key="test-key", base_url=BASE_URL)


class TestIngest:
    def test_ingest_event(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events",
            method="POST",
            json={
                "data": {
                    "id": "evt-1",
                    "entity_id": "user-123",
                    "event_type": "user.signup",
                    "payload": {"plan": "pro"},
                    "timestamp": "2026-01-01T00:00:00Z",
                    "version": 1,
                }
            },
        )

        event = client.ingest("user.signup", "user-123", {"plan": "pro"})

        assert event.id == "evt-1"
        assert event.entity_id == "user-123"
        assert event.event_type == "user.signup"
        assert event.payload == {"plan": "pro"}

        request = httpx_mock.get_request()
        assert request is not None
        assert request.headers["Authorization"] == "Bearer test-key"
        body = json.loads(request.content)
        assert body["event_type"] == "user.signup"
        assert body["entity_id"] == "user-123"
        assert body["payload"] == {"plan": "pro"}

    def test_ingest_batch(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events/batch",
            method="POST",
            json={
                "data": [
                    {
                        "id": "evt-1",
                        "entity_id": "user-1",
                        "event_type": "user.signup",
                        "payload": {},
                        "timestamp": "2026-01-01T00:00:00Z",
                        "version": 1,
                    },
                    {
                        "id": "evt-2",
                        "entity_id": "user-2",
                        "event_type": "user.signup",
                        "payload": {},
                        "timestamp": "2026-01-01T00:00:01Z",
                        "version": 1,
                    },
                ]
            },
        )

        events = client.ingest_batch([
            {"event_type": "user.signup", "entity_id": "user-1", "payload": {}},
            {"event_type": "user.signup", "entity_id": "user-2", "payload": {}},
        ])

        assert len(events) == 2
        assert events[0].id == "evt-1"
        assert events[1].id == "evt-2"


class TestQuery:
    def test_query_events(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=httpx.URL(f"{BASE_URL}/api/events", params={"event_type": "user.signup", "limit": "10"}),
            method="GET",
            json={
                "data": {
                    "data": [
                        {
                            "id": "evt-1",
                            "entity_id": "user-123",
                            "event_type": "user.signup",
                            "payload": {},
                            "timestamp": "2026-01-01T00:00:00Z",
                            "version": 1,
                        }
                    ],
                    "count": 1,
                }
            },
        )

        result = client.query(event_type="user.signup", limit=10)

        assert result.count == 1
        assert len(result.events) == 1
        assert result.events[0].event_type == "user.signup"

    def test_query_by_entity(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events/entity/user-123",
            method="GET",
            json={
                "data": {"data": [], "count": 0}
            },
        )

        result = client.get_events_by_entity("user-123")
        assert result.count == 0
        assert result.events == []

    def test_query_by_type(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events/type/user.signup",
            method="GET",
            json={
                "data": {"data": [], "count": 0}
            },
        )

        result = client.get_events_by_type("user.signup")
        assert result.count == 0


class TestProjections:
    def test_list_projections(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/projections",
            method="GET",
            json={
                "data": [
                    {
                        "id": "proj-1",
                        "name": "user-count",
                        "version": 1,
                        "status": "running",
                        "initial_state": {"count": 0},
                        "definition": "count(*)",
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:00:00Z",
                    }
                ]
            },
        )

        projections = client.get_projections()

        assert len(projections) == 1
        assert projections[0].name == "user-count"
        assert projections[0].status == "running"

    def test_get_projection(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/projections/user-count",
            method="GET",
            json={
                "data": {
                    "id": "proj-1",
                    "name": "user-count",
                    "version": 1,
                    "status": "running",
                    "initial_state": {},
                    "definition": "count(*)",
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                }
            },
        )

        projection = client.get_projection("user-count")
        assert projection.name == "user-count"

    def test_create_projection(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/projections",
            method="POST",
            json={
                "data": {
                    "id": "proj-2",
                    "name": "order-totals",
                    "version": 1,
                    "status": "running",
                    "initial_state": {"total": 0},
                    "definition": "sum(payload.amount)",
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                }
            },
        )

        projection = client.create_projection(
            "order-totals",
            "sum(payload.amount)",
            initial_state={"total": 0},
        )
        assert projection.name == "order-totals"


class TestProjectionReplay:
    def test_analyze_projection_replay(
        self, client: AllSourceClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay/preview",
            method="POST",
            json={
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
                    "sampled_entity_count": 10,
                    "sampled_entities": [
                        {"entity_id": "order-1", "event_count": 4}
                    ],
                    "first_event_at": "2026-08-01T10:00:00Z",
                    "last_event_at": "2026-08-02T10:00:00Z",
                    "analyzed_at": "2026-08-14T10:00:00Z",
                    "ready_to_replay": True,
                    "checks": [
                        {
                            "key": "tenant_scope",
                            "label": "Tenant boundary",
                            "status": "pass",
                            "detail": "Authenticated tenant only.",
                        }
                    ],
                    "warnings": [],
                }
            },
        )

        analysis = client.analyze_projection_replay("event-count")

        assert analysis.total_events == 42
        assert analysis.ready_to_replay is True
        assert analysis.event_type_distribution[0].share == 100.0

        request = httpx_mock.get_request()
        assert request is not None
        assert json.loads(request.content) == {"projection_name": "event-count"}

    def test_projection_replay_lifecycle(
        self, client: AllSourceClient, httpx_mock: HTTPXMock
    ) -> None:
        run = {
            "replay_id": "replay-1",
            "projection_name": "event-count",
            "status": "running",
            "started_at": "2026-08-14T10:00:00Z",
            "updated_at": "2026-08-14T10:00:01Z",
            "completed_at": None,
            "total_events": 42,
            "processed_events": 12,
            "failed_events": 0,
            "progress_percentage": 28.6,
            "events_per_second": 12.0,
            "error_message": None,
        }
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay", method="POST", json={"data": run}
        )
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay", method="GET", json={"data": [run]}
        )
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay/replay-1", method="GET", json={"data": run}
        )
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay/replay-1/cancel",
            method="POST",
            json={"data": {**run, "status": "cancelled"}},
        )

        started = client.start_projection_replay("event-count")
        listed = client.list_projection_replays()
        fetched = client.get_projection_replay("replay-1")
        cancelled = client.cancel_projection_replay("replay-1")

        assert started.replay_id == "replay-1"
        assert listed[0].processed_events == 12
        assert fetched.projection_name == "event-count"
        assert cancelled.status == "cancelled"


class TestPrime:
    def test_list_prime_projections(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/projections",
            method="GET",
            json={
                "data": [
                    {
                        "entity_type": "person",
                        "field_policies": {"name": "last_write", "tags": "merge_array"},
                    }
                ],
                "count": 1,
            },
        )

        projections = client.list_prime_projections()

        assert len(projections) == 1
        assert projections[0].entity_type == "person"
        assert projections[0].field_policies["name"] == "last_write"
        assert projections[0].field_policies["tags"] == "merge_array"

    def test_define_prime_projection(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/projections",
            method="POST",
            status_code=201,
            json={"data": {"entity_type": "person", "persisted": True}},
        )

        ack = client.define_prime_projection(
            "person",
            {"name": "highest_priority", "email": "most_specific"},
        )

        assert ack.entity_type == "person"
        assert ack.persisted is True

        request = httpx_mock.get_request()
        assert request is not None
        body = json.loads(request.content)
        assert body["entity_type"] == "person"
        assert body["field_policies"] == {
            "name": "highest_priority",
            "email": "most_specific",
        }

    def test_project_node(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/project",
            method="POST",
            json={
                "data": {
                    "entity_type": "person",
                    "fields": {"name": "Alice", "email": "alice@example.com"},
                    "observation_count": 3,
                }
            },
        )

        snapshot = client.project_node("person:alice")

        assert snapshot.entity_type == "person"
        assert snapshot.fields["name"] == "Alice"
        assert snapshot.observation_count == 3

    def test_node_field_provenance(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/fields/name/provenance",
            method="GET",
            json={
                "data": {
                    "field": "name",
                    "value": "Alice",
                    "source_event_id": "evt-42",
                    "source_event_at": "2026-01-01T00:00:00Z",
                    "merge_policy_applied": "last_write",
                }
            },
        )

        prov = client.node_field_provenance("person:alice", "name")

        assert prov is not None
        assert prov.field == "name"
        assert prov.value == "Alice"
        assert prov.source_event_id == "evt-42"
        assert prov.merge_policy_applied == "last_write"

    def test_node_field_provenance_404(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/fields/missing/provenance",
            method="GET",
            status_code=404,
            json={"error": {"code": "not_found", "message": "no provenance"}},
        )

        prov = client.node_field_provenance("person:alice", "missing")
        assert prov is None


class TestWebhooks:
    def test_create_webhook(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/webhooks",
            method="POST",
            json={
                "data": {
                    "id": "wh-1",
                    "url": "https://example.com/webhook",
                    "event_types": ["user.*"],
                    "active": True,
                    "created_at": "2026-01-01T00:00:00Z",
                }
            },
        )

        webhook = client.create_webhook(
            "https://example.com/webhook",
            event_types=["user.*"],
        )
        assert webhook.id == "wh-1"
        assert webhook.url == "https://example.com/webhook"

    def test_list_webhooks(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/webhooks",
            method="GET",
            json={"data": []},
        )

        webhooks = client.list_webhooks()
        assert webhooks == []

    def test_delete_webhook(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/webhooks/wh-1",
            method="DELETE",
            json={"ok": True},
        )

        client.delete_webhook("wh-1")


class TestErrorHandling:
    def test_api_error(self, client: AllSourceClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events",
            method="POST",
            status_code=422,
            json={
                "error": {
                    "code": "validation_error",
                    "message": "entity_id is required",
                }
            },
        )

        with pytest.raises(AllSourceError) as exc_info:
            client.ingest("user.signup", "", {})

        assert exc_info.value.code == "validation_error"
        assert exc_info.value.status_code == 422
        assert "entity_id is required" in str(exc_info.value)

    def test_auth_header(self, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/health",
            method="GET",
            json={"status": "ok"},
        )

        c = AllSourceClient(api_key="my-secret-key", base_url=BASE_URL)
        c.health()

        request = httpx_mock.get_request()
        assert request is not None
        assert request.headers["Authorization"] == "Bearer my-secret-key"
        c.close()

    def test_context_manager(self, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/health",
            method="GET",
            json={"status": "ok"},
        )

        with AllSourceClient(api_key="key", base_url=BASE_URL) as c:
            result = c.health()
            assert result["status"] == "ok"
