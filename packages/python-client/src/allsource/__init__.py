"""AllSource Python SDK for event store interaction."""

from allsource.client import AllSourceClient
from allsource.types import (
    AllSourceError,
    Event,
    HealthResponse,
    IngestEventInput,
    QueryEventsParams,
    QueryEventsResponse,
)

__all__ = [
    "AllSourceClient",
    "AllSourceError",
    "Event",
    "HealthResponse",
    "IngestEventInput",
    "QueryEventsParams",
    "QueryEventsResponse",
]

__version__ = "0.1.0"
