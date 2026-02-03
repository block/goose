# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument, too-many-branches
"""
Module for monitoring async Letta calls following OpenTelemetry patterns.
"""

import logging
import asyncio
import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception
from .utils import (
    get_span_name,
    process_letta_response,
    OPERATION_TYPE_MAP,
)

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


def create_async_letta_wrapper(
    gen_ai_endpoint,
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
    Creates a unified telemetry wrapper for async Letta operations following OpenTelemetry patterns.

    This follows the same pattern as sync LiteLLM and CrewAI instrumentations but for async operations.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async API call to add telemetry following OpenTelemetry semantic conventions.
        """

        async def async_wrapper(*args, **kwargs):
            # Extract operation type from endpoint
            endpoint_parts = gen_ai_endpoint.split(".")
            operation = endpoint_parts[-1] if endpoint_parts else "unknown"
            operation_type = OPERATION_TYPE_MAP.get(operation, "workflow")

            # Generate proper span name with context
            span_name = get_span_name(operation_type, gen_ai_endpoint, instance, kwargs)
            start_time = time.time()

            # Check if this is a streaming operation
            streaming = operation_type == "chat"  # Assume chat operations might stream

            if streaming:
                # Execute the operation first to check if it's actually streaming
                if asyncio.iscoroutinefunction(wrapped):
                    response = await wrapped(*args, **kwargs)
                else:
                    response = wrapped(*args, **kwargs)

                # Check if response is actually a stream
                if hasattr(response, "__aiter__") and hasattr(response, "__anext__"):
                    # For async streaming: create span without context manager (like LiteLLM)
                    span = tracer.start_span(span_name, kind=SpanKind.CLIENT)
                    # Note: For async streams, we'd need an async version of TracedLettaStream
                    # For now, we'll treat them as non-streaming
                    streaming = False

            # Non-streaming or non-chat operations
            with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
                try:
                    if not streaming:
                        # Execute async operation
                        if asyncio.iscoroutinefunction(wrapped):
                            response = await wrapped(*args, **kwargs)
                        else:
                            response = wrapped(*args, **kwargs)

                    # Process response using common helpers
                    process_letta_response(
                        span,
                        response,
                        kwargs,
                        operation_type,
                        instance,
                        start_time,
                        environment,
                        application_name,
                        version,
                        gen_ai_endpoint,
                        capture_message_content,
                        pricing_info,
                    )

                    return response

                except Exception as e:
                    handle_exception(span, e)
                    logger.error("Error in async Letta trace creation: %s", e)
                    # Re-execute without instrumentation on error
                    if asyncio.iscoroutinefunction(wrapped):
                        return await wrapped(*args, **kwargs)
                    else:
                        return wrapped(*args, **kwargs)

        return async_wrapper(*args, **kwargs)

    return wrapper


# Async agent operations
def async_create_agent(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async agent operations wrapper (create, retrieve, modify, delete, list)"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )


# Async message operations
def async_send_message(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async message operations wrapper (create, create_stream, list, modify, etc.)"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )


# Async memory operations
def async_memory_operation(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async memory operations wrapper (core memory, blocks)"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )


# Async tool operations
def async_tool_operation(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async tool operations wrapper (list, attach, detach, create)"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )


# Async context operations
def async_context_operation(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async context operations wrapper"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )


# Async source operations
def async_source_operation(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Async source operations wrapper"""
    return create_async_letta_wrapper(
        gen_ai_endpoint,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    )
