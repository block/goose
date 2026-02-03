from typing import Any

from fastapi import APIRouter, BackgroundTasks

from src.tasks.scheduler.deployment_scheduler import get_deployment_scheduler

router = APIRouter()


@router.get("/status")
async def get_scheduler_status() -> dict[str, Any]:
    """Get scheduler status and pending deployments."""
    return get_deployment_scheduler().get_status()


@router.post("/check-deployments")
async def check_pending_deployments(background_tasks: BackgroundTasks) -> dict[str, str]:
    """Manually re-evaluate the status of pending deployments."""
    background_tasks.add_task(get_deployment_scheduler()._check_pending_deployments)
    return {"status": "scheduled", "message": "Deployment statuses will be updated on GitHub accordingly."}


@router.get("/pending-deployments")
async def get_pending_deployments() -> dict[str, Any]:
    """Get list of pending deployments."""
    status = get_deployment_scheduler().get_status()
    return {"pending_count": status["pending_count"], "deployments": status["pending_deployments"]}
