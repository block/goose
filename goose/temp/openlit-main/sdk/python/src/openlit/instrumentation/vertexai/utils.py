"""
VertexAI OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    calculate_tbt,
    get_chat_model_cost,
    record_completion_metrics,
    common_span_attributes,
)
from openlit.semcov import SemanticConvention


def format_content(contents):
    """
    Format the VertexAI contents into a string for span events.
    """

    if not contents:
        return ""

    formatted_messages = []
    for content in contents:
        role = content.role
        parts = content.parts
        content_str = []

        for part in parts:
            # Collect relevant fields and handle each type of data that Part could contain
            if part.text:
                content_str.append(f"text: {part.text}")
            if part.video_metadata:
                content_str.append(f"video_metadata: {part.video_metadata}")
            if part.thought:
                content_str.append(f"thought: {part.thought}")
            if part.code_execution_result:
                content_str.append(
                    f"code_execution_result: {part.code_execution_result}"
                )
            if part.executable_code:
                content_str.append(f"executable_code: {part.executable_code}")
            if part.file_data:
                content_str.append(f"file_data: {part.file_data}")
            if part.function_call:
                content_str.append(f"function_call: {part.function_call}")
            if part.function_response:
                content_str.append(f"function_response: {part.function_response}")
            if part.inline_data:
                content_str.append(f"inline_data: {part.inline_data}")

        formatted_messages.append(f"{role}: {', '.join(content_str)}")

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

    # Aggregate response content
    scope._llmresponse += str(chunk.text)
    scope._input_tokens = chunk.usage_metadata.prompt_token_count
    scope._output_tokens = chunk.usage_metadata.candidates_token_count


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

    # Format content using VertexAI-specific logic
    contents = scope._kwargs.get("contents", [])
    formatted_messages = format_content(contents)
    prompt = formatted_messages or str(scope._args[0][0])

    cost = get_chat_model_cost(
        scope._request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_VERTEXAI,
        scope._server_address,
        scope._server_port,
        scope._request_model,
        scope._request_model,
        environment,
        application_name,
        is_stream,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Request parameters (VertexAI-specific)
    inference_config = scope._kwargs.get("generation_config", {})

    # List of attributes and their config keys
    attributes = [
        (SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, "frequency_penalty"),
        (SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, "max_output_tokens"),
        (SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY, "presence_penalty"),
        (SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, "stop_sequences"),
        (SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, "temperature"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_P, "top_p"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_K, "top_k"),
    ]

    # Set each attribute if the corresponding value exists and is not None
    for attribute, key in attributes:
        value = inference_config.get(key)
        if value is not None:
            scope._span.set_attribute(attribute, value)

    # Span Attributes for Response parameters
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
            SemanticConvention.GEN_AI_SYSTEM_VERTEXAI,
            scope._server_address,
            scope._server_port,
            scope._request_model,
            scope._request_model,
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

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._llmresponse = response.text
    scope._input_tokens = response.usage_metadata.prompt_token_count
    scope._output_tokens = response.usage_metadata.candidates_token_count
    scope._timestamps = []
    scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
    scope._server_address, scope._server_port = server_address, server_port
    scope._request_model = request_model
    scope._kwargs = kwargs
    scope._args = [kwargs.get("contents", [])]

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


def extract_vertexai_details(instance):
    """
    Extract VertexAI-specific details like location and model name.
    """
    try:
        location = instance._model._location
        request_model = "/".join(instance._model._model_name.split("/")[3:])
    except:
        location = instance._location
        request_model = "/".join(instance._model_name.split("/")[3:])

    server_address = location + "-aiplatform.googleapis.com"
    server_port = 443

    return server_address, server_port, request_model
