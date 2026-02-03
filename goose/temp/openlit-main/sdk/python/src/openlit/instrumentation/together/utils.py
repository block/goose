"""
Together AI OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    response_as_dict,
    calculate_tbt,
    get_chat_model_cost,
    get_image_model_cost,
    create_metrics_attributes,
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
    if len(chunked.get("choices")) > 0 and (
        "delta" in chunked.get("choices")[0]
        and "content" in chunked.get("choices")[0].get("delta")
    ):
        content = chunked.get("choices")[0].get("delta").get("content")
        if content:
            scope._llmresponse += content

    if chunked.get("usage"):
        scope._response_id = chunked.get("id")
        scope._response_model = chunked.get("model")
        scope._input_tokens = chunked.get("usage").get("prompt_tokens")
        scope._output_tokens = chunked.get("usage").get("completion_tokens")
        scope._finish_reason = str(chunked.get("finish_reason"))
        scope._end_time = time.time()


def common_span_attributes(
    scope,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    is_stream,
    tbt,
    ttft,
    version,
):
    """
    Set common span attributes for both chat and RAG operations.
    """

    scope._span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    scope._span.set_attribute(SemanticConvention.GEN_AI_OPERATION, gen_ai_operation)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SYSTEM, gen_ai_system)
    scope._span.set_attribute(SemanticConvention.SERVER_ADDRESS, server_address)
    scope._span.set_attribute(SemanticConvention.SERVER_PORT, server_port)
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_MODEL, response_model)
    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, is_stream)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, tbt)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, ttft)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)


def record_common_metrics(
    metrics,
    gen_ai_operation,
    gen_ai_system,
    server_address,
    server_port,
    request_model,
    response_model,
    environment,
    application_name,
    start_time,
    end_time,
    input_tokens,
    output_tokens,
    cost,
    tbt=None,
    ttft=None,
):
    """
    Record common metrics for the operation.
    """

    attributes = create_metrics_attributes(
        operation=gen_ai_operation,
        system=gen_ai_system,
        server_address=server_address,
        server_port=server_port,
        request_model=request_model,
        response_model=response_model,
        service_name=application_name,
        deployment_environment=environment,
    )
    metrics["genai_client_operation_duration"].record(end_time - start_time, attributes)
    metrics["genai_requests"].add(1, attributes)
    metrics["genai_prompt_tokens"].add(input_tokens, attributes)
    metrics["genai_completion_tokens"].add(output_tokens, attributes)
    metrics["genai_client_usage_tokens"].record(
        input_tokens + output_tokens, attributes
    )
    metrics["genai_cost"].record(cost, attributes)
    if tbt is not None:
        metrics["genai_server_tbt"].record(tbt, attributes)
    if ttft is not None:
        metrics["genai_server_ttft"].record(ttft, attributes)


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

    prompt = format_content(scope._kwargs.get("messages", ""))
    request_model = scope._kwargs.get("model", "jamba-1.5-mini")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_TOGETHER,
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

    # Span Attributes for Response parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason]
    )
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
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, scope._kwargs.get("stop", [])
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        scope._kwargs.get("temperature", 1.0),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P, scope._kwargs.get("top_p", 1.0)
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id)
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

    # Span Attributes for Tools
    if scope._tools:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_NAME, scope._tools.get("function", "")
        ).get("name", "")
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_CALL_ID, str(scope._tools.get("id", ""))
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_ARGS,
            str(scope._tools.get("function", "").get("arguments", "")),
        )

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._llmresponse
        )

        # To be removed one the change to span_attributes (from span events) is complete
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
        record_common_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            SemanticConvention.GEN_AI_SYSTEM_TOGETHER,
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
    Process chat request and generate Telemetry
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

    scope = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._llmresponse = " ".join(
        (choice.get("message", {}).get("content") or "")
        for choice in response_dict.get("choices", [])
    )
    scope._response_id = response_dict.get("id")
    scope._response_model = response_dict.get("model")
    scope._input_tokens = response_dict.get("usage").get("prompt_tokens")
    scope._output_tokens = response_dict.get("usage").get("completion_tokens")
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._finish_reason = str(response_dict.get("choices")[0].get("finish_reason"))

    if scope._kwargs.get("tools"):
        scope._tools = response_dict.get("choices")[0].get("message").get("tool_calls")
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


def common_image_logic(
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
    Process image generation request and generate Telemetry
    """

    # Find Image format
    if (
        "response_format" in scope._kwargs
        and scope._kwargs["response_format"] == "b64_json"
    ):
        image_format = "b64_json"
    else:
        image_format = "url"

    image_size = (
        str(scope._kwargs.get("width", "1024"))
        + "x"
        + str(scope._kwargs.get("height", "1024"))
    )
    request_model = scope._kwargs.get("model", "dall-e-2")

    # Calculate cost of the operation
    cost = get_image_model_cost(
        request_model,
        pricing_info,
        image_size,
        scope._kwargs.get("quality", "standard"),
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE,
        SemanticConvention.GEN_AI_SYSTEM_TOGETHER,
        scope._server_address,
        scope._server_port,
        request_model,
        scope._response_model,
        environment,
        application_name,
        False,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Image-specific span attributes
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id)
    scope._span.set_attribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, "image")
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IMAGE_SIZE, image_size)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_COST, len(scope._response_data) * cost
    )

    # Content attributes
    if capture_message_content:
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: scope._kwargs.get(
                    "prompt", ""
                ),
            },
        )

        for images_count, item in enumerate(scope._response_data):
            attribute_name = (
                f"{SemanticConvention.GEN_AI_RESPONSE_IMAGE}.{images_count}"
            )
            scope._span.add_event(
                name=attribute_name,
                attributes={
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION: getattr(
                        item, image_format
                    ),
                },
            )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_common_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_IMAGE,
            SemanticConvention.GEN_AI_SYSTEM_TOGETHER,
            scope._server_address,
            scope._server_port,
            request_model,
            scope._response_model,
            environment,
            application_name,
            scope._start_time,
            scope._end_time,
            0,
            0,
            len(scope._response_data) * cost,
        )


def process_image_response(
    response,
    request_model,
    pricing_info,
    server_address,
    server_port,
    environment,
    application_name,
    metrics,
    start_time,
    end_time,
    span,
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Process image generation request and generate Telemetry
    """

    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = end_time
    scope._span = span
    scope._response_id = response.id
    scope._response_model = response.model
    scope._response_data = response.data
    scope._server_address = server_address
    scope._server_port = server_port
    scope._kwargs = kwargs
    scope._tbt = 0
    scope._ttft = end_time - start_time

    common_image_logic(
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
