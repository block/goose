"""
Haystack async wrapper
"""

import time
from opentelemetry.trace import SpanKind
from opentelemetry import context as context_api
from openlit.__helpers import handle_exception
from openlit.instrumentation.haystack.utils import (
    process_haystack_response,
    OPERATION_MAP,
    set_server_address_and_port,
)


def async_general_wrap(
    endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Optimized async wrapper for Haystack operations"""

    async def wrapper(wrapped, instance, args, kwargs):
        """Fast async wrapper with minimal overhead"""

        # CRITICAL: Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        # Fast operation mapping
        operation_type = OPERATION_MAP.get(endpoint, "framework")

        # Optimized span naming
        if endpoint == "pipeline":
            span_name = f"{operation_type} pipeline"
        else:
            span_name = f"{operation_type} {endpoint}"

        # Fast server address
        server_address, server_port = set_server_address_and_port(instance)

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_haystack_response(
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
                    endpoint=endpoint,
                    **kwargs,
                )
            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
