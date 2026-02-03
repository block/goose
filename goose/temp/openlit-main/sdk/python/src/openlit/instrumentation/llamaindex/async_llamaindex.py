"""
Module for monitoring LlamaIndex async applications.
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.llamaindex.utils import (
    process_llamaindex_response,
    OPERATION_MAP,
    set_server_address_and_port,
)


def async_general_wrap(
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
    Generates a telemetry wrapper for LlamaIndex async function calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the LlamaIndex async function call.
        """

        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Get server address and port using the standard helper
        server_address, server_port = set_server_address_and_port(instance)

        operation_type = OPERATION_MAP.get(gen_ai_endpoint, "framework")
        span_name = f"{operation_type} {gen_ai_endpoint}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                # Process response and generate telemetry
                response = process_llamaindex_response(
                    response,
                    operation_type,
                    server_address,
                    server_port,
                    environment,
                    application_name,
                    metrics,
                    start_time,
                    span,
                    capture_message_content,
                    disable_metrics,
                    version,
                    instance,
                    args,
                    endpoint=gen_ai_endpoint,
                    **kwargs,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
