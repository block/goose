import structlog

from src.core.models import EventType, WebhookEvent, WebhookResponse
from src.event_processors.push import PushProcessor
from src.tasks.task_queue import task_queue
from src.webhooks.handlers.base import EventHandler

logger = structlog.get_logger()

# Instantiate processor once
push_processor = PushProcessor()


class PushEventHandler(EventHandler):
    """Thin handler for push webhook events—delegates to event processor."""

    async def can_handle(self, event: WebhookEvent) -> bool:
        return event.event_type.name == "PUSH"

    async def handle(self, event: WebhookEvent) -> WebhookResponse:
        """
        Orchestrates push event processing.
        Delegates to event_processors via TaskQueue.
        """
        log = logger.bind(
            event_type="push",
            repo=event.repo_full_name,
            ref=event.payload.get("ref"),
            commits_count=len(event.payload.get("commits", [])),
        )

        log.info("push_handler_invoked")

        try:
            # Build Task so process(task: Task) receives the correct type (not WebhookEvent)
            task = task_queue.build_task(
                "push",
                event.payload,
                push_processor.process,
                delivery_id=event.delivery_id,
            )
            enqueued = await task_queue.enqueue(
                push_processor.process,
                "push",
                event.payload,
                task,
                delivery_id=event.delivery_id,
            )

            if enqueued:
                log.info("push_event_enqueued")
                return WebhookResponse(
                    status="ok", detail="Push event enqueued for processing", event_type=EventType.PUSH
                )
            else:
                log.info("push_event_duplicate_skipped")
                return WebhookResponse(status="ignored", detail="Duplicate event skipped", event_type=EventType.PUSH)

        except ImportError:
            # Deployment processor may not exist yet
            log.warning("deployment_processor_not_found")
            return WebhookResponse(status="ok", detail="Push acknowledged (no processor)", event_type=EventType.PUSH)
        except Exception as e:
            log.error("push_processing_failed", error=str(e), exc_info=True)
            return WebhookResponse(
                status="error", detail=f"Push processing failed: {str(e)}", event_type=EventType.PUSH
            )
