"""Type definitions for the AllSource Python SDK."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class IngestEventInput:
    """An event to ingest into AllSource."""

    event_type: str
    """The type of event (e.g., 'user.signup', 'order.placed')."""

    entity_id: str
    """The entity this event belongs to (e.g., user ID, order ID)."""

    payload: dict[str, Any]
    """Arbitrary JSON payload for the event."""

    metadata: Optional[dict[str, Any]] = None
    """Optional metadata (e.g., source, version, ip)."""

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {
            "event_type": self.event_type,
            "entity_id": self.entity_id,
            "payload": self.payload,
        }
        if self.metadata is not None:
            d["metadata"] = self.metadata
        return d


@dataclass
class Event:
    """A stored event returned from AllSource."""

    id: str
    event_type: str
    entity_id: str
    payload: dict[str, Any]
    metadata: dict[str, Any]
    timestamp: str
    stream_id: Optional[str] = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Event:
        return cls(
            id=data["id"],
            event_type=data["event_type"],
            entity_id=data["entity_id"],
            payload=data.get("payload", {}),
            metadata=data.get("metadata", {}),
            timestamp=data["timestamp"],
            stream_id=data.get("stream_id"),
        )


@dataclass
class QueryEventsParams:
    """Query parameters for filtering events."""

    entity_id: Optional[str] = None
    event_type: Optional[str] = None
    limit: Optional[int] = None
    offset: Optional[int] = None
    start_time: Optional[str] = None
    end_time: Optional[str] = None

    def to_query_params(self) -> dict[str, str]:
        params: dict[str, str] = {}
        if self.entity_id is not None:
            params["entity_id"] = self.entity_id
        if self.event_type is not None:
            params["event_type"] = self.event_type
        if self.limit is not None:
            params["limit"] = str(self.limit)
        if self.offset is not None:
            params["offset"] = str(self.offset)
        if self.start_time is not None:
            params["start_time"] = self.start_time
        if self.end_time is not None:
            params["end_time"] = self.end_time
        return params


@dataclass
class QueryEventsResponse:
    """Response from querying events."""

    events: list[Event]
    count: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> QueryEventsResponse:
        return cls(
            events=[Event.from_dict(e) for e in data.get("events", [])],
            count=data.get("count", 0),
        )


@dataclass
class HealthResponse:
    """Response from the health endpoint."""

    status: str
    extra: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> HealthResponse:
        status = data.pop("status", "unknown")
        return cls(status=status, extra=data)


class AllSourceError(Exception):
    """Error raised by the AllSource client."""

    def __init__(
        self,
        message: str,
        status: int,
        body: Any = None,
    ) -> None:
        super().__init__(message)
        self.status = status
        self.body = body
