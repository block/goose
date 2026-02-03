from abc import ABC, abstractmethod

from src.core.models import WebhookEvent, WebhookResponse


class EventHandler(ABC):
    """
    Abstract base class for all webhook event handlers.

    Each implementation must return a WebhookResponse for standardized results.
    Handlers should be thin orchestrators that delegate to event_processors.
    """

    @abstractmethod
    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """
        Process the incoming webhook event.

        Args:
            event: The validated and parsed WebhookEvent object.

        Returns:
            A WebhookResponse containing the results of the handling logic.
        """
        pass
