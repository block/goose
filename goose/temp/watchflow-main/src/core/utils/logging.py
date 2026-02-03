"""
Structured logging utilities.

Provides context managers and decorators for structured operation logging
with timing, error tracking, and metadata.
"""

from __future__ import annotations

import inspect
import logging
import time
from collections.abc import Callable  # noqa: TCH003
from contextlib import asynccontextmanager
from functools import wraps
from typing import Any

logger = logging.getLogger(__name__)


@asynccontextmanager
async def log_operation(
    operation: str,
    subject_ids: dict[str, str] | None = None,
    **context: Any,
) -> Any:  # AsyncGenerator[None, None]
    """
    Context manager for structured operation logging.

    Logs operation start, completion, and errors with timing information.

    Args:
        operation: Name of the operation being performed
        subject_ids: Dictionary of subject identifiers (e.g., {"repo": "owner/repo", "pr": "123"})
        **context: Additional context to include in logs

    Example:
        async with log_operation("rule_evaluation", repo=repo, pr=pr_number):
            result = await evaluate_rules(...)
    """
    start_time = time.time()
    log_context = {
        "operation": operation,
        **(subject_ids or {}),
        **context,
    }

    logger.info(f"🚀 Starting {operation}", extra=log_context)

    try:
        yield
    except Exception as e:
        latency_ms = int((time.time() - start_time) * 1000)
        logger.error(
            f"❌ {operation} failed after {latency_ms}ms",
            extra={**log_context, "error": str(e), "latency_ms": latency_ms},
            exc_info=True,
        )
        raise
    else:
        latency_ms = int((time.time() - start_time) * 1000)
        logger.info(
            f"✅ {operation} completed in {latency_ms}ms",
            extra={**log_context, "latency_ms": latency_ms},
        )


def log_function_call(operation: str | None = None) -> Any:
    """
    Decorator for logging function calls with timing.

    Args:
        operation: Custom operation name (defaults to function name)

    Returns:
        Decorated function with logging

    Example:
        @log_function_call(operation="fetch_data")
        async def fetch_data():
            return await api_call()
    """

    def decorator(func: Callable[..., Any]) -> Callable[..., Any]:
        op_name = operation or func.__name__

        @wraps(func)
        async def async_wrapper(*args: Any, **kwargs: Any) -> Any:
            start_time = time.time()
            logger.info(f"🚀 Calling {op_name}")

            try:
                result = await func(*args, **kwargs)
                latency_ms = int((time.time() - start_time) * 1000)
                logger.info(f"✅ {op_name} completed in {latency_ms}ms")
                return result
            except Exception as e:
                latency_ms = int((time.time() - start_time) * 1000)
                logger.error(
                    f"❌ {op_name} failed after {latency_ms}ms: {e}",
                    exc_info=True,
                )
                raise

        @wraps(func)
        def sync_wrapper(*args: Any, **kwargs: Any) -> Any:
            start_time = time.time()
            logger.info(f"🚀 Calling {op_name}")

            try:
                result = func(*args, **kwargs)
                latency_ms = int((time.time() - start_time) * 1000)
                logger.info(f"✅ {op_name} completed in {latency_ms}ms")
                return result
            except Exception as e:
                latency_ms = int((time.time() - start_time) * 1000)
                logger.error(
                    f"❌ {op_name} failed after {latency_ms}ms: {e}",
                    exc_info=True,
                )
                raise

        # Return appropriate wrapper based on whether function is async
        if inspect.iscoroutinefunction(func):
            return async_wrapper
        return sync_wrapper

    return decorator


def log_structured(
    logger_obj: logging.Logger,
    event: str,
    level: str = "info",
    **context: Any,
) -> None:
    """
    Lightweight structured logging helper.

    Args:
        logger_obj: Logger instance to use.
        event: Event/operation name.
        level: Logging level (info|warning|error).
        **context: Arbitrary key/value metadata.
    """
    # Callable is now only needed for typing, so it's safe to use the string name or handled by __future__
    log_fn: Callable[..., Any] = getattr(logger_obj, level, logger_obj.info)
    log_fn(event, extra=context)
