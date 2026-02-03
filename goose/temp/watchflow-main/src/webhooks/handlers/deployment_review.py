import logging

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.event_processors.deployment_review import DeploymentReviewProcessor
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = logging.getLogger(__name__)


class DeploymentReviewEventHandler(EventHandler):
    """Handler for deployment review webhook events using task queue."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type == EventType.DEPLOYMENT_REVIEW

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """Handle deployment review events by enqueuing them for background processing."""
        logger.info(f"🔄 Enqueuing deployment review event for {event.repo_full_name}")

        task_id = await task_queue.enqueue(
            DeploymentReviewProcessor().process,
            event_type="deployment_review",
            repo_full_name=event.repo_full_name,
            installation_id=event.installation_id,
            payload=event.payload,
        )

        logger.info(f"✅ Deployment review event enqueued with task ID: {task_id}")

        return WebhookResponse(
            status="ok",
            detail=f"Deployment review event for {event.repo_full_name} enqueued successfully",
            event_type=EventType.DEPLOYMENT_REVIEW,
        )
