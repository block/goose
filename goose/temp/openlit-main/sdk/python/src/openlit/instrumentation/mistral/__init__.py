"""Initializer of Auto Instrumentation of Mistral Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.mistral.mistral import complete, stream, embed
from openlit.instrumentation.mistral.async_mistral import (
    async_complete,
    async_stream,
    async_embed,
)

_instruments = ("mistralai >= 1.0.0",)


class MistralInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Mistral client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name", "default")
        environment = kwargs.get("environment", "default")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        disable_metrics = kwargs.get("disable_metrics")
        version = importlib.metadata.version("mistralai")

        # sync chat completions
        wrap_function_wrapper(
            "mistralai.chat",
            "Chat.complete",
            complete(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync chat streaming
        wrap_function_wrapper(
            "mistralai.chat",
            "Chat.stream",
            stream(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # sync embeddings
        wrap_function_wrapper(
            "mistralai.embeddings",
            "Embeddings.create",
            embed(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # async chat completions
        wrap_function_wrapper(
            "mistralai.chat",
            "Chat.complete_async",
            async_complete(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # async chat streaming
        wrap_function_wrapper(
            "mistralai.chat",
            "Chat.stream_async",
            async_stream(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

        # async embeddings
        wrap_function_wrapper(
            "mistralai.embeddings",
            "Embeddings.create_async",
            async_embed(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            ),
        )

    def _uninstrument(self, **kwargs):
        pass
