"""Structured error response models for consistent API error handling."""

from typing import Any

from pydantic import BaseModel


class ErrorResponse(BaseModel):
    """Standardized error response schema."""

    error: bool = True
    code: str
    message: str
    retry_after: int | None = None
    details: dict[str, Any] | None = None


def create_error_response(
    code: str, message: str, retry_after: int | None = None, details: dict[str, Any] | None = None
) -> ErrorResponse:
    """Create standardized error response."""
    return ErrorResponse(code=code, message=message, retry_after=retry_after, details=details)
