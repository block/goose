import structlog

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.event_processors.deployment_protection_rule import (
    DeploymentProtectionRuleProcessor,
)
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = structlog.get_logger(__name__)


class DeploymentProtectionRuleEventHandler(EventHandler):
    """Handler for deployment protection rule webhook events using task queue."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type == EventType.DEPLOYMENT_PROTECTION_RULE

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """Handle deployment protection rule events by enqueuing them for background processing."""
        logger.info(f"🔄 Enqueuing deployment protection rule event for {event.repo_full_name}")

        task_id = await task_queue.enqueue(
            DeploymentProtectionRuleProcessor().process,
            event_type="deployment_protection_rule",
            repo_full_name=event.repo_full_name,
            installation_id=event.installation_id,
            payload=event.payload,
        )

        logger.info(f"✅ Deployment protection rule event enqueued with task ID: {task_id}")

        return WebhookResponse(
            status="ok",
            detail=f"Deployment protection rule event for {event.repo_full_name} enqueued successfully",
            event_type=EventType.DEPLOYMENT_PROTECTION_RULE,
        )
