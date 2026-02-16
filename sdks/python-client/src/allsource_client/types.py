"""Type definitions for AllSource SDK."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


class AllSourceError(Exception):
    """Error returned by the AllSource API."""

    def __init__(self, code: str, message: str, status_code: int = 0) -> None:
        super().__init__(message)
        self.code = code
        self.status_code = status_code


@dataclass
class Event:
    """An event in the AllSource event store."""

    id: str
    entity_id: str
    event_type: str
    payload: Dict[str, Any]
    timestamp: str
    version: int

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Event:
        return cls(
            id=data["id"],
            entity_id=data["entity_id"],
            event_type=data["event_type"],
            payload=data.get("payload", {}),
            timestamp=data["timestamp"],
            version=data.get("version", 0),
        )


@dataclass
class EventList:
    """A list of events with count."""

    events: List[Event]
    count: int

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> EventList:
        events_data = data.get("events", data.get("data", []))
        return cls(
            events=[Event.from_dict(e) for e in events_data],
            count=data.get("count", len(events_data)),
        )


@dataclass
class Projection:
    """A projection in the AllSource event store."""

    id: str
    name: str
    version: int
    status: str
    initial_state: Dict[str, Any]
    definition: str
    created_at: str
    updated_at: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Projection:
        return cls(
            id=data["id"],
            name=data["name"],
            version=data.get("version", 0),
            status=data.get("status", "unknown"),
            initial_state=data.get("initial_state", {}),
            definition=data.get("definition", ""),
            created_at=data.get("created_at", ""),
            updated_at=data.get("updated_at", ""),
        )


@dataclass
class Webhook:
    """A registered webhook."""

    id: str
    url: str
    event_types: List[str] = field(default_factory=list)
    entity_id: Optional[str] = None
    active: bool = True
    created_at: str = ""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Webhook:
        return cls(
            id=data["id"],
            url=data["url"],
            event_types=data.get("event_types", []),
            entity_id=data.get("entity_id"),
            active=data.get("active", True),
            created_at=data.get("created_at", ""),
        )


@dataclass
class WebhookDelivery:
    """A webhook delivery record."""

    id: str
    webhook_id: str
    event_id: str
    status: str
    response_code: Optional[int] = None
    attempts: int = 0
    last_attempt_at: str = ""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> WebhookDelivery:
        return cls(
            id=data["id"],
            webhook_id=data["webhook_id"],
            event_id=data.get("event_id", ""),
            status=data.get("status", "unknown"),
            response_code=data.get("response_code"),
            attempts=data.get("attempts", 0),
            last_attempt_at=data.get("last_attempt_at", ""),
        )
