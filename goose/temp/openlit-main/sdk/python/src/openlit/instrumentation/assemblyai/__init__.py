"""Initializer of Auto Instrumentation of AssemblyAI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.assemblyai.assemblyai import transcribe

_instruments = ("assemblyai >= 0.35.1",)


class AssemblyAIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for AssemblyAI client library.
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
        version = importlib.metadata.version("assemblyai")

        # sync transcribe
        wrap_function_wrapper(
            "assemblyai.transcriber",
            "Transcriber.transcribe",
            transcribe(
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
