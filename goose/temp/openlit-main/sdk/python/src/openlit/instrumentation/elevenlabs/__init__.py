"""Initializer of Auto Instrumentation of ElevenLabs Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.elevenlabs.elevenlabs import generate
from openlit.instrumentation.elevenlabs.async_elevenlabs import async_generate

_instruments = ("elevenlabs >= 1.4.0",)


class ElevenLabsInstrumentor(BaseInstrumentor):
    """
    An instrumentor for ElevenLabs client library.
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
        version = importlib.metadata.version("elevenlabs")

        # sync text_to_speech.convert
        wrap_function_wrapper(
            "elevenlabs.text_to_speech.client",
            "TextToSpeechClient.convert",
            generate(
                "elevenlabs.text_to_speech",
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

        # async text_to_speech.convert
        wrap_function_wrapper(
            "elevenlabs.text_to_speech.client",
            "AsyncTextToSpeechClient.convert",
            async_generate(
                "elevenlabs.text_to_speech",
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
