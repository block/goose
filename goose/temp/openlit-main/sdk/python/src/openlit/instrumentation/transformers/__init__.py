"""
Initializer of Auto Instrumentation of HuggingFace Transformer Functions
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.transformers.transformers import pipeline_wrapper

_instruments = ("transformers >= 4.48.0",)


class TransformersInstrumentor(BaseInstrumentor):
    """
    An instrumentor for HuggingFace Transformer library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name")
        environment = kwargs.get("environment")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info")
        capture_message_content = kwargs.get("capture_message_content")
        disable_metrics = kwargs.get("disable_metrics")
        version = importlib.metadata.version("transformers")

        wrap_function_wrapper(
            "transformers",
            "TextGenerationPipeline.__call__",
            pipeline_wrapper(
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
