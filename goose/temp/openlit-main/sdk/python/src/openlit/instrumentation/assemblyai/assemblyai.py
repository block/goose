"""
Module for monitoring AssemblyAI API calls.
"""

import logging
import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception, set_server_address_and_port
from openlit.instrumentation.assemblyai.utils import process_audio_response
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


def transcribe(
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
    Generates a telemetry wrapper for AssemblyAI transcribe function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the AssemblyAI transcribe function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "api.assemblyai.com", 443
        )
        request_model = kwargs.get("speech_model", "best")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_AUDIO} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()

            try:
                response = wrapped(*args, **kwargs)

                response = process_audio_response(
                    response=response,
                    gen_ai_endpoint="assemblyai.transcribe",
                    pricing_info=pricing_info,
                    server_port=server_port,
                    server_address=server_address,
                    environment=environment,
                    application_name=application_name,
                    metrics=metrics,
                    start_time=start_time,
                    span=span,
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                    **kwargs,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
