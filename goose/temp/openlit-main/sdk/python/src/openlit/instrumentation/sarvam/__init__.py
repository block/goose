"""Initializer of Auto Instrumentation of Sarvam AI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.sarvam.sarvam import (
    chat_completions,
    text_translate,
    text_transliterate,
    text_identify_language,
    speech_to_text_transcribe,
    speech_to_text_translate,
    text_to_speech_convert,
)
from openlit.instrumentation.sarvam.async_sarvam import (
    async_chat_completions,
    async_text_translate,
    async_text_transliterate,
    async_text_identify_language,
    async_speech_to_text_transcribe,
    async_speech_to_text_translate,
    async_text_to_speech_convert,
)

_instruments = ("sarvamai >= 0.0.1",)


class SarvamInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Sarvam AI's client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("sarvamai")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # Chat completions - sync
        try:
            wrap_function_wrapper(
                "sarvamai.chat.client",
                "ChatClient.completions",
                chat_completions(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Chat completions - async
        try:
            wrap_function_wrapper(
                "sarvamai.chat.client",
                "AsyncChatClient.completions",
                async_chat_completions(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text translate - sync
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "TextClient.translate",
                text_translate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text translate - async
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "AsyncTextClient.translate",
                async_text_translate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text transliterate - sync
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "TextClient.transliterate",
                text_transliterate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text transliterate - async
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "AsyncTextClient.transliterate",
                async_text_transliterate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text identify language - sync
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "TextClient.identify_language",
                text_identify_language(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text identify language - async
        try:
            wrap_function_wrapper(
                "sarvamai.text.client",
                "AsyncTextClient.identify_language",
                async_text_identify_language(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Speech to text - sync
        try:
            wrap_function_wrapper(
                "sarvamai.speech_to_text.client",
                "SpeechToTextClient.transcribe",
                speech_to_text_transcribe(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Speech to text - async
        try:
            wrap_function_wrapper(
                "sarvamai.speech_to_text.client",
                "AsyncSpeechToTextClient.transcribe",
                async_speech_to_text_transcribe(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Speech to text translate - sync
        try:
            wrap_function_wrapper(
                "sarvamai.speech_to_text.client",
                "SpeechToTextClient.translate",
                speech_to_text_translate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Speech to text translate - async
        try:
            wrap_function_wrapper(
                "sarvamai.speech_to_text.client",
                "AsyncSpeechToTextClient.translate",
                async_speech_to_text_translate(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text to speech - sync
        try:
            wrap_function_wrapper(
                "sarvamai.text_to_speech.client",
                "TextToSpeechClient.convert",
                text_to_speech_convert(
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
        except Exception:
            pass  # Module may not exist in all versions

        # Text to speech - async
        try:
            wrap_function_wrapper(
                "sarvamai.text_to_speech.client",
                "AsyncTextToSpeechClient.convert",
                async_text_to_speech_convert(
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
        except Exception:
            pass  # Module may not exist in all versions

    def _uninstrument(self, **kwargs):
        pass
