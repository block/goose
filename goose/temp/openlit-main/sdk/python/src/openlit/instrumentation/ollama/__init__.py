"""
Initializer of Auto Instrumentation of Ollama Functions
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.ollama.ollama import chat, embeddings, generate
from openlit.instrumentation.ollama.async_ollama import (
    async_chat,
    async_embeddings,
    async_generate,
)

_instruments = ("ollama >= 0.2.0",)


# Dispatch wrapper to route instrumentation to chat or embeddings based on path
def _dispatch(sync_chat_wrap, sync_generate_wrap, sync_emb_wrap):
    def wrapper(wrapped, instance, args, kwargs):
        if len(args) > 2 and isinstance(args[2], str):
            op = args[2].rstrip("/").split("/")[-1]
            if op == "chat":
                return sync_chat_wrap(wrapped, instance, args, kwargs)
            if op == "generate":
                return sync_generate_wrap(wrapped, instance, args, kwargs)
            if op == "embeddings":
                return sync_emb_wrap(wrapped, instance, args, kwargs)
        return wrapped(*args, **kwargs)

    return wrapper


def _dispatch_async(async_chat_wrap, async_generate_wrap, async_emb_wrap):
    async def wrapper(wrapped, instance, args, kwargs):
        if len(args) > 2 and isinstance(args[2], str):
            op = args[2].rstrip("/").split("/")[-1]
            if op == "chat":
                return await async_chat_wrap(wrapped, instance, args, kwargs)
            if op == "generate":
                return await async_generate_wrap(wrapped, instance, args, kwargs)
            if op == "embeddings":
                return await async_emb_wrap(wrapped, instance, args, kwargs)
        return await wrapped(*args, **kwargs)

    return wrapper


class OllamaInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Ollama client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name", "default_application")
        environment = kwargs.get("environment", "default_environment")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        disable_metrics = kwargs.get("disable_metrics")
        version = importlib.metadata.version("ollama")

        # Build wrapper factories for chat and embeddings
        sync_chat_wrap = chat(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )
        sync_generate_wrap = generate(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )
        sync_emb_wrap = embeddings(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )
        async_chat_wrap = async_chat(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )
        async_generate_wrap = async_generate(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )
        async_emb_wrap = async_embeddings(
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )

        # Patch underlying request methods to ensure instrumentation regardless of import order
        wrap_function_wrapper(
            "ollama._client",
            "Client._request",
            _dispatch(sync_chat_wrap, sync_generate_wrap, sync_emb_wrap),
        )
        wrap_function_wrapper(
            "ollama._client",
            "AsyncClient._request",
            _dispatch_async(async_chat_wrap, async_generate_wrap, async_emb_wrap),
        )

    def _uninstrument(self, **kwargs):
        # Proper uninstrumentation logic to revert patched methods
        pass
