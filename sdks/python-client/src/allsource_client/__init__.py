"""AllSource Event Store Python SDK."""

from allsource_client.client import AllSourceClient
from allsource_client.async_client import AllSourceAsyncClient
from allsource_client.types import (
    Event,
    EventList,
    Projection,
    PrimeProjection,
    PrimeProjectionAck,
    PrimeProvenance,
    PrimeSnapshot,
    Webhook,
    WebhookDelivery,
    AllSourceError,
)

__all__ = [
    "AllSourceClient",
    "AllSourceAsyncClient",
    "Event",
    "EventList",
    "Projection",
    "PrimeProjection",
    "PrimeProjectionAck",
    "PrimeProvenance",
    "PrimeSnapshot",
    "Webhook",
    "WebhookDelivery",
    "AllSourceError",
]

__version__ = "0.1.0"
