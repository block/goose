# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument
"""
Module for monitoring Crawl4AI synchronous calls.
Supports comprehensive 0.7.x operations with business intelligence and enhanced observability.
"""

import logging
import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api

from openlit.__helpers import handle_exception
from openlit.instrumentation.crawl4ai.utils import (
    Crawl4AIInstrumentationContext,
    get_operation_name,
    create_crawl_span_name,
    set_crawl_attributes,
    process_crawl_response,
    capture_message_content_if_enabled,
    process_llm_extraction_response,
    create_extraction_span_name,
    capture_extraction_content,
)

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


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
    Generates a telemetry wrapper for Crawl4AI operations to collect comprehensive metrics.

    Args:
        gen_ai_endpoint: Endpoint identifier for logging and tracing.
        version: Version of the Crawl4AI package.
        environment: Deployment environment (e.g., production, staging).
        application_name: Name of the application using Crawl4AI.
        tracer: OpenTelemetry tracer for creating spans.
        pricing_info: Information used for calculating the cost of Crawl4AI usage.
        capture_message_content: Flag indicating whether to trace the actual content.
        metrics: OpenLIT metrics dictionary for performance tracking.
        disable_metrics: Flag to disable metrics collection.

    Returns:
        A function that wraps Crawl4AI methods to add comprehensive telemetry.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps Crawl4AI operations to add comprehensive telemetry.

        This collects metrics such as execution time, URL information, configuration details,
        and result metrics, while handling errors gracefully for enhanced observability.

        Args:
            wrapped: The original Crawl4AI method to be wrapped.
            instance: The instance of the class where the original method is defined.
            args: Positional arguments for the Crawl4AI method.
            kwargs: Keyword arguments for the Crawl4AI method.

        Returns:
            The response from the original Crawl4AI method.
        """

        # Check if instrumentation is suppressed
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        # Get operation name and create context
        operation_name = get_operation_name(gen_ai_endpoint)
        ctx = Crawl4AIInstrumentationContext(
            instance, args, kwargs, version, environment, application_name
        )

        # Create span name following the framework guide pattern
        if operation_name == "extract":
            # For extraction operations, determine strategy type from the endpoint
            strategy_type = None
            if "llm" in gen_ai_endpoint:
                strategy_type = "llm"
            elif "css" in gen_ai_endpoint:
                strategy_type = "css"
            elif "xpath" in gen_ai_endpoint:
                strategy_type = "xpath"
            elif "cosine" in gen_ai_endpoint:
                strategy_type = "cosine"
            elif "regex" in gen_ai_endpoint:
                strategy_type = "regex"

            target_url = ctx.url if ctx.url != "unknown" else "content"
            span_name = create_extraction_span_name(
                operation_name, strategy_type, target_url
            )
        else:
            # For crawl operations, use the new improved naming
            span_name = create_crawl_span_name(operation_name, ctx, gen_ai_endpoint)

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            try:
                # Set comprehensive span attributes
                set_crawl_attributes(span, ctx, operation_name)

                # Execute the original function
                response = wrapped(*args, **kwargs)

                # Calculate duration for business intelligence
                end_time = time.time()
                duration_ms = (end_time - start_time) * 1000
                span.set_attribute("gen_ai.client.operation.duration", duration_ms)

                # Process response based on operation type
                if operation_name == "extract":
                    # Process extraction strategy response with LLM-specific metrics
                    if hasattr(instance, "__class__"):
                        # For extraction strategy operations, the instance is the strategy
                        process_llm_extraction_response(span, instance, response, ctx)

                    # Capture extraction content if enabled
                    extraction_input = None
                    if len(args) >= 3:
                        extraction_input = args[2]  # html content
                    elif len(args) >= 2:
                        extraction_input = args[1]  # html_content parameter

                    capture_extraction_content(
                        span, extraction_input, response, capture_message_content
                    )
                else:
                    # Process crawl operations
                    process_crawl_response(span, response, ctx)

                    # Capture message content for crawl operations
                    capture_message_content_if_enabled(
                        span, ctx, response, capture_message_content
                    )

                # Record metrics if enabled
                if not disable_metrics and metrics:
                    try:
                        # Record operation metrics
                        metrics_key = f"crawl4ai.{operation_name}.duration"
                        if metrics_key not in metrics:
                            metrics[metrics_key] = []
                        metrics[metrics_key].append(duration_ms)

                        # Record success/failure metrics
                        success_key = f"crawl4ai.{operation_name}.success"
                        if success_key not in metrics:
                            metrics[success_key] = 0

                        # Determine success based on response
                        is_success = True
                        if hasattr(response, "success"):
                            is_success = response.success
                        elif isinstance(response, list):
                            is_success = all(
                                getattr(r, "success", True) for r in response
                            )

                        if is_success:
                            metrics[success_key] += 1

                    except Exception as metrics_error:
                        logger.debug("Error recording metrics: %s", metrics_error)

                return response

            except Exception as e:
                # Calculate duration even for errors
                end_time = time.time()
                duration_ms = (end_time - start_time) * 1000
                span.set_attribute("gen_ai.client.operation.duration", duration_ms)

                # Handle and log the exception
                handle_exception(span, e)
                logger.error("Error in Crawl4AI operation: %s", e)

                # Record error metrics if enabled
                if not disable_metrics and metrics:
                    try:
                        error_key = f"crawl4ai.{operation_name}.error"
                        if error_key not in metrics:
                            metrics[error_key] = 0
                        metrics[error_key] += 1
                    except Exception as metrics_error:
                        logger.debug("Error recording error metrics: %s", metrics_error)

                # Re-raise the exception to maintain original behavior
                raise

    return wrapper
