"""
Google AI Studio OpenTelemetry instrumentation utility functions
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
    create_metrics_attributes,
)
from openlit.semcov import SemanticConvention


def format_content(messages):
    """
    Process a list of messages to extract content.
    """

    formatted_messages = []
    prompt = ""

    if isinstance(messages, list):
        try:
            for content in messages:
                role = content.role if content.role is not None else "user"
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
                        content_str.append(
                            f"function_response: {part.function_response}"
                        )
                    if part.inline_data:
                        content_str.append(f"inline_data: {part.inline_data}")

                formatted_messages.append(f"{role}: {', '.join(content_str)}")

            prompt = "\n".join(formatted_messages)

        except:
            prompt = str(messages)

    else:
        prompt = messages

    return prompt


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

    self._response_id = str(chunked.get("response_id"))
    self._input_tokens = chunked.get("usage_metadata").get("prompt_token_count")
    self._response_model = chunked.get("model_version")

    if chunk.text:
        self._llmresponse += str(chunk.text)

    self._output_tokens = chunked.get("usage_metadata").get("candidates_token_count")
    self._reasoning_tokens = (
        chunked.get("usage_metadata").get("thoughts_token_count") or 0
    )
    self._finish_reason = str(chunked.get("candidates")[0].get("finish_reason"))

    try:
        self._tools = (
            chunked.get("candidates", [])[0]
            .get("content", {})
            .get("parts", [])[0]
            .get("function_call", "")
        )
    except:
        self._tools = None


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

    prompt = format_content(scope._kwargs.get("contents", ""))
    request_model = scope._kwargs.get("model", "gemini-2.0-flash")

    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Set Span attributes (OTel Semconv)
    scope._span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OPERATION,
        SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_GEMINI
    )
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    scope._span.set_attribute(SemanticConvention.SERVER_PORT, scope._server_port)
    scope._span.set_attribute(SemanticConvention.SERVER_ADDRESS, scope._server_address)

    inference_config = scope._kwargs.get("config", {})

    attributes = [
        (SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY, "frequency_penalty"),
        (SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS, "max_tokens"),
        (SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY, "presence_penalty"),
        (SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, "stop_sequences"),
        (SemanticConvention.GEN_AI_REQUEST_TEMPERATURE, "temperature"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_P, "top_p"),
        (SemanticConvention.GEN_AI_REQUEST_TOP_K, "top_k"),
    ]

    # Set each attribute if the corresponding value exists and is not None
    for attribute, key in attributes:
        # Use getattr to get the attribute value from the object
        value = getattr(inference_config, key, None)
        if value is not None:
            scope._span.set_attribute(attribute, value)

    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, [scope._finish_reason]
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_MODEL, scope._response_model
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, scope._output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_REASONING_TOKENS, scope._reasoning_tokens
    )

    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OUTPUT_TYPE,
        "text" if isinstance(scope._llmresponse, str) else "json",
    )

    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, is_stream)
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, scope._tbt)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, scope._ttft)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_TOKEN_USAGE,
        scope._input_tokens + scope._output_tokens + scope._reasoning_tokens,
    )

    if scope._tools:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_NAME, scope._tools.get("name", "")
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_CALL_ID, str(scope._tools.get("id", ""))
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_TOOL_ARGS, str(scope._tools.get("args", ""))
        )

    # To be removed one the change to span_attributes (from span events) is complete
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._llmresponse
        )
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

    if not disable_metrics:
        metrics_attributes = create_metrics_attributes(
            service_name=application_name,
            deployment_environment=environment,
            operation=SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            system=SemanticConvention.GEN_AI_SYSTEM_GEMINI,
            request_model=request_model,
            server_address=scope._server_address,
            server_port=scope._server_port,
            response_model=scope._response_model,
        )

        metrics["genai_client_operation_duration"].record(
            scope._end_time - scope._start_time, metrics_attributes
        )
        metrics["genai_server_tbt"].record(scope._tbt, metrics_attributes)
        metrics["genai_server_ttft"].record(scope._ttft, metrics_attributes)
        metrics["genai_requests"].add(1, metrics_attributes)
        metrics["genai_completion_tokens"].add(scope._output_tokens, metrics_attributes)
        metrics["genai_prompt_tokens"].add(scope._input_tokens, metrics_attributes)
        metrics["genai_reasoning_tokens"].add(
            scope._reasoning_tokens, metrics_attributes
        )
        metrics["genai_cost"].record(cost, metrics_attributes)
        metrics["genai_client_usage_tokens"].record(
            scope._input_tokens + scope._output_tokens + scope._reasoning_tokens,
            metrics_attributes,
        )


def process_streaming_chat_response(
    self,
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
        self,
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
    instance,
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
    args,
    kwargs,
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
):
    """
    Process chat request and generate Telemetry
    """

    self = type("GenericScope", (), {})()
    response_dict = response_as_dict(response)

    self._start_time = start_time
    self._end_time = time.time()
    self._span = span
    self._llmresponse = str(response.text)
    self._input_tokens = response_dict.get("usage_metadata").get("prompt_token_count")
    self._output_tokens = response_dict.get("usage_metadata").get(
        "candidates_token_count"
    )
    self._reasoning_tokens = (
        response_dict.get("usage_metadata").get("thoughts_token_count") or 0
    )
    self._response_model = response_dict.get("model_version")
    self._timestamps = []
    self._ttft, self._tbt = self._end_time - self._start_time, 0
    self._server_address, self._server_port = server_address, server_port
    self._kwargs = kwargs
    self._finish_reason = str(response_dict.get("candidates")[0].get("finish_reason"))

    try:
        self._tools = (
            response_dict.get("candidates", [])[0]
            .get("content", {})
            .get("parts", [])[0]
            .get("function_call", "")
        )
    except:
        self._tools = None

    common_chat_logic(
        self,
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
