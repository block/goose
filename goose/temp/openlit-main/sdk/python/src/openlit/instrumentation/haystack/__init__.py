"""
OpenLIT Haystack Instrumentation
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.haystack.haystack import general_wrap
from openlit.instrumentation.haystack.async_haystack import async_general_wrap

_instruments = ("haystack-ai >= 2.0.0",)


class HaystackInstrumentor(BaseInstrumentor):
    """Optimized instrumentor for Haystack with minimal overhead"""

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("haystack-ai")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")
        detailed_tracing = kwargs.get("detailed_tracing", False)

        # Pipeline operations (always enabled)
        try:
            wrap_function_wrapper(
                "haystack",
                "Pipeline.run",
                general_wrap(
                    "pipeline",
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
                "haystack",
                "AsyncPipeline.run_async",
                async_general_wrap(
                    "pipeline",
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
            pass

        # Component operations (only if detailed_tracing enabled)
        if detailed_tracing:
            components = [
                (
                    "haystack.components.retrievers.in_memory",
                    "InMemoryBM25Retriever.run",
                    "bm25_retriever",
                ),
                (
                    "haystack.components.builders.prompt_builder",
                    "PromptBuilder.run",
                    "prompt_builder",
                ),
                (
                    "haystack.components.generators.openai",
                    "OpenAIGenerator.run",
                    "openai_generator",
                ),
                (
                    "haystack.components.generators.chat.openai",
                    "OpenAIChatGenerator.run",
                    "openai_chat_generator",
                ),
                (
                    "haystack.components.embedders.openai_text_embedder",
                    "OpenAITextEmbedder.run",
                    "text_embedder",
                ),
                (
                    "haystack.components.embedders.openai_document_embedder",
                    "OpenAIDocumentEmbedder.run",
                    "document_embedder",
                ),
            ]

            for module, method, component_type in components:
                try:
                    wrap_function_wrapper(
                        module,
                        method,
                        general_wrap(
                            component_type,
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
                    pass

    def _uninstrument(self, **kwargs):
        pass
