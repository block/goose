# pylint: disable=duplicate-code, broad-exception-caught, too-many-statements, unused-argument
"""
Module for monitoring Crawl4AI asynchronous calls.
Supports comprehensive 0.7.x operations with business intelligence and enhanced observability.
Includes support for async generators and streaming responses.
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


def async_general_wrap(
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
    Generates a telemetry wrapper for async Crawl4AI operations to collect comprehensive metrics.

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
        A function that wraps async Crawl4AI methods to add comprehensive telemetry.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps async Crawl4AI operations to add comprehensive telemetry.

        This collects metrics such as execution time, URL information, configuration details,
        and result metrics, while handling errors gracefully for enhanced observability.
        Supports both regular async methods and async generators for streaming.

        Args:
            wrapped: The original async Crawl4AI method to be wrapped.
            instance: The instance of the class where the original method is defined.
            args: Positional arguments for the Crawl4AI method.
            kwargs: Keyword arguments for the Crawl4AI method.

        Returns:
            The response from the original async Crawl4AI method.
        """

        async def async_wrapper():
            # Check if instrumentation is suppressed
            if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                return await wrapped(*args, **kwargs)

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

                    # Execute the original async function
                    response = await wrapped(*args, **kwargs)

                    # Handle async generators (streaming responses)
                    if hasattr(response, "__aiter__"):
                        return await _handle_async_generator(
                            span,
                            response,
                            ctx,
                            start_time,
                            operation_name,
                            capture_message_content,
                            metrics,
                            disable_metrics,
                        )

                    # Handle regular responses
                    return await _handle_regular_response(
                        span,
                        response,
                        ctx,
                        start_time,
                        operation_name,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                        instance,  # Add missing parameter
                        args,  # Add missing parameter
                    )

                except Exception as e:
                    # Calculate duration even for errors
                    end_time = time.time()
                    duration_ms = (end_time - start_time) * 1000
                    span.set_attribute("gen_ai.client.operation.duration", duration_ms)

                    # Handle and log the exception
                    handle_exception(span, e)
                    logger.error("Error in async Crawl4AI operation: %s", e)

                    # Record error metrics if enabled
                    if not disable_metrics and metrics:
                        try:
                            error_key = f"crawl4ai.{operation_name}.error"
                            if error_key not in metrics:
                                metrics[error_key] = 0
                            metrics[error_key] += 1
                        except Exception as metrics_error:
                            logger.debug(
                                "Error recording error metrics: %s", metrics_error
                            )

                    # Re-raise the exception to maintain original behavior
                    raise

        return async_wrapper()

    return wrapper


async def _handle_regular_response(
    span,
    response,
    ctx,
    start_time,
    operation_name,
    capture_message_content,
    metrics,
    disable_metrics,
    instance,
    args,
):
    """Handle regular (non-streaming) async responses."""

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
        capture_message_content_if_enabled(span, ctx, response, capture_message_content)

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
                is_success = all(getattr(r, "success", True) for r in response)

            if is_success:
                metrics[success_key] += 1

        except Exception as metrics_error:
            logger.debug("Error recording metrics: %s", metrics_error)

    return response


async def _handle_async_generator(
    span,
    response_generator,
    ctx,
    start_time,
    operation_name,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Handle async generator responses (streaming)."""

    class TrackedAsyncGenerator:
        """Async generator wrapper to track streaming results."""

        def __init__(
            self,
            generator,
            span,
            ctx,
            start_time,
            operation_name,
            capture_message_content,
            metrics,
            disable_metrics,
        ):
            self.generator = generator
            self.span = span
            self.ctx = ctx
            self.start_time = start_time
            self.operation_name = operation_name
            self.capture_message_content = capture_message_content
            self.metrics = metrics
            self.disable_metrics = disable_metrics
            self.results = []
            self.completed = False

        def __aiter__(self):
            return self

        async def __anext__(self):
            try:
                result = await self.generator.__anext__()
                self.results.append(result)

                # Update span with current result
                if hasattr(result, "url"):
                    self.span.set_attribute("gen_ai.crawl.current_url", result.url)

                return result

            except StopAsyncIteration:
                # Generator is complete, finalize metrics
                if not self.completed:
                    await self._finalize_metrics()
                    self.completed = True
                raise
            except Exception as e:
                # Handle errors in streaming
                handle_exception(self.span, e)
                logger.error("Error in streaming crawl operation: %s", e)
                raise

        async def _finalize_metrics(self):
            """Finalize metrics when streaming is complete."""
            end_time = time.time()
            duration_ms = (end_time - self.start_time) * 1000
            self.span.set_attribute("gen_ai.client.operation.duration", duration_ms)

            # Process aggregated results
            if self.results:
                process_crawl_response(self.span, self.results, self.ctx)

                # Capture message content for streaming
                capture_message_content_if_enabled(
                    self.span, self.ctx, self.results, self.capture_message_content
                )

                # Record streaming metrics
                if not self.disable_metrics and self.metrics:
                    try:
                        metrics_key = f"crawl4ai.{self.operation_name}.stream.duration"
                        if metrics_key not in self.metrics:
                            self.metrics[metrics_key] = []
                        self.metrics[metrics_key].append(duration_ms)

                        # Record stream completion
                        stream_key = f"crawl4ai.{self.operation_name}.stream.completed"
                        if stream_key not in self.metrics:
                            self.metrics[stream_key] = 0
                        self.metrics[stream_key] += 1

                    except Exception as metrics_error:
                        logger.debug(
                            "Error recording streaming metrics: %s", metrics_error
                        )

    # Return the tracked generator
    return TrackedAsyncGenerator(
        response_generator,
        span,
        ctx,
        start_time,
        operation_name,
        capture_message_content,
        metrics,
        disable_metrics,
    )
