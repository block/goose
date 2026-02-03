import structlog

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = structlog.get_logger(__name__)


class DeploymentEventHandler(EventHandler):
    """Handler for GitHub deployment events."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type == EventType.DEPLOYMENT

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """Handle deployment events."""
        payload = event.payload
        repo_full_name = event.repo_full_name
        installation_id = payload.get("installation", {}).get("id")

        if not installation_id:
            logger.error(f"No installation ID found in deployment event for {repo_full_name}")
            return WebhookResponse(status="error", detail="Missing installation ID")

        # Extract deployment—fragile if GitHub changes payload structure.
        deployment = payload.get("deployment", {})
        environment = deployment.get("environment", "unknown")

        logger.info(f"🔄 Enqueuing deployment event for {repo_full_name}")
        logger.info(f"   Environment: {environment}")
        logger.info(f"   Ref: {deployment.get('ref', 'unknown')}")

        from src.event_processors.deployment import DeploymentProcessor

        # ... existing code ...
        # Enqueue: async, may fail if queue overloaded.
        task_id = await task_queue.enqueue(
            DeploymentProcessor().process,
            event_type="deployment",
            repo_full_name=repo_full_name,
            installation_id=installation_id,
            payload=payload,
        )

        logger.info(f"✅ Deployment event enqueued with task ID: {task_id}")

        return WebhookResponse(
            status="ok",
            detail=f"Deployment event for {repo_full_name} enqueued successfully",
            event_type=EventType.DEPLOYMENT,
        )

        return {
            "status": "success",
            "message": f"Deployment event for {repo_full_name} enqueued successfully",
            "task_id": task_id,
        }
