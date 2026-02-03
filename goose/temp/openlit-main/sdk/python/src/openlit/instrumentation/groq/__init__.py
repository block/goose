"""Initializer of Auto Instrumentation of Groq Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.groq.groq import chat
from openlit.instrumentation.groq.async_groq import async_chat

_instruments = ("groq >= 0.5.0",)


class GroqInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Groq client library.
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
        version = importlib.metadata.version("groq")

        # Chat completions
        wrap_function_wrapper(
            "groq.resources.chat.completions",
            "Completions.create",
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
            "groq.resources.chat.completions",
            "AsyncCompletions.create",
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
