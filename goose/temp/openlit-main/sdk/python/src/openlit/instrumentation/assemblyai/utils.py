"""
AssemblyAI OpenTelemetry instrumentation utility functions
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


def format_audio_url(audio_url):
    """
    Process audio URL input to extract content.
    """
    return str(audio_url) if audio_url else ""


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
    Record audio metrics for the operation.
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
    Process audio transcription request and generate Telemetry
    """

    prompt = scope._response.audio_url
    request_model = scope._kwargs.get("speech_model", "best")
    is_stream = False

    # Calculate cost based on audio duration
    cost = get_audio_model_cost(
        request_model, pricing_info, prompt, scope._response.audio_duration
    )

    # Common Span Attributes
    common_span_attributes(
        scope,
        SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
        SemanticConvention.GEN_AI_SYSTEM_ASSEMBLYAI,
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

    # Span Attributes for Response parameters
    scope._span.set_attribute(SemanticConvention.GEN_AI_OUTPUT_TYPE, "text")
    scope._span.set_attribute(SemanticConvention.GEN_AI_RESPONSE_ID, scope._response.id)

    # Span Attributes for Cost
    scope._span.set_attribute(SemanticConvention.GEN_AI_USAGE_COST, cost)

    # Audio-specific span attributes
    scope._span.set_attribute(
        SemanticConvention.GEN_AI_REQUEST_AUDIO_DURATION, scope._response.audio_duration
    )

    # Span Attributes for Content
    if capture_message_content:
        scope._span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, prompt)
        scope._span.set_attribute(
            SemanticConvention.GEN_AI_CONTENT_COMPLETION, scope._response.text
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
                SemanticConvention.GEN_AI_CONTENT_COMPLETION: scope._response.text,
            },
        )

    scope._span.set_status(Status(StatusCode.OK))

    # Metrics
    if not disable_metrics:
        record_audio_metrics(
            metrics,
            SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO,
            SemanticConvention.GEN_AI_SYSTEM_ASSEMBLYAI,
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
    capture_message_content=False,
    disable_metrics=False,
    version="1.0.0",
    **kwargs,
):
    """
    Process audio transcription request and generate Telemetry
    """

    scope = type("GenericScope", (), {})()

    scope._start_time = start_time
    scope._end_time = time.time()
    scope._span = span
    scope._server_address, scope._server_port = server_address, server_port
    scope._kwargs = kwargs
    scope._response = response

    # Initialize streaming and timing values for AssemblyAI transcription
    scope._response_model = kwargs.get("speech_model", "best")
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
