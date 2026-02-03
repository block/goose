"""
Anthropic OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    response_as_dict,
    calculate_tbt,
    get_chat_model_cost,
    record_completion_metrics,
    common_span_attributes,
)
from openlit.semcov import SemanticConvention


def format_content(messages):
    """
    Format the messages into a string for span events.
    """

    if not messages:
        return ""

    formatted_messages = []
    for message in messages:
        if isinstance(message, dict):
            role = message.get("role", "user")
            content = message.get("content", "")
        else:
            # Handle Anthropic object format
            role = getattr(message, "role", "user")
            content = getattr(message, "content", "")

        if isinstance(content, list):
            # Handle structured content (e.g., text + images)
            text_parts = []
            for part in content:
                if isinstance(part, dict) and part.get("type") == "text":
                    text_parts.append(part.get("text", ""))
            content = " ".join(text_parts)
        elif not isinstance(content, str):
            content = str(content)

        formatted_messages.append(f"{role}: {content}")

    return "\n".join(formatted_messages)


def process_chunk(scope, chunk):
    """
    Process a chunk of response data and update state.
    """

    end_time = time.time()
    # Record the timestamp for the current chunk
    scope._timestamps.append(end_time)

    if len(scope._timestamps) == 1:
        # Calculate time to first chunk
        scope._ttft = calculate_ttft(scope._timestamps, scope._start_time)

    chunked = response_as_dict(chunk)

    # Collect message IDs and input token from events
    if chunked.get("type") == "message_start":
        scope._response_id = chunked.get("message").get("id")
        scope._input_tokens = chunked.get("message").get("usage").get("input_tokens")
        scope._response_model = chunked.get("message").get("model")
        scope._response_role = chunked.get("message").get("role")

    # Collect message IDs and aggregated response from events
    if chunked.get("type") == "content_block_delta":
        if chunked.get("delta").get("text"):
            scope._llmresponse += chunked.get("delta").get("text")
        elif chunked.get("delta").get("partial_json"):
            scope._tool_arguments += chunked.get("delta").get("partial_json")

    if chunked.get("type") == "content_block_start":
        if chunked.get("content_block").get("id"):
            scope._tool_id = chunked.get("content_block").get("id")
        if chunked.get("content_block").get("name"):
            scope._tool_name = chunked.get("content_block").get("name")

    # Collect output tokens and stop reason from events
    if chunked.get("type") == "message_delta":
        scope._output_tokens = chunked.get("usage").get("output_tokens")
        scope._finish_reason = chunked.get("delta").get("stop_reason")


def common_chat_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    is_stream,
):
    """
    Process chat request and generate Telemetry
    """

    scope._end_time = time.time()
    if len(scope._timestamps) > 1:
        scope._tbt = calculate_tbt(scope._timestamps)

    formatted_messages = format_content(scope._kwargs.get("messages", []))
    request_model = scope._kwargs.get("model", "claude-3-5-sonnet-latest")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_ANTHROPIC,
        scope._server_address,
        scope._server_port,
        request_model,
        scope._response_model,
        environment,
        application_name,
        is_stream,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        scope._kwargs.get("max_tokens", -1),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES,
        scope._kwargs.get("stop_sequences", []),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        scope._kwargs.get("temperature", 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_K, scope._kwargs.get("top_k", 1.0)
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P, scope._kwargs.get("top_p", 1.0)
    )

    # Span Attributes for Response parameters
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason]
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OUTPUT_TYPE,
        "text" if isinstance(scope._llmresponse, str) else "json",
    )

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, scope._output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
        scope._input_tokens + scope._output_tokens,
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Handle tool calls if present
    if scope._tool_calls:
        # Optimized tool handling - extract name, id, and arguments
        tool_name = scope._tool_calls.get("name", "")
        tool_id = scope._tool_calls.get("id", "")
        tool_args = scope._tool_calls.get("input", "")

        scope._span.set_attribute(SemanticConvention.GEN_AI_TOOL_NAME, tool_name)
        scope._span.set_attribute(SemanticConvention.GEN_AI_TOOL_CALL_ID, tool_id)
        scope._span.set_attribute(SemanticConvention.GEN_AI_TOOL_ARGS, str(tool_args))

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, formatted_messages
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._llmresponse
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: formatted_messages,
            },
        )
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: scope._llmresponse,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_ANTHROPIC,
            scope._server_address,
            scope._server_port,
            request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._input_tokens,
            scope._output_tokens,
            cost,
            scope._tbt,
            scope._ttft,
        )


def process_streaming_chat_response(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content=False,
    disable_metrics=False,
    version="",
):
    """
    Process streaming chat response and generate telemetry.
    """

    if scope._tool_id != "":
        scope._tool_calls = {
            "id": scope._tool_id,
            "name": scope._tool_name,
            "input": scope._tool_arguments,
        }

    common_chat_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        is_stream=True,
    )


def process_chat_response(
    response,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process non-streaming chat response and generate telemetry.
    """

    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    # pylint: disable = no-member
    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._llmresponse = response_dict.get("content", [{}])[0].get("text", "")
    scope._response_role = response_dict.get("role", "assistant")
    scope._input_tokens = response_dict.get("usage").get("input_tokens")
    scope._output_tokens = response_dict.get("usage").get("output_tokens")
    scope._response_model = response_dict.get("model", "")
    scope._finish_reason = response_dict.get("stop_reason", "")
    scope._response_id = response_dict.get("id", "")
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    # Handle tool calls if present
    content_blocks = response_dict.get("content", [])
    scope._tool_calls = None
    for block in content_blocks:
        if block.get("type") == "tool_use":
            scope._tool_calls = {
                "id": block.get("id", ""),
                "name": block.get("name", ""),
                "input": block.get("input", ""),
            }
            break

    common_chat_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        is_stream=False,
    )

    return response
