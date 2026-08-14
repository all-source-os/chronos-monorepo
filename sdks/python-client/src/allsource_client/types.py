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
class ProjectionReplayEventType:
    """One event-type bucket from replay impact analysis."""

    event_type: str
    count: int
    share: float

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ProjectionReplayEventType:
        return cls(
            event_type=data.get("event_type", "unknown"),
            count=data.get("count", 0),
            share=data.get("share", 0.0),
        )


@dataclass
class ProjectionReplayEntity:
    """One frequently affected entity from replay analysis."""

    entity_id: str
    event_count: int

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ProjectionReplayEntity:
        return cls(
            entity_id=data.get("entity_id", ""),
            event_count=data.get("event_count", 0),
        )


@dataclass
class ProjectionReplayCheck:
    """Server-asserted replay safety invariant."""

    key: str
    label: str
    status: str
    detail: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ProjectionReplayCheck:
        return cls(
            key=data.get("key", ""),
            label=data.get("label", ""),
            status=data.get("status", "warn"),
            detail=data.get("detail", ""),
        )


@dataclass
class ProjectionReplayAnalysis:
    """Read-only impact analysis for one tenant projection replay."""

    projection_name: str
    projection_title: str
    projection_kind: str
    projection_status: str
    current_entity_count: int
    total_events: int
    sampled_events: int
    analysis_scope: str
    event_type_distribution: List[ProjectionReplayEventType]
    sampled_entity_count: int
    sampled_entities: List[ProjectionReplayEntity]
    first_event_at: Optional[str]
    last_event_at: Optional[str]
    analyzed_at: str
    ready_to_replay: bool
    checks: List[ProjectionReplayCheck]
    warnings: List[str]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ProjectionReplayAnalysis:
        return cls(
            projection_name=data.get("projection_name", ""),
            projection_title=data.get("projection_title", ""),
            projection_kind=data.get("projection_kind", ""),
            projection_status=data.get("projection_status", "unknown"),
            current_entity_count=data.get("current_entity_count", 0),
            total_events=data.get("total_events", 0),
            sampled_events=data.get("sampled_events", 0),
            analysis_scope=data.get("analysis_scope", "full"),
            event_type_distribution=[
                ProjectionReplayEventType.from_dict(item)
                for item in data.get("event_type_distribution", [])
            ],
            sampled_entity_count=data.get("sampled_entity_count", 0),
            sampled_entities=[
                ProjectionReplayEntity.from_dict(item)
                for item in data.get("sampled_entities", [])
            ],
            first_event_at=data.get("first_event_at"),
            last_event_at=data.get("last_event_at"),
            analyzed_at=data.get("analyzed_at", ""),
            ready_to_replay=data.get("ready_to_replay", False),
            checks=[
                ProjectionReplayCheck.from_dict(item)
                for item in data.get("checks", [])
            ],
            warnings=data.get("warnings", []),
        )


@dataclass
class ProjectionReplayRun:
    """Tenant-scoped projection replay run."""

    replay_id: str
    projection_name: str
    status: str
    started_at: str
    updated_at: str
    completed_at: Optional[str]
    total_events: int
    processed_events: int
    failed_events: int
    progress_percentage: float
    events_per_second: float
    error_message: Optional[str]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ProjectionReplayRun:
        return cls(
            replay_id=data.get("replay_id", ""),
            projection_name=data.get("projection_name", ""),
            status=data.get("status", "unknown"),
            started_at=data.get("started_at", ""),
            updated_at=data.get("updated_at", ""),
            completed_at=data.get("completed_at"),
            total_events=data.get("total_events", 0),
            processed_events=data.get("processed_events", 0),
            failed_events=data.get("failed_events", 0),
            progress_percentage=data.get("progress_percentage", 0.0),
            events_per_second=data.get("events_per_second", 0.0),
            error_message=data.get("error_message"),
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
class PrimeProjection:
    """A Prime entity-type projection definition."""

    entity_type: str
    field_policies: Dict[str, str]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> PrimeProjection:
        return cls(
            entity_type=data.get("entity_type", ""),
            field_policies=data.get("field_policies", {}),
        )


@dataclass
class PrimeProjectionAck:
    """Acknowledgement of a defined Prime projection."""

    entity_type: str
    persisted: bool

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> PrimeProjectionAck:
        return cls(
            entity_type=data.get("entity_type", ""),
            persisted=data.get("persisted", False),
        )


@dataclass
class PrimeSnapshot:
    """A projected snapshot of a Prime node."""

    entity_type: str
    fields: Dict[str, Any]
    observation_count: int

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> PrimeSnapshot:
        return cls(
            entity_type=data.get("entity_type", ""),
            fields=data.get("fields", {}),
            observation_count=data.get("observation_count", 0),
        )


@dataclass
class PrimeProvenance:
    """Provenance for a single projected field of a Prime node."""

    field: str
    value: Any
    source_event_id: str
    source_event_at: str
    merge_policy_applied: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> PrimeProvenance:
        return cls(
            field=data.get("field", ""),
            value=data.get("value"),
            source_event_id=data.get("source_event_id", ""),
            source_event_at=data.get("source_event_at", ""),
            merge_policy_applied=data.get("merge_policy_applied", ""),
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
