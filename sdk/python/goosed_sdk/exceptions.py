"""
goosed-sdk - Python SDK for goosed API

Exception classes for handling API errors.
"""


class GoosedException(Exception):
    """Base exception for goosed SDK errors."""

    def __init__(self, message: str = "An error occurred", status_code: int | None = None):
        super().__init__(message)
        self.message = message
        self.status_code = status_code


class GoosedAuthError(GoosedException):
    """Authentication failed (401)."""

    def __init__(self, message: str = "Authentication failed"):
        super().__init__(message, status_code=401)


class GoosedNotFoundError(GoosedException):
    """Resource not found (404)."""

    def __init__(self, message: str = "Resource not found"):
        super().__init__(message, status_code=404)


class GoosedAgentNotInitializedError(GoosedException):
    """Agent not initialized (424)."""

    def __init__(self, message: str = "Agent not initialized"):
        super().__init__(message, status_code=424)


class GoosedServerError(GoosedException):
    """Server error (500+)."""

    def __init__(self, message: str = "Server error"):
        super().__init__(message, status_code=500)


class GoosedConnectionError(GoosedException):
    """Connection error (network issues, timeouts)."""

    def __init__(self, message: str = "Connection error"):
        super().__init__(message, status_code=None)
