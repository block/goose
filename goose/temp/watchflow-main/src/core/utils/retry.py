"""
Retry utilities with exponential backoff.

Provides decorators and functions for retrying async operations with
configurable exponential backoff strategies.
"""

import asyncio
import logging
from collections.abc import Awaitable, Callable
from functools import wraps
from typing import Any, TypeVar

logger = logging.getLogger(__name__)

T = TypeVar("T")


def retry_with_backoff(
    max_retries: int = 3,
    initial_delay: float = 1.0,
    max_delay: float = 60.0,
    exponential_base: float = 2.0,
    exceptions: tuple[type[Exception], ...] = (Exception,),
) -> Any:
    """
    Decorator for retrying async functions with exponential backoff.

    Args:
        max_retries: Maximum number of retry attempts
        initial_delay: Initial delay in seconds before first retry
        max_delay: Maximum delay in seconds between retries
        exponential_base: Base for exponential backoff calculation
        exceptions: Tuple of exception types to catch and retry on

    Returns:
        Decorated async function with retry logic

    Example:
        @retry_with_backoff(max_retries=3, initial_delay=1.0)
        async def fetch_data():
            return await api_call()
    """

    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        @wraps(func)
        async def wrapper(*args: Any, **kwargs: Any) -> Any:
            delay = initial_delay
            last_exception: Exception | None = None

            for attempt in range(max_retries):
                try:
                    result = await func(*args, **kwargs)
                    if attempt > 0:
                        logger.info(f"✅ {func.__name__} succeeded on attempt {attempt + 1}/{max_retries}")
                    return result
                except exceptions as e:
                    last_exception = e
                    if attempt == max_retries - 1:
                        logger.error(f"❌ {func.__name__} failed after {max_retries} attempts: {e}")
                        raise

                    wait_time = min(delay, max_delay)
                    logger.warning(
                        f"⚠️ {func.__name__} attempt {attempt + 1}/{max_retries} failed, "
                        f"retrying in {wait_time:.2f}s: {e}"
                    )
                    await asyncio.sleep(wait_time)
                    delay *= exponential_base

            # This should never be reached, but type checker needs it
            if last_exception:
                raise last_exception
            raise RuntimeError(f"{func.__name__} failed after {max_retries} attempts")

        return wrapper

    return decorator


async def retry_async[T](
    func: Callable[..., Awaitable[T]],
    *args: Any,
    max_retries: int = 3,
    initial_delay: float = 1.0,
    max_delay: float = 60.0,
    exponential_base: float = 2.0,
    exceptions: tuple[type[Exception], ...] = (Exception,),
    **kwargs: Any,
) -> T:
    """
    Retry an async function call with exponential backoff.

    Args:
        func: Async function to retry
        *args: Positional arguments for the function
        max_retries: Maximum number of retry attempts
        initial_delay: Initial delay in seconds before first retry
        max_delay: Maximum delay in seconds between retries
        exponential_base: Base for exponential backoff calculation
        exceptions: Tuple of exception types to catch and retry on
        **kwargs: Keyword arguments for the function

    Returns:
        Result of the function call

    Raises:
        Exception: If all retries fail

    Example:
        result = await retry_async(fetch_data, max_retries=3)
    """
    delay = initial_delay
    last_exception: Exception | None = None

    for attempt in range(max_retries):
        try:
            return await func(*args, **kwargs)
        except exceptions as e:
            last_exception = e
            if attempt == max_retries - 1:
                logger.error(f"❌ {func.__name__} failed after {max_retries} attempts: {e}")
                raise

            wait_time = min(delay, max_delay)
            logger.warning(
                f"⚠️ {func.__name__} attempt {attempt + 1}/{max_retries} failed, retrying in {wait_time:.2f}s: {e}"
            )
            await asyncio.sleep(wait_time)
            delay *= exponential_base

    # This should never be reached, but type checker needs it
    if last_exception:
        raise last_exception
    raise RuntimeError(f"{func.__name__} failed after {max_retries} attempts")
