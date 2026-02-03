"""
Module for monitoring async Pydantic AI API calls.
"""

from openlit.instrumentation.pydantic_ai.utils import (
    common_agent_run_async,
)


def async_agent_run(
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Generates a telemetry wrapper for async GenAI function call
    """

    async def wrapper(wrapped, instance, args, kwargs):
        return await common_agent_run_async(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
            pricing_info=pricing_info,
        )

    return wrapper
