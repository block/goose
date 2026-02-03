"""Initializer of Auto Instrumentation of OpenAI Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.openai.openai import (
    chat_completions,
    embedding,
    responses,
    chat_completions_parse,
    image_generate,
    image_variatons,
    audio_create,
)
from openlit.instrumentation.openai.async_openai import (
    async_chat_completions,
    async_embedding,
    async_chat_completions_parse,
    async_image_generate,
    async_image_variations,
    async_audio_create,
    async_responses,
)

_instruments = ("openai >= 1.92.0",)


class OpenAIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for OpenAI client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("openai")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # chat completions
        wrap_function_wrapper(
            "openai.resources.chat.completions",
            "Completions.create",
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

        wrap_function_wrapper(
            "openai.resources.chat.completions",
            "AsyncCompletions.create",
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

        # chat completions parse
        wrap_function_wrapper(
            "openai.resources.chat.completions",
            "Completions.parse",
            chat_completions_parse(
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
            "openai.resources.chat.completions",
            "AsyncCompletions.parse",
            async_chat_completions_parse(
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

        # responses
        wrap_function_wrapper(
            "openai.resources.responses.responses",
            "Responses.create",
            responses(
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
            "openai.resources.responses.responses",
            "AsyncResponses.create",
            async_responses(
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

        # embeddings
        wrap_function_wrapper(
            "openai.resources.embeddings",
            "Embeddings.create",
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

        wrap_function_wrapper(
            "openai.resources.embeddings",
            "AsyncEmbeddings.create",
            async_embedding(
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

        # image generation
        wrap_function_wrapper(
            "openai.resources.images",
            "Images.generate",
            image_generate(
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
            "openai.resources.images",
            "AsyncImages.generate",
            async_image_generate(
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

        # image variations
        wrap_function_wrapper(
            "openai.resources.images",
            "Images.create_variation",
            image_variatons(
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
            "openai.resources.images",
            "AsyncImages.create_variation",
            async_image_variations(
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

        # audio generation
        wrap_function_wrapper(
            "openai.resources.audio.speech",
            "Speech.create",
            audio_create(
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
            "openai.resources.audio.speech",
            "AsyncSpeech.create",
            async_audio_create(
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
