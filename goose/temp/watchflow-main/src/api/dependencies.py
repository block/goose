import logging

from fastapi import Depends, HTTPException, Request, status

from src.core.models import User
from src.integrations.github.service import GitHubService

logger = logging.getLogger(__name__)  # Logger: keep at module level for reuse.

# --- Service Dependencies ---  # DI: swap for mock in tests.


def get_github_service() -> GitHubService:
    """
    Injects GitHubService—future: allow mock for integration tests.
    """
    return GitHubService()


# --- Auth Dependencies ---  # Auth: allow anonymous for public repo support.


async def get_current_user_optional(request: Request) -> User | None:
    """
    Auth check—don't fail if missing. Critical for public repo support (Phase 1).
    """
    auth_header = request.headers.get("Authorization")

    if not auth_header:
        return None

    try:
        # Token extraction—fragile if header format changes.
        scheme, token = auth_header.split()
        if scheme.lower() != "bearer":
            return None

        # Open-source version: Pass token through without validation (users provide their own GitHub tokens).
        # No external dependencies - token validation would require IdP integration.
        from pydantic import SecretStr

        from src.core.config import config

        if config.use_mock_data:
            return User(id=123, username="authenticated_user", email="user@example.com", github_token=SecretStr(token))

        # In real usage, we would validate the token here or pass it to endpoints to use against GitHub API
        # For now, we return a User object wrapping the token so it can be used by services
        # We use a dummy ID for the anonymous/token-holder user logic
        logger.debug("Creating user wrapper for provided token")
        return User(id=0, username="token_user", email="token@user.com", github_token=SecretStr(token))
    except Exception as e:
        logger.warning(f"Failed to parse auth header: {e}")
        return None


async def get_current_user(user: User | None = Depends(get_current_user_optional)) -> User:
    """
    Strict dependency for endpoints that MUST have a user (e.g., 'Analyze Private Repo').
    """
    if not user:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED, detail="Authentication required for this operation."
        )
    return user
