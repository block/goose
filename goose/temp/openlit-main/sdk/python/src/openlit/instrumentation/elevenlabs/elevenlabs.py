"""
Module for monitoring ElevenLabs API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception
from openlit.instrumentation.elevenlabs.utils import process_audio_response
from openlit.semcov import SemanticConvention


def generate(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    Generates a telemetry wrapper for GenAI function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI function call.
        """

        server_address, server_port = "api.elevenlabs.io", 443
        request_model = kwargs.get(
            "model", kwargs.get("model_id", "eleven_multilingual_v2")
        )

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                response = process_audio_response(
                    response=response,
                    gen_ai_endpoint=gen_ai_endpoint,
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    args=args,
                    kwargs=kwargs,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
