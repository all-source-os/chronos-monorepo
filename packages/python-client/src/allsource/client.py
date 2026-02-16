"""AllSource Python client."""

from __future__ import annotations

from typing import Any, Optional

import httpx

from allsource.types import (
    AllSourceError,
    Event,
    HealthResponse,
    IngestEventInput,
    QueryEventsParams,
    QueryEventsResponse,
)

DEFAULT_TIMEOUT = 30.0


class AllSourceClient:
    """Client for interacting with the AllSource event store.

    Args:
        base_url: Base URL of the AllSource Query Service.
        api_key: API key for authentication (sent as X-API-Key header).
        timeout: Request timeout in seconds. Defaults to 30.
    """

    def __init__(
        self,
        base_url: str,
        api_key: str,
        *,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> None:
        if not base_url:
            raise ValueError("base_url is required")
        if not api_key:
            raise ValueError("api_key is required")

        self._base_url = base_url.rstrip("/")
        self._client = httpx.Client(
            base_url=self._base_url,
            headers={
                "X-API-Key": api_key,
                "Accept": "application/json",
            },
            timeout=timeout,
        )

    def ingest_event(self, event: IngestEventInput) -> Event:
        """Ingest a single event into AllSource.

        Args:
            event: The event to ingest.

        Returns:
            The stored event with server-assigned id and timestamp.

        Raises:
            AllSourceError: If the API returns a non-2xx status.
        """
        data = self._request("POST", "/api/events", json=event.to_dict())
        return Event.from_dict(data)

    def query_events(
        self, params: Optional[QueryEventsParams] = None
    ) -> QueryEventsResponse:
        """Query events with optional filters.

        Args:
            params: Optional filters (entity_id, event_type, limit, etc.).

        Returns:
            Response containing matching events and count.

        Raises:
            AllSourceError: If the API returns a non-2xx status.
        """
        query = params.to_query_params() if params else {}
        data = self._request("GET", "/api/events", params=query)
        return QueryEventsResponse.from_dict(data)

    def get_health(self) -> HealthResponse:
        """Check the health of the AllSource service.

        Returns:
            Health status response.

        Raises:
            AllSourceError: If the API returns a non-2xx status.
        """
        data = self._request("GET", "/api/health")
        return HealthResponse.from_dict(data)

    def close(self) -> None:
        """Close the underlying HTTP client."""
        self._client.close()

    def __enter__(self) -> AllSourceClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    def _request(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: Optional[dict[str, str]] = None,
    ) -> Any:
        try:
            response = self._client.request(
                method, path, json=json, params=params
            )
        except httpx.TimeoutException as exc:
            raise AllSourceError(
                f"Request timeout: {exc}", status=0
            ) from exc

        if response.status_code >= 400:
            try:
                body = response.json()
            except Exception:
                body = response.text
            raise AllSourceError(
                f"AllSource API error: {response.status_code} {response.reason_phrase}",
                status=response.status_code,
                body=body,
            )

        return response.json()
