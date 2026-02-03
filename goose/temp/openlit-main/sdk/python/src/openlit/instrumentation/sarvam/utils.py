"""
Sarvam AI OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    calculate_ttft,
    response_as_dict,
    calculate_tbt,
    get_chat_model_cost,
    record_completion_metrics,
    common_framework_span_attributes,
    get_audio_model_cost,
    record_audio_metrics,
)
from openlit.semcov import SemanticConvention


def format_content(messages):
    """
    Formats the given messages into a single string.

    Args:
        messages: A list of message dictionaries containing 'role' and 'content' keys.

    Returns:
        A formatted string representing the messages.
    """
    formatted_messages = []
    for message in messages:
        role = message.get("role", "unknown")
        content = message.get("content", "")
        formatted_messages.append(f"{role}: {content}")
    return "\n".join(formatted_messages)


def process_chunk(scope, chunk):
    """
    Processes a streaming chunk from Sarvam AI API response.

    Args:
        scope: The scope object containing span and response data.
        chunk: Individual chunk from the streaming response.
    """
    current_time = time.time()

    # Calculate TTFT for the first chunk
    if len(scope._timestamps) == 0:
        scope._ttft = calculate_ttft(scope._start_time, current_time)

    scope._timestamps.append(current_time)

    # Extract and accumulate response data from chunk
    if hasattr(chunk, "choices") and len(chunk.choices) > 0:
        choice = chunk.choices[0]

        if (
            hasattr(choice, "delta")
            and hasattr(choice.delta, "content")
            and choice.delta.content
        ):
            scope._llmresponse += choice.delta.content

        if hasattr(choice, "finish_reason") and choice.finish_reason:
            scope._finish_reason = choice.finish_reason

    # Extract usage information if available
    if hasattr(chunk, "usage") and chunk.usage:
        if hasattr(chunk.usage, "prompt_tokens"):
            scope._input_tokens = chunk.usage.prompt_tokens
        if hasattr(chunk.usage, "completion_tokens"):
            scope._output_tokens = chunk.usage.completion_tokens

    # Extract response ID and model if available
    if hasattr(chunk, "id"):
        scope._response_id = chunk.id
    if hasattr(chunk, "model"):
        scope._response_model = chunk.model


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
    Common logic for processing chat responses (both streaming and non-streaming).

    Args:
        scope: The scope object containing span and response data.
        pricing_info: Information about model pricing.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        is_stream: Whether this is a streaming response.
    """
    scope._end_time = time.time()

    # Calculate time between tokens for streaming responses
    if len(scope._timestamps) > 1:
        scope._tbt = calculate_tbt(scope._timestamps)

    # Format messages for capture
    formatted_messages = format_content(scope._kwargs.get("messages", []))
    request_model = scope._kwargs.get("model", "sarvam-m")

    # Calculate cost
    cost = get_chat_model_cost(
        request_model, pricing_info, scope._input_tokens, scope._output_tokens
    )

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=scope._server_address,
        server_port=scope._server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
    )

    # Additional GenAI-specific attributes
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_MODEL, scope._response_model
    )

    if is_stream:
        scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, True)

    if scope._tbt > 0:
        scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, scope._tbt)

    if scope._ttft > 0:
        scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, scope._ttft)

        # Span Attributes for Request parameters (using Sarvam API defaults)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_MAX_TOKENS,
        scope._kwargs.get("max_tokens", -1),
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_STOP_SEQUENCES, scope._kwargs.get("stop", [])
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TEMPERATURE,
        scope._kwargs.get("temperature", 0.2),  # API default
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TOP_P,
        scope._kwargs.get("top_p", 1.0),  # API default
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_FREQUENCY_PENALTY,
        scope._kwargs.get("frequency_penalty", 0.0),  # API default
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_PRESENCE_PENALTY,
        scope._kwargs.get("presence_penalty", 0.0),  # API default
    )

    # Additional chat parameters with API defaults
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_N,
        scope._kwargs.get("n", 1),  # API default
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_IS_STREAM,
        scope._kwargs.get("stream", False),  # API default
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_WIKI_GROUNDING,
        scope._kwargs.get("wiki_grounding", False),  # API default
    )

    if scope._kwargs.get("seed") is not None:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_SEED, scope._kwargs.get("seed")
        )

    # Sarvam-specific parameters
    if scope._kwargs.get("reasoning_effort"):
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_REASONING_EFFORT,
            scope._kwargs.get("reasoning_effort"),
        )

    # Span Attributes for Response
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response_id)
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_MODEL, scope._response_model
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_FINISH_REASON, scope._finish_reason
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_INPUT_TOKENS, scope._input_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_OUTPUT_TOKENS, scope._output_tokens
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_USAGE_TOTAL_TOKENS,
        scope._input_tokens + scope._output_tokens,
    )

    if cost:
        scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Set span attributes for prompts and generations
    if capture_message_content:
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, formatted_messages
        )
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._llmresponse
        )

    # Record metrics
    if not disable_metrics:
        record_completion_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=scope._server_address,
            server_port=scope._server_port,
            request_model=request_model,
            response_model=scope._response_model,
            environment=environment,
            application_name=application_name,
            start_time=scope._start_time,
            end_time=scope._end_time,
            cost=cost,
            input_tokens=scope._input_tokens,
            output_tokens=scope._output_tokens,
            tbt=scope._tbt,
            ttft=scope._ttft,
        )

    scope._span.set_status(Status(StatusCode.OK))


def process_streaming_chat_response(
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
    Processes streaming chat response from Sarvam AI API.

    Args:
        scope: The scope object containing span and response data.
        pricing_info: Information about model pricing.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
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
        True,
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes non-streaming chat response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    # Create scope object for common processing
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._server_address = server_address
    scope._server_port = server_port
    scope._kwargs = kwargs
    scope._timestamps = []
    scope._tbt = 0
    scope._ttft = 0

    # Initialize response attributes
    scope._llmresponse = ""
    scope._response_id = ""
    scope._response_model = request_model
    scope._finish_reason = ""
    scope._input_tokens = 0
    scope._output_tokens = 0

    # Extract response data
    response_dict = response_as_dict(response)

    if response_dict.get("choices") and len(response_dict["choices"]) > 0:
        choice = response_dict["choices"][0]
        if choice.get("message") and choice["message"].get("content"):
            scope._llmresponse = choice["message"]["content"]
        if choice.get("finish_reason"):
            scope._finish_reason = choice["finish_reason"]

    if response_dict.get("usage"):
        usage = response_dict["usage"]
        scope._input_tokens = usage.get("prompt_tokens", 0)
        scope._output_tokens = usage.get("completion_tokens", 0)

    if response_dict.get("id"):
        scope._response_id = response_dict["id"]
    if response_dict.get("model"):
        scope._response_model = response_dict["model"]

    # Use common logic for processing
    common_chat_logic(
        scope,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
        False,
    )

    return response


def process_translate_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes translation response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()
    response_dict = response_as_dict(response)

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLATE,
    )

    # Set specific attributes for translation
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Translation-specific request attributes (with API defaults)
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLATE_SOURCE_LANGUAGE,
        kwargs.get("source_language_code", ""),
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLATE_TARGET_LANGUAGE,
        kwargs.get("target_language_code", ""),
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLATE_MODE,
        kwargs.get("mode", "formal"),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLATE_ENABLE_PREPROCESSING,
        kwargs.get("enable_preprocessing", False),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLATE_NUMERALS_FORMAT,
        kwargs.get("numerals_format", "international"),  # API default
    )

    # Optional translation attributes (only set if provided)
    if kwargs.get("speaker_gender"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TRANSLATE_SPEAKER_GENDER,
            kwargs.get("speaker_gender"),
        )
    if kwargs.get("output_script"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TRANSLATE_OUTPUT_SCRIPT,
            kwargs.get("output_script"),
        )

    # Translation response attributes
    if response_dict.get("translated_text"):
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION,
            response_dict.get("translated_text"),
        )

    if response_dict.get("request_id"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID,
            response_dict.get("request_id"),
        )

    if response_dict.get("source_language_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_TRANSLATE_SOURCE_LANGUAGE,
            response_dict.get("source_language_code"),
        )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("input", "")
        )

    span.set_status(Status(StatusCode.OK))

    # Record metrics for translation
    if not disable_metrics:
        # Calculate cost based on input/output text for translation
        input_text = kwargs.get("input", "")
        output_text = response_dict.get("translated_text", "")
        cost = get_chat_model_cost(
            request_model, pricing_info, len(input_text), len(output_text)
        )

        record_completion_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLATE,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
            input_tokens=len(input_text),
            output_tokens=len(output_text),
            tbt=0,
            ttft=0,
        )

    return response


def process_speech_to_text_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes speech-to-text response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()
    response_dict = response_as_dict(response)

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT,
    )

    # Set specific attributes for speech-to-text
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Speech-to-text request attributes (with API defaults)
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_SPEECH_LANGUAGE_CODE,
        kwargs.get(
            "language_code", "unknown"
        ),  # API allows "unknown" for auto-detection
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_SPEECH_WITH_TIMESTAMPS,
        kwargs.get("with_timestamps", False),  # API default
    )

    # Optional speech-to-text attributes (only set if provided)
    if kwargs.get("prompt"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_SPEECH_PROMPT, kwargs.get("prompt")
        )

    # Speech-to-text response attributes
    if response_dict.get("transcript"):
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION,
            response_dict.get("transcript"),
        )

    if response_dict.get("request_id"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID,
            response_dict.get("request_id"),
        )

    if response_dict.get("language_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SPEECH_DETECTED_LANGUAGE,
            response_dict.get("language_code"),
        )

    # Optional response attributes (only set if present)
    if response_dict.get("timestamps"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SPEECH_TIMESTAMPS,
            str(response_dict.get("timestamps")),  # Convert to string for telemetry
        )

    if response_dict.get("diarized_transcript"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SPEECH_DIARIZED_TRANSCRIPT,
            str(
                response_dict.get("diarized_transcript")
            ),  # Convert to string for telemetry
        )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("file", "audio_file")
        )

    span.set_status(Status(StatusCode.OK))

    # Record metrics for speech-to-text
    if not disable_metrics:
        # Calculate cost based on audio duration or default for audio operations
        cost = get_audio_model_cost(
            request_model, pricing_info, "", end_time - start_time
        )

        record_audio_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
        )

    return response


def process_text_to_speech_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes text-to-speech response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_TEXT_TO_SPEECH,
    )

    # Set specific attributes for text-to-speech
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Text-to-speech specific attributes (using Sarvam API defaults)
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_TARGET_LANGUAGE_CODE,
        kwargs.get("target_language_code", ""),
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_SPEAKER,
        kwargs.get("speaker", "Anushka"),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_PITCH,
        kwargs.get("pitch", 0.0),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_PACE,
        kwargs.get("pace", 1.0),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_LOUDNESS,
        kwargs.get("loudness", 1.0),  # API default
    )

    # Additional TTS parameters with API defaults
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_SPEECH_SAMPLE_RATE,
        kwargs.get("speech_sample_rate", 22050),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_ENABLE_PREPROCESSING,
        kwargs.get("enable_preprocessing", False),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TTS_OUTPUT_AUDIO_CODEC,
        kwargs.get("output_audio_codec", ""),
    )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("inputs", "")
        )

    span.set_status(Status(StatusCode.OK))

    # Record metrics for text-to-speech
    if not disable_metrics:
        # Calculate cost based on input text for audio operations
        input_text = kwargs.get("inputs", "")
        cost = get_audio_model_cost(request_model, pricing_info, input_text)

        record_audio_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_TEXT_TO_SPEECH,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
        )

    return response


def process_transliterate_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes transliterate response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()
    response_dict = response_as_dict(response)

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLITERATE,
    )

    # Set specific attributes for transliterate
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Transliterate request attributes (with API defaults)
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLITERATE_SOURCE_LANGUAGE,
        kwargs.get("source_language_code", ""),
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLITERATE_TARGET_LANGUAGE,
        kwargs.get("target_language_code", ""),
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLITERATE_NUMERALS_FORMAT,
        kwargs.get("numerals_format", "international"),  # API default
    )
    span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_TRANSLITERATE_SPOKEN_FORM,
        kwargs.get("spoken_form", False),  # API default
    )

    # Optional transliterate attributes (only set if provided)
    if kwargs.get("spoken_form_numerals_language"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_TRANSLITERATE_SPOKEN_FORM_NUMERALS_LANGUAGE,
            kwargs.get("spoken_form_numerals_language"),
        )

    # Transliterate response attributes
    if response_dict.get("transliterated_text"):
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION,
            response_dict.get("transliterated_text"),
        )

    if response_dict.get("request_id"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID,
            response_dict.get("request_id"),
        )

    if response_dict.get("source_language_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_TRANSLITERATE_SOURCE_LANGUAGE,
            response_dict.get("source_language_code"),
        )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("input", "")
        )

    span.set_status(Status(StatusCode.OK))

    # Record metrics for transliteration
    if not disable_metrics:
        # Calculate cost based on input/output text for transliteration
        input_text = kwargs.get("input", "")
        output_text = response_dict.get("transliterated_text", "")
        cost = get_chat_model_cost(
            request_model, pricing_info, len(input_text), len(output_text)
        )

        record_completion_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLITERATE,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            input_tokens=len(input_text),
            output_tokens=len(output_text),
            tbt=0,
            ttft=0,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
        )

    return response


def process_language_identification_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes language identification response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()
    response_dict = response_as_dict(response)

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_LANGUAGE_IDENTIFICATION,
    )

    # Set specific attributes for language identification
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Language identification response attributes
    if response_dict.get("request_id"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID,
            response_dict.get("request_id"),
        )

    if response_dict.get("language_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_LANGUAGE_CODE,
            response_dict.get("language_code"),
        )

    if response_dict.get("script_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SCRIPT_CODE,
            response_dict.get("script_code"),
        )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("input", "")
        )
        # For language identification, the "completion" would be the detected language/script
        detected_info = (
            f"Language: {response_dict.get('language_code', 'unknown')}, "
            f"Script: {response_dict.get('script_code', 'unknown')}"
        )
        span.set_attribute(SemanticConvention.GEN_AI_CONTENT_COMPLETION, detected_info)

    span.set_status(Status(StatusCode.OK))

    # Record metrics for language identification
    if not disable_metrics:
        # Calculate cost based on input text length for language identification
        input_text = kwargs.get("input", "")
        cost = get_chat_model_cost(request_model, pricing_info, len(input_text), 0)

        record_completion_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_LANGUAGE_IDENTIFICATION,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            input_tokens=len(input_text),
            output_tokens=0,  # No output tokens for language identification
            tbt=0,
            ttft=0,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
        )

    return response


def process_speech_to_text_translate_response(
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
    capture_message_content,
    disable_metrics,
    version,
    **kwargs,
):
    """
    Processes speech-to-text translate response from Sarvam AI API.

    Args:
        response: The API response object.
        request_model: The model used for the request.
        pricing_info: Information about model pricing.
        server_port: The server port.
        server_address: The server address.
        environment: The environment name.
        application_name: The application name.
        metrics: Metrics collection object.
        start_time: The request start time.
        span: The OpenTelemetry span.
        capture_message_content: Whether to capture message content.
        disable_metrics: Whether to disable metrics collection.
        version: The version of the instrumentation.
        **kwargs: Additional keyword arguments.

    Returns:
        The processed response object.
    """
    end_time = time.time()
    response_dict = response_as_dict(response)

    # Create scope object for common framework attributes
    scope = type("GenericScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = end_time

    # Common Framework Span Attributes
    common_framework_span_attributes(
        scope=scope,
        framework_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
        server_address=server_address,
        server_port=server_port,
        environment=environment,
        application_name=application_name,
        version=version,
        endpoint=SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT_TRANSLATE,
    )

    # Set specific attributes for speech-to-text translate
    span.set_attribute(SemanticConvention.GEN_AI_REQUEST_MODEL, request_model)
    span.set_attribute(
        SemanticConvention.GEN_AI_CLIENT_OPERATION_DURATION, end_time - start_time
    )

    # Optional speech-to-text translate attributes (only set if provided)
    if kwargs.get("prompt"):
        span.set_attribute(
            SemanticConvention.GEN_AI_REQUEST_SPEECH_PROMPT, kwargs.get("prompt")
        )

    # Speech-to-text translate response attributes
    if response_dict.get("transcript"):
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION,
            response_dict.get("transcript"),
        )

    if response_dict.get("request_id"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_ID,
            response_dict.get("request_id"),
        )

    if response_dict.get("language_code"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SPEECH_DETECTED_LANGUAGE,
            response_dict.get("language_code"),
        )

    # Optional response attributes (only set if present)
    if response_dict.get("diarized_transcript"):
        span.set_attribute(
            SemanticConvention.GEN_AI_RESPONSE_SPEECH_DIARIZED_TRANSCRIPT,
            str(
                response_dict.get("diarized_transcript")
            ),  # Convert to string for telemetry
        )

    # Set content attributes if capture is enabled
    if capture_message_content:
        span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_PROMPT, kwargs.get("file", "audio_file")
        )

    span.set_status(Status(StatusCode.OK))

    # Record metrics for speech-to-text translate
    if not disable_metrics:
        # Calculate cost based on audio duration for speech-to-text translate
        cost = get_audio_model_cost(
            request_model, pricing_info, "", end_time - start_time
        )

        record_audio_metrics(
            metrics=metrics,
            gen_ai_operation=SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT_TRANSLATE,
            gen_ai_system=SemanticConvention.GEN_AI_SYSTEM_SARVAM,
            server_address=server_address,
            server_port=server_port,
            request_model=request_model,
            response_model=request_model,
            environment=environment,
            application_name=application_name,
            start_time=start_time,
            end_time=end_time,
            cost=cost,
        )

    return response
