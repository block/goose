"""
OpenAI OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    response_as_dict,
    calculate_tbt,
    get_chat_model_cost,
    get_embed_model_cost,
    get_audio_model_cost,
    get_image_model_cost,
    general_tokens,
    record_completion_metrics,
    record_embedding_metrics,
    record_audio_metrics,
    record_image_metrics,
    common_span_attributes,
)
from openlit.semcov import SemanticConvention


def handle_not_given(value, default=None):
    """
    Handle OpenAI's NotGiven values and None values by converting them to appropriate defaults.
    """
    if hasattr(value, "__class__") and value.__class__.__name__ == "NotGiven":
        return default
    if value is None:
        return default
    return value


def format_content(messages):
    """
    Format the messages into a string for span events.
    Handles both chat completions format and responses API input format.
    """

    if not messages:
        return ""

    # Handle string input (simple case)
    if isinstance(messages, str):
        return messages

    # Handle list of messages
    formatted_messages = []

    for message in messages:
        try:
            role = message.get("role", "user") or message.role
            content = message.get("content", "") or message.content

        except:
            role = "user"
            content = str(messages)

        if isinstance(content, list):
            content_str_list = []
            for item in content:
                # Chat completions format
                if item.get("type") == "text":
                    content_str_list.append(f"text: {item.get('text', '')}")
                elif item.get("type") == "image_url" and not item.get(
                    "image_url", {}
                ).get("url", "").startswith("data:"):
                    content_str_list.append(f"image_url: {item['image_url']['url']}")

                # Responses API format
                elif item.get("type") == "input_text":
                    content_str_list.append(f"text: {item.get('text', '')}")
                elif item.get("type") == "input_image":
                    image_url = item.get("image_url", "")
                    if image_url and not image_url.startswith("data:"):
                        content_str_list.append(f"image_url: {image_url}")

            content_str = ", ".join(content_str_list)
            formatted_messages.append(f"{role}: {content_str}")
        else:
            formatted_messages.append(f"{role}: {content}")

    return "\n".join(formatted_messages)


def process_chat_chunk(scope, chunk):
    """
    Process a chunk of chat response data and update state.
    """

    end_time = time.time()
    scope._timestamps.append(end_time)

    if len(scope._timestamps) == 1:
        scope._ttft = calculate_ttft(scope._timestamps, scope._start_time)

    chunked = response_as_dict(chunk)

    # Extract content from chat completions
    if len(chunked.get("choices", [])) > 0 and "delta" in chunked.get("choices")[0]:
        delta = chunked.get("choices")[0]["delta"]
        content = delta.get("content")
        if content:
            scope._llmresponse += content

        # Handle tool calls in streaming - optimized
        delta_tools = delta.get("tool_calls")
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

    # Extract metadata
    scope._response_id = chunked.get("id") or scope._response_id
    scope._response_model = chunked.get("model") or scope._response_model

    try:
        scope._finish_reason = (
            chunked.get("choices", [])[0].get("finish_reason") or scope._finish_reason
        )
    except (IndexError, AttributeError, TypeError):
        scope._finish_reason = "stop"

    scope._system_fingerprint = (
        chunked.get("system_fingerprint") or scope._system_fingerprint
    )
    scope._service_tier = chunked.get("service_tier") or scope._service_tier


def process_response_chunk(scope, chunk):
    """
    Process a chunk of response API data and update state.
    """

    end_time = time.time()
    scope._timestamps.append(end_time)

    if len(scope._timestamps) == 1:
        scope._ttft = calculate_ttft(scope._timestamps, scope._start_time)

    chunked = response_as_dict(chunk)

    # Extract content from responses API
    if chunked.get("type") == "response.output_text.delta":
        scope._llmresponse += chunked.get("delta", "")

    # Handle tool calls in streaming for responses API
    elif chunked.get("type") == "response.output_item.added":
        # New tool call item added
        if not hasattr(scope, "_response_tools") or scope._response_tools is None:
            scope._response_tools = []

        item = chunked.get("item", {})
        if item.get("type") == "function_call":
            scope._response_tools.append(
                {
                    "id": item.get("id", ""),
                    "call_id": item.get("call_id", ""),
                    "name": item.get("name", ""),
                    "type": item.get("type", "function_call"),
                    "arguments": item.get("arguments", ""),
                    "status": item.get("status", "in_progress"),
                }
            )

    elif chunked.get("type") == "response.function_call_arguments.delta":
        # Tool arguments being streamed
        if hasattr(scope, "_response_tools") and scope._response_tools:
            item_id = chunked.get("item_id", "")
            delta = chunked.get("delta", "")

            # Find the tool by item_id and append arguments
            for tool in scope._response_tools:
                if tool.get("id") == item_id:
                    tool["arguments"] += delta
                    break

    elif chunked.get("type") == "response.function_call_arguments.done":
        # Tool arguments complete
        if hasattr(scope, "_response_tools") and scope._response_tools:
            item_id = chunked.get("item_id", "")
            final_arguments = chunked.get("arguments", "")

            # Update the tool with final arguments
            for tool in scope._response_tools:
                if tool.get("id") == item_id:
                    tool["arguments"] = final_arguments
                    break

    elif chunked.get("type") == "response.output_item.done":
        # Tool call item complete
        if hasattr(scope, "_response_tools") and scope._response_tools:
            item = chunked.get("item", {})
            item_id = item.get("id", "")

            # Update the tool with final status and data
            for tool in scope._response_tools:
                if tool.get("id") == item_id:
                    tool.update(
                        {
                            "call_id": item.get("call_id", tool.get("call_id", "")),
                            "name": item.get("name", tool.get("name", "")),
                            "arguments": item.get(
                                "arguments", tool.get("arguments", "")
                            ),
                            "status": item.get("status", "completed"),
                        }
                    )
                    break

    elif chunked.get("type") == "response.completed":
        response_data = chunked.get("response", {})
        scope._response_id = response_data.get("id") or scope._response_id
        scope._response_model = response_data.get("model") or scope._response_model
        scope._finish_reason = response_data.get("status")

        usage = response_data.get("usage", {})
        scope._input_tokens = usage.get("input_tokens", 0)
        scope._output_tokens = usage.get("output_tokens", 0)

        # Handle reasoning tokens
        output_tokens_details = usage.get("output_tokens_details", {})
        scope._reasoning_tokens = output_tokens_details.get("reasoning_tokens", 0)


def common_response_logic(
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
    Process responses API request and generate Telemetry
    """

    scope._end_time = time.time()
    if len(scope._timestamps) > 1:
        scope._tbt = calculate_tbt(scope._timestamps)

    # For responses API, format input using the same function as chat completions
    input_data = scope._kwargs.get("input", "")
    prompt = format_content(input_data)
    request_model = scope._kwargs.get("model", "gpt-4o")

    # Calculate tokens and cost
    if hasattr(scope, "_input_tokens") and scope._input_tokens:
        input_tokens = scope._input_tokens
        output_tokens = scope._output_tokens
    else:
        input_tokens = general_tokens(prompt)
        output_tokens = general_tokens(scope._llmresponse)

    cost = get_chat_model_cost(request_model, pricing_info, input_tokens, output_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_OPENAI,
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

    # Span Attributes for Request parameters specific to responses API
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        handle_not_given(scope._kwargs.get("temperature"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P,
        handle_not_given(scope._kwargs.get("top_p"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        handle_not_given(scope._kwargs.get("max_output_tokens"), -1),
    )

    # Reasoning parameters
    reasoning = scope._kwargs.get("reasoning", {})
    if reasoning:
        if reasoning.get("effort"):
            scope._span.set_attribute(
                "gen_ai.request.reasoning_effort", reasoning.get("effort")
            )

    # Responses API specific attributes
    if hasattr(scope, "_service_tier"):
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER, scope._service_tier
        )

    # Span Attributes for Response parameters
    if scope._response_id:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id
        )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason]
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, "text")

    # Span Attributes for Tools (responses API structure) - optimized
    if hasattr(scope, "_response_tools") and scope._response_tools:
        tools = (
            scope._response_tools
            if isinstance(scope._response_tools, list)
            else [scope._response_tools]
        )

        names, ids, args = (
            zip(
                *[
                    (
                        t.get("name", ""),
                        str(t.get("call_id", "")),  # Use call_id for responses API
                        str(t.get("arguments", "")),
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

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, input_tokens + output_tokens
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Reasoning tokens
    if hasattr(scope, "_reasoning_tokens") and scope._reasoning_tokens > 0:
        scope._span.set_attribute(
            "gen_ai.usage.reasoning_tokens", scope._reasoning_tokens
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

    # Record metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_OPENAI,
            scope._server_address,
            scope._server_port,
            request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            input_tokens,
            output_tokens,
            cost,
            scope._tbt,
            scope._ttft,
        )


def process_streaming_response_response(
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
    Process streaming responses API response and generate telemetry.
    """

    common_response_logic(
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


def process_response_response(
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
    Process non-streaming responses API response and generate telemetry.
    """

    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span

    # Extract content from responses API structure with reasoning support
    output = response_dict.get("output", [])
    scope._llmresponse = ""
    scope._response_tools = None

    if output:
        # Find the message item in the output array (might not be first if reasoning is present)
        message_item = None
        for item in output:
            if item.get("type") == "message":
                message_item = item
                break
            if item.get("type") == "function_call":
                # Handle tool call
                scope._response_tools = [
                    {
                        "id": item.get("id", ""),
                        "call_id": item.get("call_id", ""),
                        "name": item.get("name", ""),
                        "type": item.get("type", "function_call"),
                        "arguments": item.get("arguments", ""),
                        "status": item.get("status", ""),
                    }
                ]

        # Extract content from message item if found
        if message_item:
            content = message_item.get("content", [])
            if content and len(content) > 0:
                scope._llmresponse = content[0].get("text", "")

    scope._response_id = response_dict.get("id", "")
    scope._response_model = response_dict.get("model", "")

    # Handle token usage including reasoning tokens
    usage = response_dict.get("usage", {})
    scope._input_tokens = usage.get("input_tokens", 0)
    scope._output_tokens = usage.get("output_tokens", 0)

    output_tokens_details = usage.get("output_tokens_details", {})
    scope._reasoning_tokens = output_tokens_details.get("reasoning_tokens", 0)

    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._service_tier = response_dict.get("service_tier", "default")
    scope._finish_reason = response_dict.get("status", "completed")

    common_response_logic(
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

    # Format messages for chat operations
    if hasattr(scope, "_operation_type") and scope._operation_type == "responses":
        # Handle responses API input format using format_content
        input_data = scope._kwargs.get("input", "")
        prompt = format_content(input_data)
    else:
        # Handle standard chat format
        prompt = format_content(scope._kwargs.get("messages", []))

    request_model = scope._kwargs.get("model", "gpt-4o")

    # Calculate tokens and cost
    if hasattr(scope, "_input_tokens") and scope._input_tokens:
        input_tokens = scope._input_tokens
        output_tokens = scope._output_tokens
    else:
        input_tokens = general_tokens(prompt)
        output_tokens = general_tokens(scope._llmresponse)

    cost = get_chat_model_cost(request_model, pricing_info, input_tokens, output_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_OPENAI,
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
        SemanticConvention.GEN_AI_REQUEST_SEED,
        str(handle_not_given(scope._kwargs.get("seed"), "")),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
        handle_not_given(scope._kwargs.get("frequency_penalty"), 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        handle_not_given(scope._kwargs.get("max_tokens"), -1),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
        handle_not_given(scope._kwargs.get("presence_penalty"), 0.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES,
        handle_not_given(scope._kwargs.get("stop"), []),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        handle_not_given(scope._kwargs.get("temperature"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P,
        handle_not_given(scope._kwargs.get("top_p"), 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_USER,
        handle_not_given(scope._kwargs.get("user"), ""),
    )

    # Span Attributes for Response parameters
    if scope._response_id:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id
        )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason]
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OUTPUT_TYPE,
        "text" if isinstance(scope._llmresponse, str) else "json",
    )

    # OpenAI-specific attributes
    if hasattr(scope, "_system_fingerprint") and scope._system_fingerprint:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SYSTEM_FINGERPRINT,
            scope._system_fingerprint,
        )
    if hasattr(scope, "_service_tier") and scope._service_tier:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_SERVICE_TIER, scope._service_tier
        )

    # Span Attributes for Tools - optimized
    if hasattr(scope, "_tools") and scope._tools:
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

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE, input_tokens + output_tokens
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

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

    # Record metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_OPENAI,
            scope._server_address,
            scope._server_port,
            request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            input_tokens,
            output_tokens,
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

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._llmresponse = " ".join(
        (choice.get("message", {}).get("content") or "")
        for choice in response_dict.get("choices", [])
    )
    scope._response_id = response_dict.get("id", "")
    scope._response_model = response_dict.get("model", "")
    scope._input_tokens = response_dict.get("usage", {}).get("prompt_tokens", 0)
    scope._output_tokens = response_dict.get("usage", {}).get("completion_tokens", 0)
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._system_fingerprint = response_dict.get("system_fingerprint", "")
    scope._service_tier = response_dict.get("service_tier", "auto")
    scope._finish_reason = (
        str(response_dict.get("choices", [])[0].get("finish_reason", ""))
        if response_dict.get("choices")
        else ""
    )

    # Handle operation type for responses API
    if kwargs.get("_operation_type") == "responses":
        scope._operation_type = "responses"

    # Handle tool calls
    if kwargs.get("tools"):
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


def common_embedding_logic(
    scope,
    request_model,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Common logic for processing embedding operations.
    """

    # Calculate cost
    cost = get_embed_model_cost(request_model, pricing_info, scope._input_tokens)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
        SemanticConvention.GEN_AI_SYSTEM_OPENAI,
        scope._server_address,
        scope._server_port,
        request_model,
        request_model,
        environment,
        application_name,
        False,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_ENCODING_FORMATS,
        [handle_not_given(scope._kwargs.get("encoding_format"), "float")],
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_USER,
        handle_not_given(scope._kwargs.get("user"), ""),
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
        input_data = scope._kwargs.get("input", "")
        formatted_content = (
            format_content(input_data)
            if isinstance(input_data, (list, dict))
            else str(input_data)
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, formatted_content
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics
    if not disable_metrics:
        record_embedding_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING,
            SemanticConvention.GEN_AI_SYSTEM_OPENAI,
            scope._server_address,
            scope._server_port,
            request_model,
            request_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            scope._input_tokens,
            cost,
        )


def common_image_logic(
    scope,
    request_model,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Common logic for processing image operations.
    """

    # Calculate cost
    cost = get_image_model_cost(
        request_model,
        pricing_info,
        scope._kwargs.get("size", "1024x1024"),
        scope._kwargs.get("quality", "standard"),
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE,
        SemanticConvention.GEN_AI_SYSTEM_OPENAI,
        scope._server_address,
        scope._server_port,
        request_model,
        request_model,
        environment,
        application_name,
        False,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_IMAGE_SIZE,
        handle_not_given(scope._kwargs.get("size"), "1024x1024"),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_IMAGE_QUALITY,
        handle_not_given(scope._kwargs.get("quality"), "standard"),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_USER,
        handle_not_given(scope._kwargs.get("user"), ""),
    )

    # Extract response data
    response_dict = scope._response_dict
    images_data = response_dict.get("data", [])
    response_created = response_dict.get("created")
    response_size = response_dict.get("size")
    response_quality = response_dict.get("quality")
    response_output_format = response_dict.get("output_format")

    # Span Attributes for Response
    if response_created:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID, str(response_created)
        )

    # Process image data and collect URLs/base64 content
    if images_data:
        # Collect image URLs or base64 content
        image_contents = []

        for image in images_data:
            # Collect image content (URL or base64)
            if image.get("url"):
                image_contents.append(image["url"])
            elif image.get("b64_json"):
                # For base64, we typically dont want to store the full content in spans
                # Just indicate its base64 format
                image_contents.append("[base64_image_data]")

        # Set image response data using semantic conventions
        if image_contents:
            scope._span.set_attribute(
                SemanticConvention.GEN_AI_RESPONSE_IMAGE, image_contents
            )

    # Response-level attributes if different from request
    if response_size:
        scope._span.set_attribute("gen_ai.response.image_size", response_size)
    if response_quality:
        scope._span.set_attribute("gen_ai.response.image_quality", response_quality)
    if response_output_format:
        scope._span.set_attribute(
            "gen_ai.response.output_format", response_output_format
        )

    # Span Attributes for Cost
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Span Attributes for Content
    if capture_message_content:
        # Always collect the original prompt
        prompt = scope._kwargs.get("prompt", "")
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)

        # Collect and set revised prompts if available
        if images_data:
            revised_prompts = []
            for image in images_data:
                if image.get("revised_prompt"):
                    revised_prompts.append(image["revised_prompt"])

            # Set revised prompts as span attribute if any were found
            if revised_prompts:
                scope._span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_REVISED_PROMPT, revised_prompts
                )

            # Add revised prompt events for detailed tracking
            for i, image in enumerate(images_data):
                if image.get("revised_prompt"):
                    scope._span.add_event(
                        name=SemanticConvention.GEN_AI_CONTENT_REVISED_PROMPT,
                        attributes={
                            SemanticConvention.GEN_AI_CONTENT_REVISED_PROMPT: image[
                                "revised_prompt"
                            ],
                            "image_index": i,
                        },
                    )

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics
    if not disable_metrics:
        record_image_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE,
            SemanticConvention.GEN_AI_SYSTEM_OPENAI,
            scope._server_address,
            scope._server_port,
            request_model,
            request_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            cost,
        )


def common_audio_logic(
    scope,
    request_model,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Common logic for processing audio operations.
    """

    # Calculate cost
    input_text = scope._kwargs.get("input", "")
    cost = get_audio_model_cost(request_model, pricing_info, input_text)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
        SemanticConvention.GEN_AI_SYSTEM_OPENAI,
        scope._server_address,
        scope._server_port,
        request_model,
        request_model,
        environment,
        application_name,
        False,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_VOICE,
        handle_not_given(scope._kwargs.get("voice"), "alloy"),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_RESPONSE_FORMAT,
        handle_not_given(scope._kwargs.get("response_format"), "mp3"),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_SPEED,
        handle_not_given(scope._kwargs.get("speed"), 1.0),
    )

    # Span Attributes for Cost
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Span Attributes for Content
    if capture_message_content:
        input_text = scope._kwargs.get("input", "")
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, input_text)

    scope._span.set_status(Status(StatusCode.OK))

    # Record metrics
    if not disable_metrics:
        record_audio_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
            SemanticConvention.GEN_AI_SYSTEM_OPENAI,
            scope._server_address,
            scope._server_port,
            request_model,
            request_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            cost,
        )


def process_audio_response(
    response,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    end_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process audio generation response and generate telemetry.
    """

    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = end_time
    scope._span = span
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    common_audio_logic(
        scope,
        request_model,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
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
    Process embedding response and generate telemetry.
    """

    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._input_tokens = response_dict.get("usage", {}).get("prompt_tokens", 0)
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs

    common_embedding_logic(
        scope,
        request_model,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
    )

    return response


def process_image_response(
    response,
    request_model,
    pricing_info,
    server_port,
    server_address,
    environment,
    application_name,
    metrics,
    start_time,
    end_time,
    span,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process image generation response and generate telemetry.
    """

    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = end_time
    scope._span = span
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._response_dict = response_dict

    common_image_logic(
        scope,
        request_model,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
    )

    return response
