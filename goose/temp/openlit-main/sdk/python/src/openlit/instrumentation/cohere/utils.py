"""
Cohere OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    response_as_dict,
    calculate_tbt,
    get_chat_model_cost,
    get_embed_model_cost,
    common_span_attributes,
    record_completion_metrics,
    record_embedding_metrics,
)
from openlit.semcov import SemanticConvention


def format_content(messages):
    """
    Process a list of messages to extract content.
    """

    formatted_messages = []
    for message in messages:
        # Handle both dictionary and object formats
        if isinstance(message, dict):
            role = message.get("role", "user")
            content = message.get("content", "")
        else:
            # Handle Cohere object format (e.g., cohere.UserChatMessageV2)
            role = getattr(message, "role", "user")
            content = getattr(message, "content", "")

        if isinstance(content, list):
            content_str = ", ".join(
                f"{item['type']}: {item['text'] if 'text' in item else item.get('image_url', '')}"
                if "type" in item
                else f"text: {item.get('text', '')}"
                for item in content
            )
            formatted_messages.append(f"{role}: {content_str}")
        else:
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

    # Handle different chunk types for Cohere streaming
    if chunked.get("type") == "message-start":
        scope._response_id = chunked.get("id")

    if chunked.get("type") == "content-delta":
        content = (
            chunked.get("delta", {}).get("message", {}).get("content", {}).get("text")
        )
        if content:
            scope._llmresponse += content

    # Handle tool plan deltas
    if chunked.get("type") == "tool-plan-delta":
        tool_plan_text = (
            chunked.get("delta", {}).get("message", {}).get("tool_plan", "")
        )
        if tool_plan_text:
            if not hasattr(scope, "_tool_plan"):
                scope._tool_plan = ""
            scope._tool_plan += tool_plan_text

    # Handle tool call start
    if chunked.get("type") == "tool-call-start":
        if not hasattr(scope, "_tools") or scope._tools is None:
            scope._tools = []

        index = chunked.get("index", 0)
        tool_call = chunked.get("delta", {}).get("message", {}).get("tool_calls", {})

        # Extend list if needed
        scope._tools.extend([{}] * (index + 1 - len(scope._tools)))

        # Initialize tool call
        scope._tools[index] = {
            "id": tool_call.get("id", ""),
            "type": tool_call.get("type", "function"),
            "function": {
                "name": tool_call.get("function", {}).get("name", ""),
                "arguments": "",
            },
        }

    # Handle tool call deltas (arguments)
    if chunked.get("type") == "tool-call-delta":
        if hasattr(scope, "_tools") and scope._tools:
            index = chunked.get("index", 0)
            if index < len(scope._tools):
                arguments = (
                    chunked.get("delta", {})
                    .get("message", {})
                    .get("tool_calls", {})
                    .get("function", {})
                    .get("arguments", "")
                )
                if arguments:
                    scope._tools[index]["function"]["arguments"] += arguments

    if chunked.get("type") == "message-end":
        delta = chunked.get("delta", {})
        scope._finish_reason = delta.get("finish_reason", "")
        usage = delta.get("usage", {}).get("billed_units", {})
        scope._input_tokens = usage.get("input_tokens", 0)
        scope._output_tokens = usage.get("output_tokens", 0)
        scope._end_time = time.time()


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

    if len(scope._timestamps) > 1:
        scope._tbt = calculate_tbt(scope._timestamps)

    prompt = format_content(scope._kwargs.get("messages", []))
    request_model = scope._kwargs.get("model", "command-r-plus-08-2024")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_COHERE,
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
        SemanticConvention.GEN_AI_REQUEST_SEED, scope._kwargs.get("seed", "")
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
        scope._kwargs.get("frequency_penalty", 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        scope._kwargs.get("max_tokens", -1),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
        scope._kwargs.get("presence_penalty", 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES,
        scope._kwargs.get("stop_sequences", []),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        scope._kwargs.get("temperature", 0.3),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_K, scope._kwargs.get("k", 1.0)
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P, scope._kwargs.get("p", 1.0)
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

    # Span Attributes for Tools - optimized
    if scope._tools:
        tools = scope._tools if isinstance(scope._tools, list) else [scope._tools]

        names, ids, args = (
            zip(
                *[
                    (
                        t.get("function", {}).get("name", ""),
                        str(t.get("id", "")),
                        str(t.get("function", {}).get("arguments", "")),
                    )
                    for t in tools
                    if isinstance(t, dict) and t
                ]
            )
            if tools
            else ([], [], [])
        )

        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_NAME, ", ".join(filter(None, names))
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_CALL_ID, ", ".join(filter(None, ids))
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_ARGS, ", ".join(filter(None, args))
        )

    # Span Attributes for Tool Plan (Cohere specific)
    if hasattr(scope, "_tool_plan") and scope._tool_plan:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_REASONING, scope._tool_plan
        )

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._llmresponse
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: prompt,
            },
        )
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_COMPLETION_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: scope._llmresponse,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_COHERE,
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
    Process streaming chat request and generate Telemetry
    """

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
    Process chat request and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    # Extract response content - handle both text and tool responses
    message = response_dict.get("message", {})
    content_list = message.get("content", [])
    if content_list and isinstance(content_list, list) and len(content_list) > 0:
        scope._llmresponse = content_list[0].get("text", "")
    else:
        scope._llmresponse = ""
    scope._response_id = response_dict.get("id")
    scope._response_model = request_model
    scope._input_tokens = (
        response_dict.get("usage", {}).get("billed_units", {}).get("input_tokens", 0)
    )
    scope._output_tokens = (
        response_dict.get("usage", {}).get("billed_units", {}).get("output_tokens", 0)
    )
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._finish_reason = response_dict.get("finish_reason", "")

    # Handle tool calls
    if scope._kwargs.get("tools"):
        scope._tools = response_dict.get("message", {}).get("tool_calls")
        # Handle tool plan if present
        scope._tool_plan = response_dict.get("message", {}).get("tool_plan", "")
    else:
        scope._tools = None
        scope._tool_plan = ""

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


def common_embedding_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Process embedding request and generate Telemetry
    """

    request_model = scope._kwargs.get("model", "embed-english-v3.0")
    inputs = scope._kwargs.get("texts", [])

    cost = get_embed_model_cost(request_model, pricing_info, scope._input_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
        SemanticConvention.GEN_AI_SYSTEM_COHERE,
        scope._server_address,
        scope._server_port,
        request_model,
        scope._response_model,
        environment,
        application_name,
        False,
        0,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_ENCODING_FORMATS,
        scope._kwargs.get("embedding_types", ["float"]),
    )

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, scope._input_tokens
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OUTPUT_TYPE, scope._response_type
    )

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, str(inputs))

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: str(inputs),
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_embedding_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
            SemanticConvention.GEN_AI_SYSTEM_COHERE,
            scope._server_address,
            scope._server_port,
            request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._input_tokens,
            cost,
        )


def process_embedding_response(
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
    Process embedding request and generate Telemetry
    """

    # Create scope object
    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._input_tokens = (
        response_dict.get("meta", {}).get("billed_units", {}).get("input_tokens", 0)
    )
    scope._response_model = request_model
    scope._response_type = response_dict.get("response_type", "")
    scope._ttft = scope._end_time - scope._start_time
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    common_embedding_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
    )

    return response
