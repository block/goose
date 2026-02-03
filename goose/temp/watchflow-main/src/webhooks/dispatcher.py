from collections.abc import Callable, Coroutine
from typing import Any

import structlog

from src.core.models import EventType, WebhookEvent
from src.tasks.task_queue import TaskQueue, task_queue

logger = structlog.get_logger()


class WebhookDispatcher:
    """
    Orchestrates Event -> Handler -> TaskQueue handover.
    No business logic—pure routing to background processing.
    """

    def __init__(self, queue: TaskQueue | None = None) -> None:
        # Map event types to their specific business logic handlers
        self.handlers: dict[str, Callable[..., Coroutine[Any, Any, Any]]] = {}
        # Use provided queue or default to singleton
        self.queue = queue or task_queue

    def register_handler(self, event_type: str, handler: Callable[..., Coroutine[Any, Any, Any]]) -> None:
        """Registers a handler for a specific GitHub event."""
        self.handlers[event_type] = handler
        logger.debug("handler_registered", event_type=event_type)

    async def dispatch(self, event: WebhookEvent) -> dict[str, Any]:
        """
        Routes the event to the appropriate handler via TaskQueue.
        Returns status indicating if the event was dispatched.
        """
        # Extract event type as string for routing
        event_type = event.event_type.value if isinstance(event.event_type, EventType) else str(event.event_type)

        log = logger.bind(event_type=event_type)

        handler = self.handlers.get(event_type)
        if not handler:
            log.warning("handler_not_found")
            return {"status": "skipped", "reason": f"No handler for event type {event_type}"}

        # Offload to TaskQueue for background execution (delivery_id so each webhook delivery is processed)
        success = await self.queue.enqueue(handler, event_type, event.payload, event, delivery_id=event.delivery_id)

        if success:
            log.info("event_dispatched_to_queue")
            return {"status": "queued", "event_type": event_type}
        else:
            log.info("event_duplicate_skipped")
            return {"status": "duplicate", "event_type": event_type}


dispatcher = WebhookDispatcher()
