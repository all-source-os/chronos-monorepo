"""Tests for the AllSource Python client."""

import json

import httpx
import pytest
import respx

from allsource import (
    AllSourceClient,
    AllSourceError,
    Event,
    HealthResponse,
    IngestEventInput,
    QueryEventsParams,
    QueryEventsResponse,
)

BASE_URL = "https://allsource-query.fly.dev"
API_KEY = "test-api-key-123"


@pytest.fixture
def client():
    c = AllSourceClient(base_url=BASE_URL, api_key=API_KEY)
    yield c
    c.close()


class TestConstructor:
    def test_requires_base_url(self):
        with pytest.raises(ValueError, match="base_url is required"):
            AllSourceClient(base_url="", api_key="key")

    def test_requires_api_key(self):
        with pytest.raises(ValueError, match="api_key is required"):
            AllSourceClient(base_url="http://localhost", api_key="")

    def test_strips_trailing_slash(self):
        c = AllSourceClient(base_url="http://localhost:3902/", api_key="key")
        assert c._base_url == "http://localhost:3902"
        c.close()


class TestContextManager:
    def test_context_manager(self):
        with AllSourceClient(base_url=BASE_URL, api_key=API_KEY) as c:
            assert c._base_url == BASE_URL


class TestGetHealth:
    @respx.mock
    def test_get_health(self, client):
        respx.get(f"{BASE_URL}/api/health").mock(
            return_value=httpx.Response(
                200, json={"status": "healthy", "version": "0.10.3"}
            )
        )
        health = client.get_health()
        assert isinstance(health, HealthResponse)
        assert health.status == "healthy"
        assert health.extra["version"] == "0.10.3"


class TestIngestEvent:
    @respx.mock
    def test_ingest_event(self, client):
        event_input = IngestEventInput(
            event_type="user.signup",
            entity_id="user-123",
            payload={"email": "test@example.com"},
            metadata={"source": "sdk-test"},
        )
        respx.post(f"{BASE_URL}/api/events").mock(
            return_value=httpx.Response(
                200,
                json={
                    "id": "evt-abc",
                    "event_type": "user.signup",
                    "entity_id": "user-123",
                    "payload": {"email": "test@example.com"},
                    "metadata": {"source": "sdk-test"},
                    "timestamp": "2026-02-16T00:00:00Z",
                },
            )
        )
        event = client.ingest_event(event_input)
        assert isinstance(event, Event)
        assert event.id == "evt-abc"
        assert event.event_type == "user.signup"
        assert event.entity_id == "user-123"

    @respx.mock
    def test_ingest_sends_api_key_header(self, client):
        route = respx.post(f"{BASE_URL}/api/events").mock(
            return_value=httpx.Response(
                200,
                json={
                    "id": "evt-1",
                    "event_type": "test",
                    "entity_id": "e-1",
                    "payload": {},
                    "metadata": {},
                    "timestamp": "2026-02-16T00:00:00Z",
                },
            )
        )
        client.ingest_event(
            IngestEventInput(event_type="test", entity_id="e-1", payload={})
        )
        assert route.calls[0].request.headers["x-api-key"] == API_KEY

    @respx.mock
    def test_ingest_without_metadata(self, client):
        route = respx.post(f"{BASE_URL}/api/events").mock(
            return_value=httpx.Response(
                200,
                json={
                    "id": "evt-2",
                    "event_type": "test",
                    "entity_id": "e-1",
                    "payload": {"key": "val"},
                    "metadata": {},
                    "timestamp": "2026-02-16T00:00:00Z",
                },
            )
        )
        event_input = IngestEventInput(
            event_type="test", entity_id="e-1", payload={"key": "val"}
        )
        client.ingest_event(event_input)
        body = json.loads(route.calls[0].request.content)
        assert "metadata" not in body


class TestQueryEvents:
    @respx.mock
    def test_query_events_no_params(self, client):
        respx.get(f"{BASE_URL}/api/events").mock(
            return_value=httpx.Response(
                200,
                json={
                    "events": [
                        {
                            "id": "evt-1",
                            "event_type": "user.signup",
                            "entity_id": "user-1",
                            "payload": {},
                            "metadata": {},
                            "timestamp": "2026-02-16T00:00:00Z",
                        }
                    ],
                    "count": 1,
                },
            )
        )
        result = client.query_events()
        assert isinstance(result, QueryEventsResponse)
        assert result.count == 1
        assert len(result.events) == 1
        assert result.events[0].event_type == "user.signup"

    @respx.mock
    def test_query_events_with_filters(self, client):
        route = respx.get(f"{BASE_URL}/api/events").mock(
            return_value=httpx.Response(
                200, json={"events": [], "count": 0}
            )
        )
        params = QueryEventsParams(
            entity_id="user-123",
            event_type="user.signup",
            limit=10,
            offset=5,
        )
        client.query_events(params)
        url = route.calls[0].request.url
        assert url.params["entity_id"] == "user-123"
        assert url.params["event_type"] == "user.signup"
        assert url.params["limit"] == "10"
        assert url.params["offset"] == "5"


class TestErrorHandling:
    @respx.mock
    def test_api_error_json_body(self, client):
        respx.get(f"{BASE_URL}/api/health").mock(
            return_value=httpx.Response(
                401, json={"error": "unauthorized"}
            )
        )
        with pytest.raises(AllSourceError) as exc_info:
            client.get_health()
        assert exc_info.value.status == 401
        assert exc_info.value.body == {"error": "unauthorized"}

    @respx.mock
    def test_api_error_text_body(self, client):
        respx.get(f"{BASE_URL}/api/health").mock(
            return_value=httpx.Response(500, text="Internal Server Error")
        )
        with pytest.raises(AllSourceError) as exc_info:
            client.get_health()
        assert exc_info.value.status == 500
        assert exc_info.value.body == "Internal Server Error"

    @respx.mock
    def test_timeout_error(self):
        c = AllSourceClient(
            base_url=BASE_URL, api_key=API_KEY, timeout=0.001
        )
        respx.get(f"{BASE_URL}/api/health").mock(
            side_effect=httpx.ReadTimeout("timeout")
        )
        with pytest.raises(AllSourceError, match="timeout"):
            c.get_health()
        c.close()
