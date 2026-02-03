"""Initializer of Auto Instrumentation of Prem AI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.premai.premai import chat, embedding

_instruments = ("premai >= 0.3.79",)


class PremAIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Prem AI's client library.
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
        version = importlib.metadata.version("premai")

        # sync chat
        wrap_function_wrapper(
            "premai.api",
            "ChatCompletionsModule.create",
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

        # sync embedding
        wrap_function_wrapper(
            "premai.api",
            "EmbeddingsModule.create",
            embedding(
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
        # Proper uninstrumentation logic to revert patched methods
        pass
