"""
Module for monitoring HF Transformers API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import set_server_address_and_port
from openlit.instrumentation.transformers.utils import process_chat_response
from openlit.semcov import SemanticConvention


def pipeline_wrapper(
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

        server_address, server_port = set_server_address_and_port(
            instance, "127.0.0.1", 80
        )
        request_model = instance.model.config.name_or_path

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            response = process_chat_response(
                instance=instance,
                response=response,
                request_model=request_model,
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

        return response

    return wrapper
