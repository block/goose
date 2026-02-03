import structlog
from fastapi import APIRouter, Depends, HTTPException, Request

from src.core.models import EventType, WebhookEvent
from src.webhooks.auth import verify_github_signature
from src.webhooks.dispatcher import WebhookDispatcher, dispatcher
from src.webhooks.models import GitHubEventModel, WebhookResponse

logger = structlog.get_logger()
router = APIRouter()


# DI for dispatcher—testability, lifecycle control.
def get_dispatcher() -> WebhookDispatcher:
    """Returns the shared WebhookDispatcher instance."""
    return dispatcher


def _create_event_from_request(
    event_name: str | None,
    payload: dict,
    delivery_id: str | None = None,
) -> WebhookEvent:
    """Factory function to create a WebhookEvent from raw request data."""
    if not event_name:
        raise HTTPException(status_code=400, detail="Missing X-GitHub-Event header")

    # GitHub sometimes sends event names with dot suffixes—strip for enum match.
    normalized_event_name = event_name.split(".")[0]
    logger.info("webhook_event_received", github_event=event_name, normalized=normalized_event_name)

    try:
        # Enum mapping—fail if GitHub adds new event type we don't support.
        event_type = EventType(normalized_event_name)
    except ValueError as e:
        logger.warning("unsupported_event_type", github_event=event_name, error=str(e))
        # Defensive: Accept unknown events, but don't process—avoids GitHub retries/spam.
        raise HTTPException(status_code=202, detail=f"Event type '{event_name}' is received but not supported.") from e

    return WebhookEvent(event_type=event_type, payload=payload, delivery_id=delivery_id)


@router.post("/github", summary="Endpoint for all GitHub webhooks")
async def github_webhook_endpoint(
    request: Request,
    is_verified: bool = Depends(verify_github_signature),
    dispatcher_instance: WebhookDispatcher = Depends(get_dispatcher),
) -> WebhookResponse:
    """
    This endpoint receives all events from a configured GitHub App.

    - It first verifies the request signature to ensure it's from GitHub.
    - It then creates a domain event object from the request payload.
    - Finally, it passes the event to a dispatcher to be routed to the
      correct application service.
    """
    # Signature check handled by dependency—fail fast if invalid.
    import json
    from typing import Any, cast

    try:
        payload = cast("dict[str, Any]", await request.json())
    except json.JSONDecodeError as e:
        logger.error("webhook_json_parse_failed", error=str(e))
        raise HTTPException(status_code=400, detail="Invalid JSON payload") from e

    event_name = request.headers.get("X-GitHub-Event")
    delivery_id = request.headers.get("X-GitHub-Delivery")

    # Parse and validate incoming event payload
    try:
        github_event = GitHubEventModel(**payload)
        logger.info(
            "webhook_validated",
            event_type=event_name,
            repository=github_event.repository.full_name,
            sender=github_event.sender.login,
        )
    except Exception as e:
        logger.error("webhook_validation_failed", event_type=event_name, error=str(e))
        raise HTTPException(status_code=400, detail="Invalid webhook payload structure") from e

    try:
        event = _create_event_from_request(event_name, payload, delivery_id=delivery_id)
        await dispatcher_instance.dispatch(event)
        return WebhookResponse(
            status="ok",
            detail="Event dispatched successfully",
            event_type=event.event_type.value,
        )
    except HTTPException as e:
        # Don't 500 on unknown event—keeps GitHub happy, avoids alert noise.
        if e.status_code == 202:
            normalized = event_name.split(".")[0] if event_name else None
            return WebhookResponse(
                status="ignored",
                detail=e.detail,
                event_type=normalized,
            )
        raise e
