import structlog

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.event_processors.check_run import CheckRunProcessor
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = structlog.get_logger(__name__)


class CheckRunEventHandler(EventHandler):
    """Handler for check run webhook events using task queue."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type == EventType.CHECK_RUN

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """Handle check run events by enqueuing them for background processing."""
        logger.info(f"🔄 Enqueuing check run event for {event.repo_full_name}")

        task_id = await task_queue.enqueue(
            CheckRunProcessor().process,
            event_type="check_run",
            repo_full_name=event.repo_full_name,
            installation_id=event.installation_id,
            payload=event.payload,
        )

        logger.info(f"✅ Check run event enqueued with task ID: {task_id}")

        return WebhookResponse(
            status="ok",
            detail=f"Check run event has been queued for processing with task ID: {task_id}",
            event_type=EventType.CHECK_RUN,
        )
