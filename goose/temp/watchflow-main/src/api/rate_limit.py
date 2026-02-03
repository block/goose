"""
Rate limiting dependency for FastAPI endpoints.
Limits requests per IP (anonymous) or user (authenticated).

Open-source version: In-memory rate limiting (resets on restart, no external dependencies).
"""

import time

from fastapi import Depends, HTTPException, Request, status

from src.api.dependencies import get_current_user_optional
from src.core.config import config
from src.core.models import User

# In-memory store: { key: [timestamps] }
# Note: This resets on server restart (no external dependencies for persistence).
_RATE_LIMIT_STORE: dict[str, list[float]] = {}

ANON_LIMIT = config.anonymous_rate_limit  # Default: 5 requests per hour
AUTH_LIMIT = config.authenticated_rate_limit  # Default: 100 requests per hour
WINDOW = 3600  # seconds


async def rate_limiter(request: Request, user: User | None = Depends(get_current_user_optional)) -> None:
    now = time.time()
    if user and user.email:
        key = f"user:{user.email}"
        limit = AUTH_LIMIT
    else:
        client_host = request.client.host if request.client else "unknown"
        key = f"ip:{client_host}"
        limit = ANON_LIMIT

    timestamps = _RATE_LIMIT_STORE.get(key, [])
    # Remove timestamps outside the window
    timestamps = [ts for ts in timestamps if now - ts < WINDOW]
    if len(timestamps) >= limit:
        retry_after = int(WINDOW - (now - min(timestamps)))
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail=f"Rate limit exceeded. Try again in {retry_after} seconds.",
            headers={"Retry-After": str(retry_after)},
        )
    timestamps.append(now)
    _RATE_LIMIT_STORE[key] = timestamps
