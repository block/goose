"""Repository-related API endpoints."""

from typing import Any

import structlog
from fastapi import APIRouter, HTTPException, status

from src.core.config import config
from src.integrations.github.api import github_client

logger = structlog.get_logger()

router = APIRouter(prefix="/repos", tags=["Repositories"])


@router.get(
    "/{owner}/{repo}/installation",
    response_model=dict[str, Any],
    status_code=status.HTTP_200_OK,
    summary="Check GitHub App Installation",
    description="Check if the Watchflow GitHub App is installed for a given repository.",
)
async def check_installation(owner: str, repo: str) -> dict[str, Any]:
    """Check if Watchflow GitHub App is installed for repository."""
    repo_full_name = f"{owner}/{repo}"

    try:
        # We need to find the installation for this repo
        # Requires app JWT authentication to query installations

        jwt_token = github_client._generate_jwt()
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Accept": "application/vnd.github.v3+json",
        }
        url = f"{config.github.api_base_url}/repos/{owner}/{repo}/installation"

        session = await github_client._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 200:
                installation = await response.json()
                return {
                    "installed": True,
                    "installation_id": installation.get("id"),
                    "permissions": installation.get("permissions", {}),
                    "message": "Installation found",
                }
            elif response.status == 404:
                return {"installed": False, "message": "App not installed on this repository"}
            else:
                error_text = await response.text()
                logger.error("installation_check_failed", repo=repo_full_name, status=response.status, error=error_text)
                raise HTTPException(
                    status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                    detail=f"Failed to check installation: {error_text}",
                )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception("installation_check_failed", repo=repo_full_name, error=str(e))
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to check installation: {str(e)}",
        ) from e
