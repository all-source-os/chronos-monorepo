"""Synchronous AllSource Event Store client."""

from __future__ import annotations

from typing import Any, Dict, List, Optional

import httpx

from allsource_client.types import (
    AllSourceError,
    Event,
    EventList,
    Projection,
    Webhook,
    WebhookDelivery,
)


class AllSourceClient:
    """Synchronous client for the AllSource Event Store API.

    Usage::

        client = AllSourceClient(api_key="as_xxx", base_url="https://api.all-source.xyz")
        event = client.ingest("user.signup", "user-123", {"plan": "pro"})
        events = client.query(event_type="user.signup")
    """

    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.all-source.xyz",
        timeout: float = 30.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._http = httpx.Client(
            base_url=self._base_url,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            timeout=timeout,
        )

    def close(self) -> None:
        """Close the underlying HTTP connection."""
        self._http.close()

    def __enter__(self) -> AllSourceClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()

    def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        response = self._http.request(method, path, **kwargs)
        if response.status_code >= 400:
            try:
                body = response.json()
                error = body.get("error", {})
                raise AllSourceError(
                    code=error.get("code", "api_error"),
                    message=error.get("message", response.text),
                    status_code=response.status_code,
                )
            except (ValueError, KeyError):
                raise AllSourceError(
                    code="api_error",
                    message=response.text,
                    status_code=response.status_code,
                )
        return response.json()

    # --- Events ---

    def ingest(
        self,
        event_type: str,
        entity_id: str,
        data: Dict[str, Any],
    ) -> Event:
        """Ingest a single event into the store."""
        body = {
            "event_type": event_type,
            "entity_id": entity_id,
            "payload": data,
        }
        result = self._request("POST", "/api/events", json=body)
        return Event.from_dict(result.get("data", result))

    def ingest_batch(
        self,
        events: List[Dict[str, Any]],
    ) -> List[Event]:
        """Ingest a batch of events.

        Each dict should have keys: event_type, entity_id, payload.
        """
        result = self._request("POST", "/api/events/batch", json={"events": events})
        items = result.get("data", result)
        if isinstance(items, list):
            return [Event.from_dict(e) for e in items]
        return []

    def query(
        self,
        *,
        event_type: Optional[str] = None,
        entity_id: Optional[str] = None,
        start: Optional[str] = None,
        end: Optional[str] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> EventList:
        """Query events with optional filters."""
        params: Dict[str, Any] = {}
        if event_type is not None:
            params["event_type"] = event_type
        if entity_id is not None:
            params["entity_id"] = entity_id
        if start is not None:
            params["start"] = start
        if end is not None:
            params["end"] = end
        if limit is not None:
            params["limit"] = limit
        if offset is not None:
            params["offset"] = offset
        result = self._request("GET", "/api/events", params=params)
        data = result.get("data", result)
        if isinstance(data, dict):
            return EventList.from_dict(data)
        return EventList.from_dict(result)

    def get_events_by_entity(self, entity_id: str) -> EventList:
        """Get all events for a specific entity."""
        result = self._request("GET", f"/api/events/entity/{entity_id}")
        data = result.get("data", result)
        if isinstance(data, dict):
            return EventList.from_dict(data)
        return EventList.from_dict(result)

    def get_events_by_type(self, event_type: str) -> EventList:
        """Get all events of a specific type."""
        result = self._request("GET", f"/api/events/type/{event_type}")
        data = result.get("data", result)
        if isinstance(data, dict):
            return EventList.from_dict(data)
        return EventList.from_dict(result)

    # --- Projections ---

    def get_projections(self) -> List[Projection]:
        """List all projections."""
        result = self._request("GET", "/api/projections")
        data = result.get("data", result)
        if isinstance(data, list):
            return [Projection.from_dict(p) for p in data]
        return []

    def get_projection(self, name: str) -> Projection:
        """Get a projection by name."""
        result = self._request("GET", f"/api/projections/{name}")
        data = result.get("data", result)
        return Projection.from_dict(data)

    def create_projection(
        self,
        name: str,
        definition: str,
        *,
        version: int = 1,
        initial_state: Optional[Dict[str, Any]] = None,
    ) -> Projection:
        """Create a new projection."""
        body: Dict[str, Any] = {
            "name": name,
            "definition": definition,
            "version": version,
            "initial_state": initial_state or {},
        }
        result = self._request("POST", "/api/projections", json=body)
        data = result.get("data", result)
        return Projection.from_dict(data)

    # --- Webhooks ---

    def create_webhook(
        self,
        url: str,
        *,
        event_types: Optional[List[str]] = None,
        entity_id: Optional[str] = None,
    ) -> Webhook:
        """Register a webhook endpoint."""
        body: Dict[str, Any] = {"url": url}
        if event_types is not None:
            body["event_types"] = event_types
        if entity_id is not None:
            body["entity_id"] = entity_id
        result = self._request("POST", "/api/webhooks", json=body)
        data = result.get("data", result)
        return Webhook.from_dict(data)

    def list_webhooks(self) -> List[Webhook]:
        """List all registered webhooks."""
        result = self._request("GET", "/api/webhooks")
        data = result.get("data", result)
        if isinstance(data, list):
            return [Webhook.from_dict(w) for w in data]
        return []

    def delete_webhook(self, webhook_id: str) -> None:
        """Delete a webhook."""
        self._request("DELETE", f"/api/webhooks/{webhook_id}")

    def get_webhook_deliveries(self, webhook_id: str) -> List[WebhookDelivery]:
        """Get delivery history for a webhook."""
        result = self._request("GET", f"/api/webhooks/{webhook_id}/deliveries")
        data = result.get("data", result)
        if isinstance(data, list):
            return [WebhookDelivery.from_dict(d) for d in data]
        return []

    # --- Health ---

    def health(self) -> Dict[str, Any]:
        """Check API health."""
        return self._request("GET", "/health")  # type: ignore[no-any-return]
