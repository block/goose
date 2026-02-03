"""
Module for monitoring async Sarvam AI API calls.
"""

import time
from opentelemetry import context as context_api
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception, set_server_address_and_port
from openlit.instrumentation.sarvam.utils import (
    process_chunk,
    process_chat_response,
    process_streaming_chat_response,
    process_translate_response,
    process_transliterate_response,
    process_language_identification_response,
    process_speech_to_text_response,
    process_speech_to_text_translate_response,
    process_text_to_speech_response,
)
from openlit.semcov import SemanticConvention


def async_chat_completions(
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
    Generates a telemetry wrapper for async Sarvam AI Chat Completions calls.
    """

    class TracedAsyncStream:
        """
        Wrapper for async streaming responses to collect telemetry.
        """

        def __init__(
            self,
            wrapped,
            span,
            span_name,
            kwargs,
            server_address,
            server_port,
        ):
            self.__wrapped__ = wrapped
            self._span = span
            self._span_name = span_name
            self._llmresponse = ""
            self._response_id = ""
            self._response_model = ""
            self._finish_reason = ""
            self._input_tokens = 0
            self._output_tokens = 0
            self._response_role = ""
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

        async def __aexit__(self, exc_type, exc_value, traceback):
            await self.__wrapped__.__aexit__(exc_type, exc_value, traceback)

        def __aiter__(self):
            return self

        def __getattr__(self, name):
            """Delegate attribute access to the wrapped object."""
            return getattr(self.__wrapped__, name)

        async def __anext__(self):
            try:
                chunk = await self.__wrapped__.__anext__()
                process_chunk(self, chunk)
                return chunk
            except StopAsyncIteration:
                try:
                    with self._span:
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

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Chat Completions call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        streaming = kwargs.get("stream", False)
        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = kwargs.get("model", "sarvam-m")

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"

        # pylint: disable=no-else-return
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

                try:
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

                except Exception as e:
                    handle_exception(span, e)

            return response

    return wrapper


def async_text_translate(
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
    Generates a telemetry wrapper for async Sarvam AI Text Translate calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Text Translate call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = kwargs.get("model", "mayura:v1")

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLATE} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_translate_response(
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


def async_speech_to_text_transcribe(
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
    Generates a telemetry wrapper for async Sarvam AI Speech to Text transcribe calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Speech to Text transcribe call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = kwargs.get("model", "saarika:v2.5")  # API default

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_speech_to_text_response(
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


def async_text_to_speech_convert(
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
    Generates a telemetry wrapper for async Sarvam AI Text to Speech convert calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Text to Speech convert call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = kwargs.get("model", "bulbul:v2")

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_TEXT_TO_SPEECH} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_text_to_speech_response(
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


def async_text_transliterate(
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
    Generates a telemetry wrapper for async Sarvam AI Text transliterate calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Text transliterate call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = "transliterate:v1"  # Default model for transliteration

        span_name = (
            f"{SemanticConvention.GEN_AI_OPERATION_TYPE_TRANSLITERATE} {request_model}"
        )

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_transliterate_response(
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


def async_text_identify_language(
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
    Generates a telemetry wrapper for async Sarvam AI Text language identification calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Text language identification call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = (
            "language_identification:v1"  # Default model for language identification
        )

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_LANGUAGE_IDENTIFICATION} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_language_identification_response(
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


def async_speech_to_text_translate(
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
    Generates a telemetry wrapper for async Sarvam AI Speech to Text translate calls.
    """

    async def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the async Sarvam AI Speech to Text translate call.
        """

        # Suppression check
        if context_api.get_value(context_api._SUPPRESS_INSTRUMENTATION_KEY):
            return await wrapped(*args, **kwargs)

        server_address, server_port = set_server_address_and_port(
            instance, "api.sarvam.ai", 443
        )
        request_model = kwargs.get("model", "saaras:v2.5")  # API default

        span_name = f"{SemanticConvention.GEN_AI_OPERATION_TYPE_SPEECH_TO_TEXT_TRANSLATE} {request_model}"

        with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
            start_time = time.time()
            response = await wrapped(*args, **kwargs)

            try:
                response = process_speech_to_text_translate_response(
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
