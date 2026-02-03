"""Authentication-related API endpoints."""

import structlog
from fastapi import APIRouter, HTTPException, status
from pydantic import BaseModel

from src.integrations.github.api import github_client

logger = structlog.get_logger()

# Use prefix to keep URLs clean: /auth/validate-token
router = APIRouter(prefix="/auth", tags=["Authentication"])


class ValidateTokenRequest(BaseModel):
    """Request model for token validation."""

    token: str


class ValidateTokenResponse(BaseModel):
    """Response model for token validation."""

    valid: bool
    has_repo_scope: bool
    has_public_repo_scope: bool
    scopes: list[str] | None = None
    message: str | None = None


@router.post(
    "/validate-token",
    response_model=ValidateTokenResponse,
    status_code=status.HTTP_200_OK,
    summary="Validate GitHub Token",
    description="Check if a GitHub Personal Access Token has the required scopes (repo or public_repo).",
)
async def validate_token(request: ValidateTokenRequest) -> ValidateTokenResponse:
    """Validate GitHub PAT and check for repo/public_repo scopes."""
    token = request.token.strip()

    if not token:
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Token is required")

    try:
        url = "https://api.github.com/user"
        headers = {
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github.v3+json",
        }

        # Use the shared session from the global client
        session = await github_client._get_session()
        async with session.get(url, headers=headers) as response:
            if response.status == 401:
                return ValidateTokenResponse(
                    valid=False,
                    has_repo_scope=False,
                    has_public_repo_scope=False,
                    message="Token is invalid or expired.",
                )

            if response.status != 200:
                error_text = await response.text()
                logger.error("token_validation_failed", status=response.status, error=error_text)
                raise HTTPException(
                    status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                    detail="Token validation failed. Please try again.",
                )

            # Get scopes from X-OAuth-Scopes header
            oauth_scopes_header = response.headers.get("X-OAuth-Scopes", "")
            scopes = [s.strip() for s in oauth_scopes_header.split(",")] if oauth_scopes_header else []

            has_repo_scope = "repo" in scopes
            has_public_repo_scope = "public_repo" in scopes
            has_required_scope = has_repo_scope or has_public_repo_scope

            return ValidateTokenResponse(
                valid=True,
                has_repo_scope=has_repo_scope,
                has_public_repo_scope=has_public_repo_scope,
                scopes=scopes,
                message=(
                    "Token is valid and has required scopes."
                    if has_required_scope
                    else "Token is valid but missing required scopes (repo or public_repo)."
                ),
            )

    except HTTPException:
        raise
    except Exception as e:
        logger.exception("token_validation_error", error=str(e))
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Token validation failed. Please try again.",
        ) from e
