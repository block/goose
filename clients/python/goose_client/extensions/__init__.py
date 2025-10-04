"""Extension system for advanced features"""

from typing import TYPE_CHECKING

from .base import Extension
from .retry import RetryExtension
from .session_manager import SessionManagerExtension

if TYPE_CHECKING:
    from ..client import GooseClient


def with_retry(client: "GooseClient", **kwargs) -> "GooseClient":
    """Add retry capability to client.

    Args:
        client: The GooseClient instance to extend
        **kwargs: Additional arguments for RetryExtension
            - max_retries: Maximum number of retry attempts (default: 3)
            - backoff_factor: Exponential backoff multiplier (default: 2.0)
            - max_wait: Maximum wait time between retries (default: 30.0)
            - retry_on: Tuple of exception types to retry on (default: (Exception,))

    Returns:
        The same client instance with retry capability added

    Example:
        >>> client = with_retry(GooseClient(api_key="key"), max_retries=5)
    """
    ext = RetryExtension(client, **kwargs)
    ext.install()
    return client


def with_session_manager(client: "GooseClient", **kwargs) -> "GooseClient":
    """Add session management to client.

    Args:
        client: The GooseClient instance to extend
        **kwargs: Additional arguments for SessionManagerExtension
            - max_saved_sessions: Maximum number of sessions to keep (default: 10)

    Returns:
        The same client instance with session management added

    Example:
        >>> client = with_session_manager(GooseClient(api_key="key"))
        >>> client.session_manager.save_session(session_id, "my_session")
    """
    ext = SessionManagerExtension(client, **kwargs)
    ext.install()
    return client


# Note: Extension classes are already imported at the top of the file

__all__ = [
    "with_retry",
    "with_session_manager",
    "Extension",
    "RetryExtension",
    "SessionManagerExtension",
]
