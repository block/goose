"""Initializer of Auto Instrumentation of GPT4All Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.gpt4all.gpt4all import embed, generate

_instruments = ("gpt4all >= 2.6.0",)


class GPT4AllInstrumentor(BaseInstrumentor):
    """
    An instrumentor for GPT4All client library.
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
        version = importlib.metadata.version("gpt4all")

        # generate
        wrap_function_wrapper(
            "gpt4all",
            "GPT4All.generate",
            generate(
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

        # embed
        wrap_function_wrapper(
            "gpt4all",
            "Embed4All.embed",
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

    def _uninstrument(self, **kwargs):
        pass
