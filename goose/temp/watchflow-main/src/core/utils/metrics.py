"""
Performance metrics utilities.

Provides utilities for tracking and recording performance metrics
for operations, API calls, and agent executions.
"""

import logging
import time
from contextlib import asynccontextmanager
from functools import wraps
from typing import Any

logger = logging.getLogger(__name__)


@asynccontextmanager
async def track_metrics(
    operation: str,
    **metadata: Any,
) -> Any:  # AsyncGenerator[dict[str, Any], None] but Any is simpler for mypy in context managers sometimes
    """
    Context manager for tracking operation metrics.

    Records timing and metadata for performance analysis.

    Args:
        operation: Name of the operation
        **metadata: Additional metadata to record

    Yields:
        Dictionary with metrics that can be updated during execution

    Example:
        async with track_metrics("rule_evaluation", rule_count=5) as metrics:
            result = await evaluate_rules(...)
            metrics["violations_found"] = len(result.violations)
    """
    start_time = time.time()
    metrics: dict[str, Any] = {
        "operation": operation,
        "start_time": start_time,
        **metadata,
    }

    try:
        yield metrics
    finally:
        end_time = time.time()
        latency_ms = int((end_time - start_time) * 1000)
        metrics.update(
            {
                "end_time": end_time,
                "latency_ms": latency_ms,
                "success": "error" not in metrics,
            }
        )

        # Log metrics
        logger.info(
            f"📊 Metrics for {operation}: {latency_ms}ms",
            extra=metrics,
        )


def metrics_decorator(operation: str | None = None, **default_metadata: Any) -> Any:
    """
    Decorator for tracking function call metrics.

    Args:
        operation: Custom operation name (defaults to function name)
        **default_metadata: Default metadata to include in metrics

    Returns:
        Decorated function with metrics tracking

    Example:
        @metrics_decorator(operation="api_call", endpoint="/rules")
        async def fetch_rules():
            return await api_call()
    """

    def decorator(func: Any) -> Any:
        op_name = operation or func.__name__

        @wraps(func)
        async def async_wrapper(*args: Any, **kwargs: Any) -> Any:
            async with track_metrics(op_name, **default_metadata) as metrics:
                try:
                    result = await func(*args, **kwargs)
                    metrics["success"] = True
                    return result
                except Exception as e:
                    metrics["error"] = str(e)
                    metrics["success"] = False
                    raise

        @wraps(func)
        def sync_wrapper(*args: Any, **kwargs: Any) -> Any:
            start_time = time.time()
            metrics: dict[str, Any] = {
                "operation": op_name,
                "start_time": start_time,
                **default_metadata,
            }

            try:
                result = func(*args, **kwargs)
                end_time = time.time()
                metrics.update(
                    {
                        "end_time": end_time,
                        "latency_ms": int((end_time - start_time) * 1000),
                        "success": True,
                    }
                )
                logger.info(
                    f"📊 Metrics for {op_name}: {metrics['latency_ms']}ms",
                    extra=metrics,
                )
                return result
            except Exception as e:
                end_time = time.time()
                metrics.update(
                    {
                        "end_time": end_time,
                        "latency_ms": int((end_time - start_time) * 1000),
                        "error": str(e),
                        "success": False,
                    }
                )
                logger.error(
                    f"📊 Metrics for {op_name}: {metrics['latency_ms']}ms (failed)",
                    extra=metrics,
                )
                raise

        # Return appropriate wrapper based on whether function is async
        import inspect

        if inspect.iscoroutinefunction(func):
            return async_wrapper
        return sync_wrapper

    return decorator
