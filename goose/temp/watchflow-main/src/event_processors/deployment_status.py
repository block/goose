import logging
import time
from typing import Any

from src.event_processors.base import BaseEventProcessor, ProcessingResult
from src.tasks.task_queue import Task

logger = logging.getLogger(__name__)


class DeploymentStatusProcessor(BaseEventProcessor):
    """Processor for deployment_status events - for logging and monitoring only."""

    def __init__(self) -> None:
        # Call super class __init__ first
        super().__init__()

    def get_event_type(self) -> str:
        return "deployment_status"

    async def process(self, task: Task) -> ProcessingResult:
        start_time = time.time()
        payload = task.payload
        deployment_status = payload.get("deployment_status", {})
        deployment = payload.get("deployment", {})

        state = deployment_status.get("state", "")
        environment = deployment.get("environment", "")
        creator = deployment.get("creator", {}).get("login", "")

        logger.info("=" * 80)
        logger.info(f"📊 Processing DEPLOYMENT_STATUS event for {task.repo_full_name}")
        logger.info(f"   State: {state}")
        logger.info(f"   Environment: {environment}")
        logger.info(f"   Creator: {creator}")
        logger.info("=" * 80)

        # Log different states for monitoring purposes
        if state == "error":
            logger.info(f"💥 Deployment to {environment} had an error")
        elif state == "waiting":
            logger.info(f"⏳ Deployment to {environment} is waiting for protection rule review")
        elif state == "success":
            logger.info(f"✅ Deployment to {environment} was successful")
        elif state == "failure":
            logger.info(f"❌ Deployment to {environment} failed")
        else:
            logger.info(f"📋 Deployment to {environment} has state: {state}")

        logger.info("=" * 80)
        logger.info(f"🏁 DEPLOYMENT_STATUS processing completed in {int((time.time() - start_time) * 1000)}ms")
        logger.info("=" * 80)

        return ProcessingResult(
            success=True, violations=[], api_calls_made=0, processing_time_ms=int((time.time() - start_time) * 1000)
        )

    async def prepare_webhook_data(self, task: Task) -> dict[str, Any]:
        """Extract data available in webhook payload."""
        return task.payload

    async def prepare_api_data(self, task: Task) -> dict[str, Any]:
        """Fetch data not available in webhook."""
        return {}
