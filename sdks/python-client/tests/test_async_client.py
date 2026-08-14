"""Tests for the async AllSource client."""

import json

import pytest
from pytest_httpx import HTTPXMock

from allsource_client import AllSourceAsyncClient, AllSourceError


BASE_URL = "http://localhost:3902"


@pytest.fixture
def client() -> AllSourceAsyncClient:
    return AllSourceAsyncClient(api_key="test-key", base_url=BASE_URL)


class TestAsyncIngest:
    async def test_ingest_event(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
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

        event = await client.ingest("user.signup", "user-123", {"plan": "pro"})

        assert event.id == "evt-1"
        assert event.entity_id == "user-123"
        assert event.event_type == "user.signup"
        await client.close()

    async def test_ingest_batch(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events/batch",
            method="POST",
            json={
                "data": [
                    {
                        "id": "evt-1",
                        "entity_id": "u-1",
                        "event_type": "user.signup",
                        "payload": {},
                        "timestamp": "2026-01-01T00:00:00Z",
                        "version": 1,
                    }
                ]
            },
        )

        events = await client.ingest_batch([
            {"event_type": "user.signup", "entity_id": "u-1", "payload": {}}
        ])
        assert len(events) == 1
        await client.close()


class TestAsyncQuery:
    async def test_query_events(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events",
            method="GET",
            json={
                "data": {"data": [], "count": 0}
            },
        )

        result = await client.query()
        assert result.count == 0
        await client.close()


class TestAsyncProjections:
    async def test_list_projections(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/projections",
            method="GET",
            json={"data": []},
        )

        projections = await client.get_projections()
        assert projections == []
        await client.close()

    async def test_get_projection(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/projections/my-proj",
            method="GET",
            json={
                "data": {
                    "id": "p-1",
                    "name": "my-proj",
                    "version": 1,
                    "status": "running",
                    "initial_state": {},
                    "definition": "count(*)",
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                }
            },
        )

        p = await client.get_projection("my-proj")
        assert p.name == "my-proj"
        await client.close()


class TestAsyncProjectionReplay:
    async def test_analyze_and_run_projection_replay(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        analysis = {
            "projection_name": "event-count",
            "projection_title": "Event Count",
            "projection_kind": "counter",
            "projection_status": "ready",
            "current_entity_count": 1,
            "total_events": 1,
            "sampled_events": 1,
            "analysis_scope": "full",
            "event_type_distribution": [
                {"event_type": "order.created", "count": 1, "share": 100.0}
            ],
            "sampled_entity_count": 1,
            "sampled_entities": [{"entity_id": "order-1", "event_count": 1}],
            "first_event_at": "2026-08-14T10:00:00Z",
            "last_event_at": "2026-08-14T10:00:00Z",
            "analyzed_at": "2026-08-14T10:01:00Z",
            "ready_to_replay": True,
            "checks": [],
            "warnings": [],
        }
        run = {
            "replay_id": "replay-1",
            "projection_name": "event-count",
            "status": "running",
            "started_at": "2026-08-14T10:01:00Z",
            "updated_at": "2026-08-14T10:01:00Z",
            "completed_at": None,
            "total_events": 1,
            "processed_events": 0,
            "failed_events": 0,
            "progress_percentage": 0.0,
            "events_per_second": 0.0,
            "error_message": None,
        }
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay/preview",
            method="POST",
            json={"data": analysis},
        )
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/replay", method="POST", json={"data": run}
        )

        preview = await client.analyze_projection_replay("event-count")
        started = await client.start_projection_replay("event-count")

        assert preview.ready_to_replay is True
        assert preview.total_events == 1
        assert started.replay_id == "replay-1"
        await client.close()


class TestAsyncPrime:
    async def test_list_prime_projections(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/projections",
            method="GET",
            json={
                "data": [
                    {
                        "entity_type": "person",
                        "field_policies": {"name": "last_write"},
                    }
                ],
                "count": 1,
            },
        )

        projections = await client.list_prime_projections()
        assert len(projections) == 1
        assert projections[0].entity_type == "person"
        assert projections[0].field_policies["name"] == "last_write"
        await client.close()

    async def test_define_prime_projection(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/projections",
            method="POST",
            status_code=201,
            json={"data": {"entity_type": "person", "persisted": True}},
        )

        ack = await client.define_prime_projection(
            "person", {"name": "merge_array"}
        )
        assert ack.entity_type == "person"
        assert ack.persisted is True

        request = httpx_mock.get_request()
        assert request is not None
        body = json.loads(request.content)
        assert body["entity_type"] == "person"
        assert body["field_policies"] == {"name": "merge_array"}
        await client.close()

    async def test_project_node(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/project",
            method="POST",
            json={
                "data": {
                    "entity_type": "person",
                    "fields": {"name": "Alice"},
                    "observation_count": 2,
                }
            },
        )

        snapshot = await client.project_node("person:alice")
        assert snapshot.entity_type == "person"
        assert snapshot.fields["name"] == "Alice"
        assert snapshot.observation_count == 2
        await client.close()

    async def test_node_field_provenance(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/fields/name/provenance",
            method="GET",
            json={
                "data": {
                    "field": "name",
                    "value": "Alice",
                    "source_event_id": "evt-7",
                    "source_event_at": "2026-01-01T00:00:00Z",
                    "merge_policy_applied": "last_write",
                }
            },
        )

        prov = await client.node_field_provenance("person:alice", "name")
        assert prov is not None
        assert prov.field == "name"
        assert prov.source_event_id == "evt-7"
        await client.close()

    async def test_node_field_provenance_404(
        self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock
    ) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/v1/prime/nodes/person:alice/fields/missing/provenance",
            method="GET",
            status_code=404,
            json={"error": {"code": "not_found", "message": "no provenance"}},
        )

        prov = await client.node_field_provenance("person:alice", "missing")
        assert prov is None
        await client.close()


class TestAsyncErrorHandling:
    async def test_api_error(self, client: AllSourceAsyncClient, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/api/events",
            method="POST",
            status_code=401,
            json={"error": {"code": "unauthorized", "message": "Invalid API key"}},
        )

        with pytest.raises(AllSourceError) as exc_info:
            await client.ingest("test", "test", {})

        assert exc_info.value.code == "unauthorized"
        assert exc_info.value.status_code == 401
        await client.close()

    async def test_async_context_manager(self, httpx_mock: HTTPXMock) -> None:
        httpx_mock.add_response(
            url=f"{BASE_URL}/health",
            method="GET",
            json={"status": "ok"},
        )

        async with AllSourceAsyncClient(api_key="key", base_url=BASE_URL) as c:
            result = await c.health()
            assert result["status"] == "ok"
