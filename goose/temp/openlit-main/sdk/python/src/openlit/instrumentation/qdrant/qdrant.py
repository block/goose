# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument, possibly-used-before-assignment, too-many-branches
"""
Module for monitoring Qdrant operations.
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.qdrant.utils import (
    DB_OPERATION_MAP,
    process_qdrant_response,
    set_server_address_and_port,
)


def general_wrap(
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
    Generates a telemetry wrapper for Pinecone function calls.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the Qdrant operation with tracing and logging.
        """
        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Extract operation from endpoint
        db_operation = DB_OPERATION_MAP.get(gen_ai_endpoint, "unknown")

        # Server address calculation
        server_address, server_port = set_server_address_and_port(instance)

        # Span naming: use operation + collection
        collection_name = kwargs.get("collection_name", "unknown")
        span_name = f"{db_operation} {collection_name}"

        # CRITICAL: Use tracer.start_as_current_span() for proper context
        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                # Process response with endpoint information
                response = process_qdrant_response(
                    response,
                    db_operation,
                    server_address,
                    server_port,
                    environment,
                    application_name,
                    metrics,
                    start_time,
                    span,
                    capture_message_content,
                    disable_metrics,
                    version,
                    instance=instance,
                    args=args,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
