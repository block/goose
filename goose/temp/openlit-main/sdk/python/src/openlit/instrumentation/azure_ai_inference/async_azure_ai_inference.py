"""
Module for monitoring Azure AI Inference API calls.
"""

import logging
import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import (
    handle_exception,
    set_server_address_and_port,
)
from openlit.instrumentation.azure_ai_inference.utils import (
    process_chunk,
    process_chat_response,
    process_streaming_chat_response,
    process_embedding_response,
)
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)


def async_complete(
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

    class TracedAsyncStream:
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
            self._response_service_tier = ""
            self._tools = None
            self._input_tokens = 0
            self._output_tokens = 0
            self._reasoning_tokens = 0

            self._args = args
            self._kwargs = kwargs
            self._start_time = time.time()
            self._end_time = None
            self._timestamps = []
            self._ttft = 0
            self._tbt = 0
            self._server_address = server_address
            self._server_port = server_port

        async def __aenter__(self):
            await self.__wrapped__.__aenter__()
            return self

        async def __aexit__(self, exc_type, exc_val, exc_tb):
            await self.__wrapped__.__aexit__(exc_type, exc_val, exc_tb)
            process_streaming_chat_response(
                self,
                pricing_info,
                environment,
                application_name,
                metrics,
                capture_message_content,
                disable_metrics,
                version,
            )

        def __aiter__(self):
            return self

        async def __anext__(self):
            chunk = await self.__wrapped__.__anext__()
            process_chunk(self, chunk)
            return chunk

        def __getattr__(self, name):
            return getattr(self.__wrapped__, name)

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI function call.
        """

        streaming = kwargs.get("stream", False)
        server_address, server_port = set_server_address_and_port(
            instance, "models.github.ai", 443
        )
        request_model = kwargs.get("model", "gpt-4o")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        if streaming:
            awaited_wrapped = await wrapped(*args, **kwargs)
            span = tracer.start_span(span_name, kind=SpanKind.CLIENT)

            return TracedAsyncStream(
                awaited_wrapped, span, span_name, kwargs, server_address, server_port
            )

        else:
            with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
                start_time = time.time()
                response = await wrapped(*args, **kwargs)
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


def async_embed(
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

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the GenAI embedding function call.
        """

        server_address, server_port = set_server_address_and_port(
            instance, "models.github.ai", 443
        )
        request_model = kwargs.get("model", "text-embedding-3-small")

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_EMBEDDING} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

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
