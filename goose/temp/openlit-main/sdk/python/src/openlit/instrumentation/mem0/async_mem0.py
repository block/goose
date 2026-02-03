# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument
"""
Optimized async mem0 instrumentation following OpenLIT Framework Guide.
"""

import time
from opentelemetry.trace import SpanKind, Status, StatusCode, set_span_in_context
from opentelemetry import context
from opentelemetry.instrumentation.utils import _SUPPRESS_INSTRUMENTATION_KEY
from openlit.__helpers import handle_exception
from openlit.instrumentation.mem0.utils import (
    Mem0Context,
    set_mem0_span_attributes,
    set_mem0_content_attributes,
    logger,
)


def async_mem0_wrap(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
):
    """
    Optimized wrapper for async mem0 operations with proper hierarchy and performance.

    Returns:
        Wrapper function for async mem0 operations
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Inner wrapper with comprehensive tracing for async operations.
        """
        # Suppression check
        if context.get_value(_SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Handle AsyncMemory.__init__ to create parent span for initialization operations
        if gen_ai_endpoint == "memory init":
            with tracer.start_as_current_span("memory init", kind=SpanKind.INTERNAL):
                return wrapped(*args, **kwargs)

        # Determine span type efficiently
        is_internal = any(
            marker in gen_ai_endpoint
            for marker in ["_", "vector_store", "graph", "create_memory", "init"]
        )
        span_kind = SpanKind.INTERNAL if is_internal else SpanKind.CLIENT

        # Create optimized context
        ctx = Mem0Context(
            instance, args, kwargs, version, environment, application_name
        )
        current_context = context.get_current()

        with tracer.start_as_current_span(
            gen_ai_endpoint, kind=span_kind, context=current_context
        ) as span:
            span_context = set_span_in_context(span, context=current_context)
            token = context.attach(span_context)

            start_time = time.perf_counter()

            try:
                # Execute async operation (no threading patches needed for async)
                response = wrapped(*args, **kwargs)

                # Calculate duration
                duration = time.perf_counter() - start_time

                # Set span attributes
                set_mem0_span_attributes(span, gen_ai_endpoint, ctx, response)
                span.set_attribute("gen_ai.client.operation.duration", duration)

                # Set content attributes if enabled
                set_mem0_content_attributes(
                    span, gen_ai_endpoint, ctx, response, capture_message_content
                )

                span.set_status(Status(StatusCode.OK))
                return response

            except Exception as e:
                duration = time.perf_counter() - start_time
                span.set_attribute("gen_ai.client.operation.duration", duration)

                try:
                    set_mem0_span_attributes(span, gen_ai_endpoint, ctx)
                except Exception as attr_error:
                    logger.debug(
                        "Failed to set span attributes on error: %s", attr_error
                    )

                handle_exception(span, e)
                raise

            finally:
                context.detach(token)

    return wrapper
