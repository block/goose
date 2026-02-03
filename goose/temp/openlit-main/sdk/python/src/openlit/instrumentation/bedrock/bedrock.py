"""
Module for monitoring Amazon Bedrock API calls.
"""

import time
from opentelemetry.trace import SpanKind
from openlit.__helpers import handle_exception, set_server_address_and_port
from openlit.instrumentation.bedrock.utils import (
    process_chunk,
    process_chat_response,
    process_streaming_chat_response,
)
from openlit.semcov import SemanticConvention


def converse(
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
    Generates a telemetry wrapper for AWS Bedrock converse calls.
    """

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the ClientCreator.create_client call.
        """

        def converse_wrapper(original_method, *method_args, **method_kwargs):
            """
            Wraps the individual converse method call.
            """

            server_address, server_port = set_server_address_and_port(
                instance, "aws.amazon.com", 443
            )
            request_model = method_kwargs.get("modelId", "amazon.titan-text-express-v1")

            span_name = (
                f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"
            )

            with tracer.start_as_current_span(span_name, kind=SpanKind.CLIENT) as span:
                start_time = time.time()
                response = original_method(*method_args, **method_kwargs)
                llm_config = method_kwargs.get("inferenceConfig", {})

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
                        llm_config=llm_config,
                        **method_kwargs,
                    )

                except Exception as e:
                    handle_exception(span, e)

                return response

        # Get the original client instance from the wrapper
        client = wrapped(*args, **kwargs)

        # Replace the original method with the instrumented one
        if kwargs.get("service_name") == "bedrock-runtime":
            original_invoke_model = client.converse
            client.converse = lambda *args, **kwargs: converse_wrapper(
                original_invoke_model, *args, **kwargs
            )

        return client

    return wrapper


def converse_stream(
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
    Generates a telemetry wrapper for AWS Bedrock converse_stream calls.
    """

    class TracedSyncStream:
        """
        Wrapper for streaming responses to collect telemetry.
        """

        def __init__(
            self,
            wrapped_response,
            span,
            span_name,
            kwargs,
            server_address,
            server_port,
            **args,
        ):
            self.__wrapped_response = wrapped_response
            # Extract the actual stream iterator from the response
            if isinstance(wrapped_response, dict) and "stream" in wrapped_response:
                self.__wrapped_stream = iter(wrapped_response["stream"])
            else:
                self.__wrapped_stream = iter(wrapped_response)

            self._span = span
            self._span_name = span_name
            self._llmresponse = ""
            self._response_id = ""
            self._response_model = ""
            self._finish_reason = ""
            self._tools = None
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
            if hasattr(self.__wrapped_stream, "__enter__"):
                self.__wrapped_stream.__enter__()
            return self

        def __exit__(self, exc_type, exc_value, traceback):
            if hasattr(self.__wrapped_stream, "__exit__"):
                self.__wrapped_stream.__exit__(exc_type, exc_value, traceback)

        def __iter__(self):
            return self

        def __getattr__(self, name):
            """Delegate attribute access to the wrapped response."""
            return getattr(self.__wrapped_response, name)

        def get(self, key, default=None):
            """Delegate get method to the wrapped response if its a dict."""
            if isinstance(self.__wrapped_response, dict):
                return self.__wrapped_response.get(key, default)
            return getattr(self.__wrapped_response, key, default)

        def __getitem__(self, key):
            """Delegate item access to the wrapped response if its a dict."""
            if isinstance(self.__wrapped_response, dict):
                return self.__wrapped_response[key]
            return getattr(self.__wrapped_response, key)

        def __next__(self):
            try:
                chunk = next(self.__wrapped_stream)
                process_chunk(self, chunk)
                return chunk
            except StopIteration:
                try:
                    llm_config = self._kwargs.get("inferenceConfig", {})
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
                            llm_config=llm_config,
                        )

                except Exception as e:
                    handle_exception(self._span, e)

                raise

    def wrapper(wrapped, instance, args, kwargs):
        """
        Wraps the ClientCreator.create_client call.
        """

        def converse_stream_wrapper(original_method, *method_args, **method_kwargs):
            """
            Wraps the individual converse_stream method call.
            """

            server_address, server_port = set_server_address_and_port(
                instance, "aws.amazon.com", 443
            )
            request_model = method_kwargs.get("modelId", "amazon.titan-text-express-v1")

            span_name = (
                f"{SemanticConvention.GEN_AI_OPERATION_TYPE_CHAT} {request_model}"
            )

            # Get the streaming response
            stream_response = original_method(*method_args, **method_kwargs)
            span = tracer.start_span(span_name, kind=SpanKind.CLIENT)

            return TracedSyncStream(
                stream_response,
                span,
                span_name,
                method_kwargs,
                server_address,
                server_port,
            )

        # Get the original client instance from the wrapper
        client = wrapped(*args, **kwargs)

        # Replace the original method with the instrumented one
        if kwargs.get("service_name") == "bedrock-runtime":
            original_stream_model = client.converse_stream
            client.converse_stream = lambda *args, **kwargs: converse_stream_wrapper(
                original_stream_model, *args, **kwargs
            )

        return client

    return wrapper
