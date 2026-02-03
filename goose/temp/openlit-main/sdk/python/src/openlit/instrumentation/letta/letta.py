# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument, too-many-branches
"""
Module for monitoring Letta calls following OpenTelemetry patterns.
"""

import logging
import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception
from .utils import (
    get_span_name,
    process_letta_response,
    TracedLettaStream,
    OPERATION_TYPE_MAP,
)

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


def create_letta_wrapper(
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
    Creates a unified telemetry wrapper for Letta operations following OpenTelemetry patterns.

    This follows the same pattern as LiteLLM and CrewAI instrumentations.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the API call to add telemetry following OpenTelemetry semantic conventions.
        """

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
            response = wrapped(*args, **kwargs)

            # Check if response is actually a stream
            if hasattr(response, "__iter__") and hasattr(response, "__next__"):
                # For streaming: create span without context manager (like LiteLLM)
                span = tracer.start_span(span_name, kind=SpanKind.CLIENT)
                return TracedLettaStream(
                    wrapped_stream=response,
                    span=span,
                    span_name=span_name,
                    kwargs=kwargs,
                    operation_type=operation_type,
                    instance=instance,
                    start_time=start_time,
                    environment=environment,
                    application_name=application_name,
                    version=version,
                    endpoint=gen_ai_endpoint,
                    capture_content=capture_message_content,
                    pricing_info=pricing_info,
                    tracer=tracer,
                )

        # Non-streaming or non-chat operations
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            try:
                if not streaming:
                    # Execute the operation for non-streaming
                    response = wrapped(*args, **kwargs)

                # Process non-streaming response using common helpers
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
                logger.error("Error in Letta trace creation: %s", e)
                # Re-execute without instrumentation on error
                return wrapped(*args, **kwargs)

    return wrapper


# Agent operations
def create_agent(
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
    """Agent operations wrapper (create, retrieve, modify, delete, list)"""
    return create_letta_wrapper(
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


# Message operations
def send_message(
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
    """Message operations wrapper (create, create_stream, list, modify, etc.)"""
    return create_letta_wrapper(
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


# Memory operations
def memory_operation(
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
    """Memory operations wrapper (core memory, blocks)"""
    return create_letta_wrapper(
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


# Tool operations
def tool_operation(
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
    """Tool operations wrapper (list, attach, detach, create)"""
    return create_letta_wrapper(
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


# Context operations
def context_operation(
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
    """Context operations wrapper"""
    return create_letta_wrapper(
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


# Source operations
def source_operation(
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
    """Source operations wrapper"""
    return create_letta_wrapper(
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
