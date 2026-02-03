"""
Async instrumentation for Firecrawl operations
"""

import logging

from opentelemetry import context as context_api
from opentelemetry.trace import SpanKind

from openlit.instrumentation.firecrawl.utils import (
    FirecrawlInstrumentationContext,
    get_operation_name,
    get_span_name,
    set_span_attributes,
    process_response,
    handle_firecrawl_error,
)

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


def async_general_wrap(
    endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    trace_content,
    metrics,
    disable_metrics,
):
    """
    Creates a telemetry wrapper for async Firecrawl operations to collect comprehensive metrics.

    Args:
        endpoint: Endpoint identifier for logging and tracing.
        version: Version of the monitoring package.
        environment: Deployment environment (e.g., production, staging).
        application_name: Name of the application using Firecrawl.
        tracer: OpenTelemetry tracer for creating spans.
        pricing_info: Information used for calculating the cost of Firecrawl usage.
        trace_content: Flag indicating whether to trace the actual content.
        metrics: Metrics dictionary for collecting telemetry data.
        disable_metrics: Flag to disable metrics collection.

    Returns:
        A function that wraps async Firecrawl methods to add comprehensive telemetry.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Enhanced async wrapper function with business intelligence and error handling.
        Collects comprehensive telemetry for async Firecrawl operations.
        """

        async def async_wrapper(*args, **kwargs):
            # Implement suppression check per framework guide
            if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                return await wrapped(*args, **kwargs)

            try:
                # Create context object for caching expensive operations
                ctx = FirecrawlInstrumentationContext(
                    instance, args, kwargs, version, environment, application_name
                )

                # Get operation details
                operation_name = get_operation_name(endpoint)
                span_name = get_span_name(operation_name, ctx, endpoint)

                # Start span with proper hierarchy
                with tracer.start_as_current_span(
                    span_name, kind=SpanKind.CLIENT
                ) as span:
                    # Set comprehensive span attributes
                    set_span_attributes(
                        span,
                        operation_name,
                        ctx,
                        endpoint=endpoint,
                        pricing_info=pricing_info,
                        trace_content=trace_content,
                        **kwargs,
                    )

                    try:
                        # Execute the async wrapped function - outside try block per framework guide
                        response = await wrapped(*args, **kwargs)

                        # Process response and capture telemetry
                        try:
                            process_response(
                                span,
                                response,
                                ctx,
                                endpoint=endpoint,
                                trace_content=trace_content,
                                **kwargs,
                            )
                        except Exception as e:
                            handle_firecrawl_error(span, e)

                        return response

                    except Exception as e:
                        handle_firecrawl_error(span, e)
                        raise

            except Exception as e:
                logger.debug(
                    "Failed to create async firecrawl telemetry wrapper: %s", e
                )
                return await wrapped(*args, **kwargs)

        return async_wrapper(*args, **kwargs)

    return wrapper
