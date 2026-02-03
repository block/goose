"""
Module for monitoring Pydantic AI API calls.
"""

from openlit.instrumentation.pydantic_ai.utils import (
    common_agent_run,
    common_agent_create,
    common_graph_execution,
    common_user_prompt_processing,
    common_model_request_processing,
    common_tool_calls_processing,
)


def agent_create(
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
    Generates a telemetry wrapper for GenAI function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_agent_create(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
        )

    return wrapper


def agent_run(
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
    Generates a telemetry wrapper for GenAI function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_agent_run(
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


def graph_execution(
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
    Generates a telemetry wrapper for Pydantic AI graph execution
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_graph_execution(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
        )

    return wrapper


def user_prompt_processing(
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
    Generates a telemetry wrapper for Pydantic AI user prompt processing
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_user_prompt_processing(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
        )

    return wrapper


def model_request_processing(
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
    Generates a telemetry wrapper for Pydantic AI model request processing
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_model_request_processing(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
        )

    return wrapper


def tool_calls_processing(
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
    Generates a telemetry wrapper for Pydantic AI tool calls processing
    """

    def wrapper(wrapped, instance, args, kwargs):
        return common_tool_calls_processing(
            wrapped,
            instance,
            args,
            kwargs,
            tracer,
            version,
            environment,
            application_name,
            capture_message_content,
        )

    return wrapper
