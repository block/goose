"""
AWS Bedrock OpenTelemetry instrumentation utility functions
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
    handle_exception,
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
            # Handle Bedrock object format
            role = getattr(message, "role", "user")
            content = getattr(message, "content", "")

        if isinstance(content, list):
            # Handle structured content (e.g., text + images)
            text_parts = []
            for part in content:
                if isinstance(part, dict):
                    # Bedrock format: {"text": "content"} or generic format: {"type": "text", "text": "content"}
                    if "text" in part:
                        text_parts.append(part.get("text", ""))
                    elif part.get("type") == "text":
                        text_parts.append(part.get("text", ""))
            content = " ".join(text_parts)
        elif not isinstance(content, str):
            content = str(content)

        formatted_messages.append(f"{role}: {content}")

    return "\n".join(formatted_messages)


def process_chunk(self, chunk):
    """
    Process a chunk of response data and update state.
    """

    end_time = time.time()
    # Record the timestamp for the current chunk
    self._timestamps.append(end_time)

    if len(self._timestamps) == 1:
        # Calculate time to first chunk
        self._ttft = calculate_ttft(self._timestamps, self._start_time)

    chunked = response_as_dict(chunk)

    # Handle Bedrock messageStart event
    if "messageStart" in chunked:
        message_start = chunked.get("messageStart", {})
        self._response_role = message_start.get("role", "assistant")

    # Handle Bedrock contentBlockDelta event
    if "contentBlockDelta" in chunked:
        content_delta = chunked.get("contentBlockDelta", {})
        delta = content_delta.get("delta", {})
        if "text" in delta:
            self._llmresponse += delta.get("text", "")

    # Handle Bedrock messageStop event
    if "messageStop" in chunked:
        message_stop = chunked.get("messageStop", {})
        self._finish_reason = message_stop.get("stopReason", "")

    # Handle Bedrock metadata event (final event with usage info)
    if "metadata" in chunked:
        metadata = chunked.get("metadata", {})
        usage = metadata.get("usage", {})
        self._input_tokens = usage.get("inputTokens", 0)
        self._output_tokens = usage.get("outputTokens", 0)
        self._end_time = end_time


def common_chat_logic(
    scope,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
    llm_config,
    is_stream,
):
    """
    Process chat request and generate Telemetry
    """

    scope._end_time = time.time()
    if len(scope._timestamps) > 1:
        scope._tbt = calculate_tbt(scope._timestamps)

    formatted_messages = format_content(scope._kwargs.get("messages", []))
    request_model = scope._kwargs.get("modelId", "amazon.titan-text-express-v1")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
        SemanticConvention.GEN_AI_SYSTEM_AWS_BEDROCK,
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

    # Bedrock-specific attributes from llm_config
    bedrock_attributes = [
        (SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, "frequencyPenalty"),
        (SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, "maxTokens"),
        (SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY, "presencePenalty"),
        (SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, "stopSequences"),
        (SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, "temperature"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_P, "topP"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_K, "topK"),
    ]

    # Set each bedrock-specific attribute if the corresponding value exists and is not None
    for attribute, key in bedrock_attributes:
        value = llm_config.get(key)
        if value is not None:
            scope._span.set_attribute(attribute, value)

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
            SemanticConvention.GEN_AI_SYSTEM_AWS_BEDROCK,
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
    llm_config=None,
):
    """
    Process streaming chat response and generate telemetry.
    """

    try:
        if llm_config is None:
            llm_config = {}

        common_chat_logic(
            scope,
            pricing_info,
            environment,
            application_name,
            metrics,
            capture_message_content,
            disable_metrics,
            version,
            llm_config,
            is_stream=True,
        )
    except Exception as e:
        handle_exception(scope._span, e)
        raise


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
    llm_config=None,
    **kwargs,
):
    """
    Process non-streaming chat response and generate telemetry.
    """

    try:
        if llm_config is None:
            llm_config = {}

        scope = type("GenericScope", (), {})()
        response_dict = response_as_dict(response)

        scope._start_time = start_time
        scope._end_time = time.time()
        scope._span = span
        scope._llmresponse = (
            response_dict.get("output", {})
            .get("message", {})
            .get("content", [{}])[0]
            .get("text", "")
        )
        scope._response_role = (
            response_dict.get("output", {}).get("message", {}).get("role", "assistant")
        )
        scope._input_tokens = response_dict.get("usage", {}).get("inputTokens", 0)
        scope._output_tokens = response_dict.get("usage", {}).get("outputTokens", 0)
        scope._response_model = request_model
        scope._finish_reason = response_dict.get("stopReason", "")
        scope._response_id = response_dict.get("RequestId", "")
        scope._timestamps = []
        scope._ttft, scope._tbt = scope._end_time - scope._start_time, 0
        scope._server_address, scope._server_port = server_address, server_port
        scope._kwargs = kwargs

        common_chat_logic(
            scope,
            pricing_info,
            environment,
            application_name,
            metrics,
            capture_message_content,
            disable_metrics,
            version,
            llm_config,
            is_stream=False,
        )

        return response
    except Exception as e:
        handle_exception(span, e)
        raise
