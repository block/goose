"""
Module for monitoring Cohere API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import (
    handle_exception,
    set_server_address_and_port,
)
from openlit.instrumentation.cohere.utils import (
    process_chunk,
    process_chat_response,
    process_streaming_chat_response,
    process_embedding_response,
)
from openlit.semcov import SemanticConvention


def chat(
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
    Generates a telemetry wrapper for GenAI chat function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI chat function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "api.cohere.com", 443
        )
        request_model = kwargs.get("model", "command-r-plus-08-2024")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)
            response = process_chat_response(
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
                capture_message_content=capture_message_content,
                disable_metrics=disable_metrics,
                version=version,
                **kwargs,
            )

        return response

    return wrapper


def chat_stream(
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
    Generates a telemetry wrapper for GenAI chat_stream function call
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
            self._response_id = ""
            self._response_model = ""
            self._finish_reason = ""
            self._tools = None
            self._tool_plan = ""
            self._input_tokens = 0
            self._output_tokens = 0

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
        Wraps the GenAI chat_stream function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "api.cohere.com", 443
        )
        request_model = kwargs.get("model", "command-r-plus-08-2024")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        # Stream endpoint is always streaming
        awaited_wrapped = wrapped(*args, **kwargs)
        span = tracer.start_span(span_name, kind=SpanKind.CLIENT)

        return TracedSyncStream(
            awaited_wrapped, span, span_name, kwargs, server_address, server_port
        )

    return wrapper


def embed(
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
    Generates a telemetry wrapper for GenAI embedding function call
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI embedding function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "api.cohere.com", 443
        )
        request_model = kwargs.get("model", "embed-english-v3.0")

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = wrapped(*args, **kwargs)

            try:
                response = process_embedding_response(
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
                    capture_message_content=capture_message_content,
                    disable_metrics=disable_metrics,
                    version=version,
                    **kwargs,
                )

            except Exception as e:
                handle_exception(span, e)

            return response

    return wrapper
