"""
Module for monitoring Browser-Use synchronous operations.
Handles the few sync methods in browser-use (pause, resume, stop).
"""

import json
import logging
import time
from typing import Any
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api

from openlit.__helpers import handle_exception, common_framework_span_attributes
from openlit.instrumentation.browser_use.utils import (
    BrowserUseInstrumentationContext,
    get_operation_name,
    capture_token_and_cost_metrics,
)
from openlit.semcov import SemanticConvention

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
    Enhanced telemetry wrapper for Browser-Use sync operations with detailed spans.
    Handles sync methods like pause, resume, stop.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """Enhanced wrapper with detailed span creation."""

        # Check if instrumentation is suppressed
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return wrapped(*args, **kwargs)

        operation_name = get_operation_name(gen_ai_endpoint)
        ctx = BrowserUseInstrumentationContext(
            instance, args, kwargs, version, environment, application_name
        )

        # Create enhanced span name
        span_name = _create_enhanced_span_name(operation_name, ctx)

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as main_span:
            start_time = time.time()

            try:
                # Set enhanced span attributes
                _set_enhanced_span_attributes(
                    main_span, operation_name, ctx, start_time, start_time
                )

                # Execute the original sync function
                logger.debug(
                    "Executing enhanced Browser-Use sync operation: %s", operation_name
                )
                response = wrapped(*args, **kwargs)

                # Calculate duration
                end_time = time.time()
                duration_ms = (end_time - start_time) * 1000
                main_span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration_ms
                )

                # Process response with enhanced details
                _process_enhanced_response(
                    main_span, response, ctx, capture_message_content
                )

                # Capture token usage and cost metrics
                model_name = ctx.model_name
                if model_name != "unknown" and pricing_info:
                    capture_token_and_cost_metrics(
                        main_span, response, model_name, pricing_info
                    )

                # Record metrics if enabled
                if not disable_metrics and metrics:
                    _record_operation_metrics(
                        metrics, operation_name, duration_ms, True
                    )

                return response

            except Exception as e:
                # Calculate duration even for errors
                end_time = time.time()
                duration_ms = (end_time - start_time) * 1000
                main_span.set_attribute(
                    SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, duration_ms
                )

                # Handle and log the exception
                handle_exception(main_span, e)
                logger.error(
                    "Error in enhanced Browser-Use sync operation %s: %s",
                    operation_name,
                    e,
                )

                # Record error metrics if enabled
                if not disable_metrics and metrics:
                    _record_operation_metrics(
                        metrics, operation_name, duration_ms, False
                    )

                # Re-raise the exception to maintain original behavior
                raise

    return wrapper


def _create_enhanced_span_name(
    operation_name: str, ctx: BrowserUseInstrumentationContext
) -> str:
    """Create enhanced span names with more context."""

    if operation_name == "run":
        return f"invoke_agent {ctx.agent_name}"
    if operation_name == "step":
        step_count = ctx.step_count
        if step_count is not None:
            return f"execute_task step {step_count}"
        model_name = ctx.model_name if ctx.model_name != "unknown" else "llm"
        return f"{SemanticConvention.GEN_AI_SPAN_INVOKE_MODEL} {model_name}"
    return f"browser {operation_name}"


class SpanScope:
    """Simple scope wrapper for common_framework_span_attributes."""

    def __init__(self, span, start_time, end_time):
        self._span = span
        self._start_time = start_time
        self._end_time = end_time

    @property
    def span(self):
        """Get the span."""
        return self._span

    @property
    def start_time(self):
        """Get the start time."""
        return self._start_time


def _set_enhanced_span_attributes(
    span: Any,
    operation_name: str,
    ctx: BrowserUseInstrumentationContext,
    start_time: float,
    end_time: float,
) -> None:
    """Set enhanced span attributes with detailed information using common framework helpers."""

    # Create scope for common_framework_span_attributes
    scope = SpanScope(span, start_time, end_time)

    # Use common framework span attributes like crewai does
    common_framework_span_attributes(
        scope,
        SemanticConvention.GEN_AI_SYSTEM_BROWSER_USE,
        "browser-use.com",  # server_address
        443,  # server_port
        ctx.environment,
        ctx.application_name,
        ctx.version,
        operation_name,
        ctx,  # instance for model extraction
    )

    # Add browser-use specific attributes
    _add_browser_use_attributes(span, operation_name, ctx)


def _add_browser_use_attributes(
    span: Any, operation_name: str, ctx: BrowserUseInstrumentationContext
) -> None:
    """Add browser-use specific attributes based on SDK research."""

    # Enhanced agent attributes
    span.set_attribute(SemanticConvention.GEN_AI_AGENT_NAME, ctx.agent_name)
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_TYPE,
        SemanticConvention.GEN_AI_AGENT_TYPE_BROWSER,
    )

    # Task description (agent description)
    agent_desc = ctx.agent_description
    if agent_desc != "browser_automation_task":
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_DESCRIPTION, agent_desc)

    # Current URL if available
    current_url = ctx.current_url
    if current_url:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_BROWSE_URL, current_url)

    # Step information
    step_count = ctx.step_count
    if step_count is not None:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_STEP_COUNT, step_count)

    max_steps = ctx.max_steps
    if max_steps is not None:
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_MAX_STEPS, max_steps)

    # Browser-use specific agent information
    try:
        instance = ctx.instance
        if hasattr(instance, "id"):
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_ID, str(instance.id))
        if hasattr(instance, "task_id"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_TASK_ID, str(instance.task_id)
            )
        if hasattr(instance, "session_id"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_SESSION_ID, str(instance.session_id)
            )

        # Agent settings information
        if hasattr(instance, "settings"):
            settings = instance.settings
            if hasattr(settings, "use_vision"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_USE_VISION, settings.use_vision
                )
            if hasattr(settings, "max_failures"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_MAX_FAILURES, settings.max_failures
                )
            if hasattr(settings, "max_actions_per_step"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_MAX_ACTIONS_PER_STEP,
                    settings.max_actions_per_step,
                )

        # Browser profile information
        if hasattr(instance, "browser_profile"):
            profile = instance.browser_profile
            if hasattr(profile, "headless"):
                headless_value = profile.headless
                if headless_value is not None:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_HEADLESS, bool(headless_value)
                    )
            if hasattr(profile, "allowed_domains") and profile.allowed_domains:
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_ALLOWED_DOMAINS,
                    json.dumps(profile.allowed_domains),
                )

    except Exception as e:
        logger.debug("Error capturing browser-use specific attributes: %s", e)

    # Operation type mapping
    operation_type_map = {
        "run": SemanticConvention.GEN_AI_OPERATION_TYPE_AGENT,
        "step": SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
        "pause": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
        "resume": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
        "stop": SemanticConvention.GEN_AI_OPERATION_TYPE_FRAMEWORK,
    }

    if operation_name in operation_type_map:
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION_TYPE, operation_type_map[operation_name]
        )


def _calculate_step_stats(history):
    """Calculate success/failure statistics for steps."""
    successful_steps = 0
    failed_steps = 0
    total_actions = 0

    for step in history:
        if step.result:
            step_success = True
            for result in step.result:
                total_actions += 1
                if hasattr(result, "is_success") and not result.is_success:
                    step_success = False
                elif hasattr(result, "error") and result.error:
                    step_success = False

            if step_success:
                successful_steps += 1
            else:
                failed_steps += 1

    return {
        "successful_steps": successful_steps,
        "failed_steps": failed_steps,
        "total_actions": total_actions,
    }


def _process_enhanced_response(
    span: Any,
    response: Any,
    _ctx: BrowserUseInstrumentationContext,
    capture_message_content: bool,
) -> None:
    """Process response with enhanced details and summary information."""

    try:
        # Handle AgentHistoryList (from agent.run())
        if hasattr(response, "history") and hasattr(response, "usage"):
            history_list = response

            # Overall execution summary
            total_steps = len(history_list.history)
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_STEP_COUNT, total_steps)

            # Calculate success/failure stats
            stats = _calculate_step_stats(history_list.history)
            successful_steps = stats["successful_steps"]
            failed_steps = stats["failed_steps"]
            total_actions = stats["total_actions"]

            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_TOTAL_ACTIONS, total_actions
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_SUCCESSFUL_STEPS, successful_steps
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_FAILED_STEPS, failed_steps
            )

            if total_steps > 0:
                success_rate = (successful_steps / total_steps) * 100
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_SUCCESS_RATE, success_rate
                )

            # Final result
            final_result = history_list.final_result()
            if final_result and capture_message_content:
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_FINAL_RESULT,
                    str(final_result)[:1000],
                )

            # Usage summary if available
            if history_list.usage:
                usage = history_list.usage
                if hasattr(usage, "total_input_tokens") and usage.total_input_tokens:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS,
                        usage.total_input_tokens,
                    )
                if hasattr(usage, "total_output_tokens") and usage.total_output_tokens:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS,
                        usage.total_output_tokens,
                    )
                if hasattr(usage, "total_cost") and usage.total_cost:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_USAGE_COST, usage.total_cost
                    )

            # Total duration
            if hasattr(history_list, "total_duration_seconds"):
                duration = history_list.total_duration_seconds()
                if duration:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_EXECUTION_TIME, duration
                    )

        # Handle single action results
        elif hasattr(response, "is_success") or hasattr(response, "error"):
            if hasattr(response, "is_success"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_ACTION_SUCCESS, response.is_success
                )

            if hasattr(response, "error") and response.error:
                span.set_attribute(SemanticConvention.ERROR_TYPE, "action_failed")
                if capture_message_content:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_ACTION_ERROR,
                        str(response.error)[:200],
                    )

    except Exception as e:
        logger.debug("Error processing enhanced response: %s", e)


def _record_operation_metrics(metrics, operation_name, duration_ms, is_success):
    """Record operation metrics for performance tracking."""

    try:
        # Record duration metrics
        duration_key = f"browser_use.{operation_name}.duration"
        if duration_key not in metrics:
            metrics[duration_key] = []
        metrics[duration_key].append(duration_ms)

        # Record success/error metrics
        if is_success:
            success_key = f"browser_use.{operation_name}.success"
            if success_key not in metrics:
                metrics[success_key] = 0
            metrics[success_key] += 1
        else:
            error_key = f"browser_use.{operation_name}.error"
            if error_key not in metrics:
                metrics[error_key] = 0
            metrics[error_key] += 1

    except Exception as e:
        logger.debug("Error recording operation metrics: %s", e)
