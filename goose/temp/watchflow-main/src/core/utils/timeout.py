"""
Timeout utilities for async operations.

Provides functions for executing async operations with timeout handling.
"""

import asyncio
import logging
from collections.abc import Coroutine
from typing import Any

logger = logging.getLogger(__name__)


async def execute_with_timeout(
    coro: Coroutine[Any, Any, Any],
    timeout: float = 30.0,
    timeout_message: str | None = None,
) -> Any:
    """
    Execute a coroutine with timeout handling.

    Args:
        coro: The coroutine to execute
        timeout: Timeout in seconds
        timeout_message: Custom message for timeout exception

    Returns:
        The result of the coroutine

    Raises:
        TimeoutError: If the operation times out

    Example:
        result = await execute_with_timeout(
            long_running_operation(),
            timeout=60.0
        )
    """
    try:
        return await asyncio.wait_for(coro, timeout=timeout)
    except TimeoutError as err:
        msg = timeout_message or f"Operation timed out after {timeout} seconds"
        logger.error(f"❌ {msg}")
        raise TimeoutError(msg) from err


def timeout_decorator(timeout: float = 30.0, timeout_message: str | None = None) -> Any:
    """
    Decorator for adding timeout to async functions.

    Args:
        timeout: Timeout in seconds
        timeout_message: Custom message for timeout exception

    Returns:
        Decorated async function with timeout

    Example:
        @timeout_decorator(timeout=60.0)
        async def long_operation():
            await asyncio.sleep(100)  # Will timeout
    """

    def decorator(func: Any) -> Any:
        async def wrapper(*args: Any, **kwargs: Any) -> Any:
            return await execute_with_timeout(
                func(*args, **kwargs),
                timeout=timeout,
                timeout_message=timeout_message,
            )

        return wrapper

    return decorator
