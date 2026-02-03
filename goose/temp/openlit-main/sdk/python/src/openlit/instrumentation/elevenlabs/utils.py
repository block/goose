"""
ElevenLabs OpenTelemetry instrumentation utility functions
"""

import time

from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)
from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    get_audio_model_cost,
    create_metrics_attributes,
)
from openlit.semcov import SemanticConvention


def format_content(text):
    """
    Process text input to extract content.
    """
    return str(text) if text else ""


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
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_RESPONSE_MODEL, scope._response_model
    )
    scope._span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
    scope._span.set_attribute(SERVICE_NAME, application_name)
    scope._span.set_attribute(SemanticConvention.GEN_AI_REQUEST_IS_STREAM, is_stream)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TBT, scope._tbt)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SERVER_TTFT, scope._ttft)
    scope._span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, version)


def record_audio_metrics(
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
    cost,
):
    """
    Record audio generation metrics for the operation.
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
    metrics["genai_cost"].record(cost, attributes)


def common_audio_logic(
    scope,
    gen_ai_endpoint,
    pricing_info,
    environment,
    application_name,
    metrics,
    capture_message_content,
    disable_metrics,
    version,
):
    """
    Process audio generation request and generate Telemetry
    """

    text = format_content(scope._kwargs.get("text", ""))
    request_model = scope._kwargs.get(
        "model", scope._kwargs.get("model_id", "eleven_multilingual_v2")
    )
    is_stream = False  # ElevenLabs audio generation is not streaming

    cost = get_audio_model_cost(request_model, pricing_info, text)

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
        SemanticConvention.GEN_AI_SYSTEM_ELEVENLABS,
        scope._server_address,
        scope._server_port,
        request_model,
        request_model,
        environment,
        application_name,
        is_stream,
        scope._tbt,
        scope._ttft,
        version,
    )

    # Span Attributes for Cost and Tokens
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Span Attributes for Response parameters
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_OUTPUT_TYPE,
        scope._kwargs.get("output_format", "mp3_44100_128"),
    )

    # Audio-specific span attributes
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_VOICE, scope._kwargs.get("voice_id", "")
    )
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_SETTINGS,
        str(scope._kwargs.get("voice_settings", "")),
    )

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, text)

        # To be removed once the change to span_attributes (from span events) is complete
        scope._span.add_event(
            name=SemanticConvention.GEN_AI_CONTENT_PROMPT_EVENT,
            attributes={
                SemanticConvention.GEN_AI_CONTENT_PROMPT: text,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_audio_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
            SemanticConvention.GEN_AI_SYSTEM_ELEVENLABS,
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
    gen_ai_endpoint,
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
    Process audio generation request and generate Telemetry
    """

    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._args = args

    # Initialize streaming and timing values for ElevenLabs audio generation
    scope._response_model = kwargs.get(
        "model", kwargs.get("model_id", "eleven_multilingual_v2")
    )
    scope._tbt = 0.0
    scope._ttft = scope._end_time - scope._start_time

    common_audio_logic(
        scope,
        gen_ai_endpoint,
        pricing_info,
        environment,
        application_name,
        metrics,
        capture_message_content,
        disable_metrics,
        version,
    )

    return response
