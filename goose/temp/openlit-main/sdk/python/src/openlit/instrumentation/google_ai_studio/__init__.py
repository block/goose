"""Initializer of Auto Instrumentation of Google AI Studio Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.google_ai_studio.google_ai_studio import (
    generate,
    generate_stream,
)

from openlit.instrumentation.google_ai_studio.async_google_ai_studio import (
    async_generate,
    async_generate_stream,
)

_instruments = ("google-genai >= 1.3.0",)


class GoogleAIStudioInstrumentor(BaseInstrumentor):
    """
    An instrumentor for google-genai's client library.
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
        version = importlib.metadata.version("google-genai")

        # sync generate
        wrap_function_wrapper(
            "google.genai.models",
            "Models.generate_content",
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

        # sync stream generate
        wrap_function_wrapper(
            "google.genai.models",
            "Models.generate_content_stream",
            generate_stream(
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

        # async generate
        wrap_function_wrapper(
            "google.genai.models",
            "AsyncModels.generate_content",
            async_generate(
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

        # async stream generate
        wrap_function_wrapper(
            "google.genai.models",
            "AsyncModels.generate_content_stream",
            async_generate_stream(
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
