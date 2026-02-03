from functools import lru_cache

import structlog

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.event_processors.pull_request.processor import PullRequestProcessor
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = structlog.get_logger()


# Instantiate processor once (singleton-like) but lazily
@lru_cache(maxsize=1)
def get_pr_processor() -> PullRequestProcessor:
    return PullRequestProcessor()


class PullRequestEventHandler(EventHandler):
    """Thin handler for pull request webhook events—delegates to event processor."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type.name == "PULL_REQUEST"

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """
        Orchestrates pull request event processing.
        Delegates to event_processors via TaskQueue.
        """
        log = logger.bind(
            event_type="pull_request",
            repo=event.repo_full_name,
            pr_number=event.payload.get("pull_request", {}).get("number"),
            action=event.payload.get("action"),
        )

        # Filter relevant actions to reduce noise (optional but good practice)
        action = event.payload.get("action")
        if action not in ["opened", "synchronize", "reopened", "edited"]:
            log.info("pr_action_ignored", action=action)
            return WebhookResponse(
                status="ignored", detail=f"PR action '{action}' is not processed", event_type=EventType.PULL_REQUEST
            )

        log.info("pr_handler_invoked")

        try:
            # Build Task so process(task: Task) receives the correct type (not WebhookEvent)
            processor = get_pr_processor()
            task = task_queue.build_task(
                "pull_request",
                event.payload,
                processor.process,
                delivery_id=event.delivery_id,
            )
            enqueued = await task_queue.enqueue(
                processor.process,
                "pull_request",
                event.payload,
                task,
                delivery_id=event.delivery_id,
            )

            if enqueued:
                log.info("pr_event_enqueued")
                return WebhookResponse(
                    status="ok", detail="Pull request event enqueued for processing", event_type=EventType.PULL_REQUEST
                )
            else:
                log.info("pr_event_duplicate_skipped")
                return WebhookResponse(
                    status="ignored", detail="Duplicate event skipped", event_type=EventType.PULL_REQUEST
                )

        except Exception as e:
            log.error("pr_processing_failed", error=str(e), exc_info=True)
            return WebhookResponse(
                status="error", detail=f"PR processing failed: {str(e)}", event_type=EventType.PULL_REQUEST
            )
