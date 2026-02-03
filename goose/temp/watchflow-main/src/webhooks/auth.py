import hashlib
import hmac
from collections.abc import Mapping

import structlog
from fastapi import HTTPException, Request

from src.core.config import config

logger = structlog.get_logger(__name__)

GITHUB_WEBHOOK_SECRET = config.github.webhook_secret

# Headers that should never be logged (security-sensitive)
_SENSITIVE_HEADERS = frozenset({"authorization", "cookie", "x-hub-signature-256", "x-hub-signature"})


def _redact_headers(headers: Mapping[str, str]) -> dict[str, str]:
    """Redact sensitive headers for safe logging."""
    return {k: "[REDACTED]" if k.lower() in _SENSITIVE_HEADERS else v for k, v in headers.items()}


async def verify_github_signature(request: Request) -> bool:
    """
    FastAPI dependency that verifies the GitHub webhook signature.

    This function reads the 'X-Hub-Signature-256' header and compares it
    with a hash of the raw request body, using our configured webhook secret.

    Raises:
        HTTPException: If the signature is missing or invalid.

    Returns:
        True if the signature is valid.
    """
    signature = request.headers.get("X-Hub-Signature-256")

    # Log headers with sensitive values redacted
    logger.debug("request_headers_received", headers=_redact_headers(request.headers))

    if not signature:
        logger.warning("Received a request without the X-Hub-Signature-256 header.")
        raise HTTPException(status_code=401, detail="Missing GitHub webhook signature.")

    # Raw bytes—GitHub signs body, not parsed JSON.
    payload = await request.body()

    # HMAC-SHA256—GitHub standard. Brittle if GitHub changes algo.
    mac = hmac.new(GITHUB_WEBHOOK_SECRET.encode(), msg=payload, digestmod=hashlib.sha256)
    expected_signature = f"sha256={mac.hexdigest()}"

    # Constant-time compare—prevents timing attacks.
    if not hmac.compare_digest(signature, expected_signature):
        logger.error("Invalid webhook signature.")
        raise HTTPException(status_code=401, detail="Invalid GitHub webhook signature.")

    logger.debug("GitHub webhook signature verified successfully.")
    return True
