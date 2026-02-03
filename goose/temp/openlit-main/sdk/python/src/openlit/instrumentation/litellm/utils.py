"""
LiteLLM OpenTelemetry instrumentation utility functions
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
        role = message["role"]
        content = message["content"]

        if isinstance(content, list):
            content_str = ", ".join(
                f"{item['type']}: {item['text'] if 'text' in item else item['image_url']}"
                if "type" in item
                else f"text: {item['text']}"
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

    # Collect message IDs and aggregated response from events
    if len(chunked.get("choices", [])) > 0 and (
        "delta" in chunked.get("choices")[0]
        and "content" in chunked.get("choices")[0].get("delta", {})
    ):
        content = chunked.get("choices")[0].get("delta").get("content")
        if content:
            scope._llmresponse += content

        # Handle tool calls in streaming - optimized
        delta_tools = chunked.get("choices", [{}])[0].get("delta", {}).get("tool_calls")
        if delta_tools:
            scope._tools = scope._tools or []

            for tool in delta_tools:
                idx = tool.get("index", 0)

                # Extend list if needed
                scope._tools.extend([{}] * (idx + 1 - len(scope._tools)))

                if tool.get("id"):  # New tool (id exists)
                    func = tool.get("function", {})
                    scope._tools[idx] = {
                        "id": tool["id"],
                        "function": {
                            "name": func.get("name", ""),
                            "arguments": func.get("arguments", ""),
                        },
                        "type": tool.get("type", "function"),
                    }
                elif (
                    scope._tools[idx] and "function" in tool
                ):  # Append args (id is None)
                    scope._tools[idx]["function"]["arguments"] += tool["function"].get(
                        "arguments", ""
                    )

    if chunked.get("usage"):
        scope._input_tokens = chunked.get("usage").get("prompt_tokens", 0)
        scope._output_tokens = chunked.get("usage").get("completion_tokens", 0)
        scope._response_id = chunked.get("id")
        scope._response_model = chunked.get("model")
        finish_reason = chunked.get("choices", [{}])[0].get("finish_reason")
        # Only update finish_reason if it's not None (preserve previous valid value)
        if finish_reason is not None:
            scope._finish_reason = finish_reason
        scope._response_service_tier = str(chunked.get("system_fingerprint", ""))
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
    request_model = scope._kwargs.get("model", "openai/gpt-4o")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_LITELLM,
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

    # Helper function to handle None values with proper defaults
    def safe_get(value, default):
        return default if value is None else value

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_SEED, safe_get(scope._kwargs.get("seed"), "")
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
        safe_get(scope._kwargs.get("frequency_penalty"), 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        safe_get(scope._kwargs.get("max_tokens"), -1),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
        safe_get(scope._kwargs.get("presence_penalty"), 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, scope._kwargs.get("stop", [])
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        safe_get(scope._kwargs.get("temperature"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P,
        safe_get(scope._kwargs.get("top_p"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_USER, safe_get(scope._kwargs.get("user"), "")
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER,
        safe_get(scope._kwargs.get("service_tier"), "auto"),
    )

    # Span Attributes for Response parameters
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason or ""]
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_SERVICE_TIER, scope._response_service_tier
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_SYSTEM_FINGERPRINT,
        scope._response_service_tier,
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
            SemanticConvention.GEN_AI_SYSTEM_LITELLM,
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
    scope._llmresponse = " ".join(
        (choice.get("message", {}).get("content") or "")
        for choice in response_dict.get("choices", [])
    )
    scope._input_tokens = response_dict.get("usage", {}).get("prompt_tokens", 0)
    scope._output_tokens = response_dict.get("usage", {}).get("completion_tokens", 0)
    scope._response_id = response_dict.get("id")
    scope._response_model = response_dict.get("model")
    scope._finish_reason = str(
        response_dict.get("choices", [])[0].get("finish_reason", "")
    )
    scope._response_service_tier = str(response_dict.get("system_fingerprint", ""))
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    # Handle tool calls
    if scope._kwargs.get("tools"):
        scope._tools = (
            response_dict.get("choices", [{}])[0].get("message", {}).get("tool_calls")
        )
    else:
        scope._tools = None

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
    scope._input_tokens = response_dict.get("usage", {}).get("prompt_tokens", 0)
    scope._response_model = response_dict.get("model")
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    # Calculate cost of the operation
    cost = get_embed_model_cost(request_model, pricing_info, scope._input_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
        SemanticConvention.GEN_AI_SYSTEM_LITELLM,
        scope._server_address,
        scope._server_port,
        request_model,
        scope._response_model,
        environment,
        application_name,
        False,
        0,
        scope._end_time - scope._start_time,
        version,
    )

    # Helper function to handle None values with proper defaults
    def safe_get(value, default):
        return default if value is None else value

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_ENCODING_FORMATS,
        [scope._kwargs.get("encoding_format", "float")],
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_USER, safe_get(scope._kwargs.get("user"), "")
    )

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, scope._input_tokens
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT,
            str(scope._kwargs.get("input", "")),
        )

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: str(
                    scope._kwargs.get("input", "")
                ),
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_embedding_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
            SemanticConvention.GEN_AI_SYSTEM_LITELLM,
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

    return response
