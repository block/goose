"""
Module for monitoring Browser-Use asynchronous operations.
Creates detailed spans with agent settings, step-by-step execution, and individual actions.
"""

import asyncio
import json
import logging
import time
from typing import Any, List
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
    Enhanced telemetry wrapper for Browser-Use async operations with detailed spans.
    Creates agent settings spans, step-by-step execution tracking, and individual action spans.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """Enhanced wrapper with detailed span creation and step hooks."""

        async def _async_wrapper():
            # Check if instrumentation is suppressed
            if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
                return await wrapped(*args, **kwargs)

            operation_name = get_operation_name(gen_ai_endpoint)
            ctx = BrowserUseInstrumentationContext(
                instance, args, kwargs, version, environment, application_name
            )

            # Create enhanced span name
            span_name = _create_enhanced_span_name(operation_name, ctx)

            with tracer.start_as_current_span(
                span_name, kind=SpanKind.CLIENT
            ) as main_span:
                start_time = time.time()

                try:
                    # Set enhanced span attributes (before execution, so end_time not available yet)
                    _set_enhanced_span_attributes(
                        main_span, operation_name, ctx, start_time, start_time
                    )

                    # For agent.run(), add configuration to main span and install hooks
                    if operation_name == "run":
                        _add_agent_configuration_attributes(main_span, instance, ctx)
                        _install_step_hooks(
                            tracer,
                            main_span,
                            instance,
                            ctx,
                            capture_message_content,
                            kwargs,
                        )

                    # Execute the original async function
                    logger.debug(
                        "Executing enhanced Browser-Use async operation: %s",
                        operation_name,
                    )
                    response = await wrapped(*args, **kwargs)

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
                        "Error in enhanced Browser-Use async operation %s: %s",
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

        return _async_wrapper()

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


def _add_agent_configuration_attributes(
    span: Any, agent_instance: Any, _ctx: BrowserUseInstrumentationContext
) -> None:
    """Add agent configuration attributes to the main execution span."""

    try:
        # Agent settings from the instance
        if hasattr(agent_instance, "settings"):
            settings = agent_instance.settings

            # Vision settings
            if hasattr(settings, "use_vision"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_USE_VISION, settings.use_vision
                )
            if hasattr(settings, "vision_detail_level"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_VISION_DETAIL_LEVEL,
                    settings.vision_detail_level,
                )

            # Execution settings
            if hasattr(settings, "max_failures"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_MAX_FAILURES, settings.max_failures
                )
            if hasattr(settings, "retry_delay"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_RETRY_DELAY, settings.retry_delay
                )
            if hasattr(settings, "validate_output"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_VALIDATE_OUTPUT,
                    settings.validate_output,
                )
            if hasattr(settings, "max_actions_per_step"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_MAX_ACTIONS_PER_STEP,
                    settings.max_actions_per_step,
                )

            # LLM settings
            if hasattr(settings, "llm_timeout"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_LLM_TIMEOUT, settings.llm_timeout
                )

        # Browser profile information
        if hasattr(agent_instance, "browser_profile"):
            profile = agent_instance.browser_profile
            if hasattr(profile, "allowed_domains") and profile.allowed_domains:
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_ALLOWED_DOMAINS,
                    json.dumps(profile.allowed_domains),
                )
            if hasattr(profile, "headless"):
                headless_value = profile.headless
                if headless_value is not None:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_HEADLESS, bool(headless_value)
                    )

        # Task ID and session information
        if hasattr(agent_instance, "task_id"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_TASK_ID, str(agent_instance.task_id)
            )
        if hasattr(agent_instance, "session_id"):
            span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_SESSION_ID,
                str(agent_instance.session_id),
            )

    except Exception as e:
        logger.debug("Error capturing agent configuration: %s", e)


def _install_step_hooks(
    tracer: Any,
    _parent_span: Any,
    _agent_instance: Any,
    ctx: BrowserUseInstrumentationContext,
    capture_message_content: bool,
    run_kwargs: dict,
) -> None:
    """Install step hooks to create detailed spans for each agent step."""

    # Get original hooks from run() kwargs
    original_on_step_start = run_kwargs.get("on_step_start", None)
    original_on_step_end = run_kwargs.get("on_step_end", None)

    async def enhanced_step_start_hook(agent):
        """Enhanced step start hook."""
        try:
            # Call original hook if it exists
            if original_on_step_start:
                if asyncio.iscoroutinefunction(original_on_step_start):
                    await original_on_step_start(agent)
                else:
                    original_on_step_start(agent)
        except Exception as e:
            logger.debug("Error in original step start hook: %s", e)

    async def enhanced_step_end_hook(agent):
        """Enhanced step end hook that creates detailed step spans."""
        try:
            # Create detailed step span with all the information we can extract
            await _create_detailed_step_span(
                tracer, agent, ctx, capture_message_content
            )

            # Call original hook if it exists
            if original_on_step_end:
                if asyncio.iscoroutinefunction(original_on_step_end):
                    await original_on_step_end(agent)
                else:
                    original_on_step_end(agent)
        except Exception as e:
            logger.debug("Error in enhanced step end hook: %s", e)

    # Replace the hooks in run_kwargs so they get used
    run_kwargs["on_step_start"] = enhanced_step_start_hook
    run_kwargs["on_step_end"] = enhanced_step_end_hook


async def _create_detailed_step_span(
    tracer: Any,
    agent_instance: Any,
    ctx: BrowserUseInstrumentationContext,
    capture_message_content: bool,
) -> None:
    """Create a detailed span for a completed agent step."""

    try:
        # Get the latest step information
        if (
            not hasattr(agent_instance, "state")
            or not agent_instance.state.history.history
        ):
            return

        latest_step = agent_instance.state.history.history[-1]
        step_number = len(agent_instance.state.history.history)

        span_name = f"execute_task step {step_number}"

        with tracer.start_as_current_span(
            span_name, kind=SpanKind.INTERNAL
        ) as step_span:
            # Basic step information
            step_span.set_attribute(
                SemanticConvention.GEN_AI_SYSTEM,
                SemanticConvention.GEN_AI_SYSTEM_BROWSER_USE,
            )
            step_span.set_attribute(SemanticConvention.GEN_AI_OPERATION, "step")
            step_span.set_attribute(
                SemanticConvention.GEN_AI_OPERATION_TYPE,
                SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
            )
            step_span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_NAME, ctx.agent_name
            )
            step_span.set_attribute(
                SemanticConvention.GEN_AI_AGENT_STEP_COUNT, step_number
            )

            # Extract agent thoughts and decision-making
            if latest_step.model_output:
                model_output = latest_step.model_output

                # Agent thoughts and reasoning
                if hasattr(model_output, "thinking") and model_output.thinking:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_THINKING,
                        str(model_output.thinking)[:500],
                    )

                if hasattr(model_output, "memory") and model_output.memory:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_MEMORY,
                        str(model_output.memory)[:500],
                    )

                if hasattr(model_output, "next_goal") and model_output.next_goal:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_NEXT_GOAL,
                        str(model_output.next_goal)[:200],
                    )

                if (
                    hasattr(model_output, "evaluation_previous_goal")
                    and model_output.evaluation_previous_goal
                ):
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_EVALUATION,
                        str(model_output.evaluation_previous_goal)[:200],
                    )

                # Actions taken in this step
                if hasattr(model_output, "action") and model_output.action:
                    await _create_individual_action_spans(
                        tracer,
                        step_span,
                        model_output.action,
                        latest_step,
                        ctx,
                        capture_message_content,
                    )

                    # Summary of actions
                    actions_summary = []
                    for action in model_output.action:
                        if hasattr(action, "model_dump"):
                            action_dict = action.model_dump()
                            # Get the action type (first key that's not empty)
                            action_type = next(
                                (k for k, v in action_dict.items() if v), "unknown"
                            )
                            actions_summary.append(f"{action_type}")

                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_ACTIONS,
                        json.dumps(actions_summary),
                    )
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_ACTIONS_COUNT,
                        len(model_output.action),
                    )

            # Browser state information
            if latest_step.state:
                state = latest_step.state

                # Current URL and page title
                if hasattr(state, "url") and state.url:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_BROWSE_URL, state.url
                    )

                if hasattr(state, "title") and state.title:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_PAGE_TITLE, state.title[:100]
                    )

                # Tab information
                if hasattr(state, "tabs") and state.tabs:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_TABS_COUNT, len(state.tabs)
                    )

                # Elements interacted with
                if hasattr(state, "interacted_element") and state.interacted_element:
                    interacted_count = sum(
                        1 for el in state.interacted_element if el is not None
                    )
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_INTERACTED_ELEMENTS_COUNT,
                        interacted_count,
                    )

            # Action results summary
            if latest_step.result:
                success_count = 0
                error_count = 0
                error_messages = []

                for result in latest_step.result:
                    if hasattr(result, "is_success"):
                        if result.is_success:
                            success_count += 1
                        else:
                            error_count += 1

                    if hasattr(result, "error") and result.error:
                        error_messages.append(str(result.error)[:100])

                step_span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_ACTIONS_SUCCESS_COUNT, success_count
                )
                step_span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_ACTIONS_ERROR_COUNT, error_count
                )

                if error_messages and capture_message_content:
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_ACTION_ERRORS,
                        json.dumps(error_messages),
                    )

            # Step metadata (timing, etc.)
            if latest_step.metadata:
                metadata = latest_step.metadata
                if hasattr(metadata, "duration_seconds"):
                    step_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_STEP_DURATION,
                        metadata.duration_seconds,
                    )

    except Exception as e:
        logger.debug("Error creating detailed step span: %s", e)


async def _create_individual_action_spans(
    tracer: Any,
    _parent_span: Any,
    actions: List[Any],
    step: Any,
    ctx: BrowserUseInstrumentationContext,
    capture_message_content: bool,
) -> None:
    """Create individual spans for each action in a step."""

    try:
        for i, action in enumerate(actions):
            if hasattr(action, "model_dump"):
                action_dict = action.model_dump()
                # Get the action type (first key that's not empty)
                action_type = next((k for k, v in action_dict.items() if v), "unknown")
                action_data = action_dict.get(action_type, {})

                span_name = f"invoke_agent {action_type}"

                with tracer.start_as_current_span(
                    span_name, kind=SpanKind.INTERNAL
                ) as action_span:
                    # Basic action information
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_SYSTEM,
                        SemanticConvention.GEN_AI_SYSTEM_BROWSER_USE,
                    )
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_OPERATION, "invoke_agent"
                    )
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_OPERATION_TYPE,
                        SemanticConvention.GEN_AI_OPERATION_TYPE_EXECUTE_AGENT_TASK,
                    )
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_AGENT_NAME, ctx.agent_name
                    )
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_ACTION_TYPE, action_type
                    )
                    action_span.set_attribute(
                        SemanticConvention.GEN_AI_ACTION_INDEX, i + 1
                    )

                    # Enhanced action-specific parameters
                    if isinstance(action_data, dict):
                        # Core action parameters
                        for key, value in action_data.items():
                            if key in [
                                "index",
                                "text",
                                "url",
                                "query",
                                "selector",
                                "new_tab",
                                "down",
                                "num_pages",
                            ]:
                                action_span.set_attribute(
                                    f"gen_ai.action.{key}", str(value)[:200]
                                )

                        # Special handling for sensitive data
                        if action_type == "input_text" and action_data.get(
                            "has_sensitive_data"
                        ):
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_ACTION_HAS_SENSITIVE_DATA,
                                True,
                            )

                        # File operations
                        if action_type in ["upload_file", "write_file", "read_file"]:
                            if "path" in action_data or "file_name" in action_data:
                                file_path = action_data.get(
                                    "path", action_data.get("file_name", "")
                                )
                                action_span.set_attribute(
                                    SemanticConvention.GEN_AI_ACTION_FILE_PATH,
                                    str(file_path)[:100],
                                )

                    # Enhanced action result tracking
                    if step.result and i < len(step.result):
                        result = step.result[i]
                        if hasattr(result, "is_success"):
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_ACTION_SUCCESS,
                                result.is_success,
                            )
                        if (
                            hasattr(result, "error")
                            and result.error
                            and capture_message_content
                        ):
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_ACTION_ERROR,
                                str(result.error)[:200],
                            )
                        if (
                            hasattr(result, "extracted_content")
                            and result.extracted_content
                            and capture_message_content
                        ):
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_ACTION_EXTRACTED_CONTENT_LENGTH,
                                len(str(result.extracted_content)),
                            )

                    # Enhanced browser state context
                    if step.state:
                        if hasattr(step.state, "url") and step.state.url:
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_AGENT_BROWSE_URL,
                                step.state.url,
                            )
                        if hasattr(step.state, "title") and step.state.title:
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_BROWSER_PAGE_TITLE,
                                step.state.title[:100],
                            )
                        if hasattr(step.state, "tabs") and step.state.tabs:
                            action_span.set_attribute(
                                SemanticConvention.GEN_AI_BROWSER_TABS_COUNT,
                                len(step.state.tabs),
                            )

    except Exception as e:
        logger.debug("Error creating individual action spans: %s", e)


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
