"""Tests for the async AllSource client."""

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
