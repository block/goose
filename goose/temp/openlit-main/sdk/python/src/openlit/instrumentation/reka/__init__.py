"""Initializer of Auto Instrumentation of Reka Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.reka.reka import chat
from openlit.instrumentation.reka.async_reka import async_chat

_instruments = ("reka-api >= 3.2.0",)


class RekaInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Reka client library.
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
        version = importlib.metadata.version("reka-api")

        # Chat completions
        wrap_function_wrapper(
            "reka.chat.client",
            "ChatClient.create",
            chat(
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

        # Chat completions
        wrap_function_wrapper(
            "reka.chat.client",
            "AsyncChatClient.create",
            async_chat(
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
