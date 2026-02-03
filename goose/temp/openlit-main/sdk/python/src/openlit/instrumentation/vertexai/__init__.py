"""Initializer of Auto Instrumentation of VertexAI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.vertexai.vertexai import send_message
from openlit.instrumentation.vertexai.async_vertexai import async_send_message

_instruments = ("google-cloud-aiplatform >= 1.38.1",)


class VertexAIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for VertexAI client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("google-cloud-aiplatform")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # sync generative models
        wrap_function_wrapper(
            "vertexai.generative_models",
            "GenerativeModel.generate_content",
            send_message(
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

        wrap_function_wrapper(
            "vertexai.generative_models",
            "ChatSession.send_message",
            send_message(
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

        # sync language models
        wrap_function_wrapper(
            "vertexai.language_models",
            "ChatSession.send_message",
            send_message(
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

        wrap_function_wrapper(
            "vertexai.language_models",
            "ChatSession.send_message_streaming",
            send_message(
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

        # async generative models
        wrap_function_wrapper(
            "vertexai.generative_models",
            "GenerativeModel.generate_content_async",
            async_send_message(
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

        wrap_function_wrapper(
            "vertexai.generative_models",
            "ChatSession.send_message_async",
            async_send_message(
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

        # async language models
        wrap_function_wrapper(
            "vertexai.language_models",
            "ChatSession.send_message_async",
            async_send_message(
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

        wrap_function_wrapper(
            "vertexai.language_models",
            "ChatSession.send_message_streaming_async",
            async_send_message(
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
