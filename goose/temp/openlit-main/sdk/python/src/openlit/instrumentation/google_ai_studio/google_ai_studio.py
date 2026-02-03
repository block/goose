"""
Module for monitoring Google AI Studio API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception, set_server_address_and_port
from openlit.instrumentation.google_ai_studio.utils import (
    process_chat_response,
    process_chunk,
    process_streaming_chat_response,
)
from openlit.semcov import SemanticConvention


def generate(
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
            instance, "generativelanguage.googleapis.com", 443
        )
        request_model = kwargs.get("model", "gemini-2.0-flash")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
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

            except Exception as e:
                handle_exception(span, e)

            # Return original response
            return response

    return wrapper


def generate_stream(
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

    class TracedSyncStream:
        """
        Wrapper for streaming responses to collect telemetry.
        """

        def __init__(
            self,
            wrapped,
            span,
            span_name,
            kwargs,
            server_address,
            server_port,
            **args,
        ):
            self.__wrapped__ = wrapped
            self._span = span
            self._span_name = span_name
            self._llmresponse = ""
            self._finish_reason = ""
            self._output_tokens = ""
            self._input_tokens = ""
            self._response_model = ""
            self._tools = None

            self._args = args
            self._kwargs = kwargs
            self._start_time = time.time()
            self._end_time = None
            self._timestamps = []
            self._ttft = 0
            self._tbt = 0
            self._server_address = server_address
            self._server_port = server_port

        def __enter__(self):
            self.__wrapped__.__enter__()
            return self

        def __exit__(self, exc_type, exc_value, traceback):
            self.__wrapped__.__exit__(exc_type, exc_value, traceback)

        def __iter__(self):
            return self

        def __getattr__(self, name):
            """Delegate attribute access to the wrapped object."""
            return getattr(self.__wrapped__, name)

        def __next__(self):
            try:
                chunk = self.__wrapped__.__next__()
                process_chunk(self, chunk)
                return chunk
            except StopIteration:
                try:
                    with tracer.start_as_current_span(
                        self._span_name, kind=SpanKind.CLIENT
                    ) as self._span:
                        process_streaming_chat_response(
                            self,
                            pricing_info=pricing_info,
                            environment=environment,
                            application_name=application_name,
                            metrics=metrics,
                            capture_message_content=capture_message_content,
                            disable_metrics=disable_metrics,
                            version=version,
                        )

                except Exception as e:
                    handle_exception(self._span, e)
                raise

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "generativelanguage.googleapis.com", 443
        )
        request_model = kwargs.get("model", "gemini-2.0-flash")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        awaited_wrapped = wrapped(*args, **kwargs)
        span = tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT)

        return TracedSyncStream(
            awaited_wrapped, span, span_name, kwargs, server_address, server_port
        )

    return wrapper
