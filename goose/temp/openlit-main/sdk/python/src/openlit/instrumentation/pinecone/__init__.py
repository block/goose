"""Initializer of Auto Instrumentation of Pinecone Functions"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.pinecone.pinecone import general_wrap
from openlit.instrumentation.pinecone.async_pinecone import async_general_wrap

_instruments = ("pinecone >= 7.3.0",)

# Pinecone sync operations
PINECONE_SYNC_OPERATIONS = [
    ("pinecone.pinecone", "Pinecone.create_index", "pinecone.create_collection"),
    (
        "pinecone.pinecone",
        "Pinecone.create_index_for_model",
        "pinecone.create_collection",
    ),
    ("pinecone.db_data.index", "Index.upsert", "pinecone.upsert"),
    ("pinecone.db_data.index", "Index.upsert_records", "pinecone.upsert_records"),
    ("pinecone.db_data.index", "Index.query", "pinecone.query"),
    ("pinecone.db_data.index", "Index.search", "pinecone.search"),
    ("pinecone.db_data.index", "Index.fetch", "pinecone.fetch"),
    ("pinecone.db_data.index", "Index.search_records", "pinecone.search_records"),
    ("pinecone.db_data.index", "Index.update", "pinecone.update"),
    ("pinecone.db_data.index", "Index.delete", "pinecone.delete"),
]

# Pinecone async operations
PINECONE_ASYNC_OPERATIONS = [
    (
        "pinecone.pinecone_asyncio",
        "PineconeAsyncio.create_index",
        "pinecone.create_index",
    ),
    (
        "pinecone.pinecone_asyncio",
        "PineconeAsyncio.create_index_for_model",
        "pinecone.create_index",
    ),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.upsert", "pinecone.upsert"),
    (
        "pinecone.db_data.index_asyncio",
        "_IndexAsyncio.upsert_records",
        "pinecone.upsert_records",
    ),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.query", "pinecone.query"),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.search", "pinecone.search"),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.fetch", "pinecone.fetch"),
    (
        "pinecone.db_data.index_asyncio",
        "_IndexAsyncio.search_records",
        "pinecone.search_records",
    ),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.update", "pinecone.update"),
    ("pinecone.db_data.index_asyncio", "_IndexAsyncio.delete", "pinecone.delete"),
]


class PineconeInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Pinecone client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("pinecone")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # Wrap sync operations
        for module, class_method, endpoint in PINECONE_SYNC_OPERATIONS:
            wrap_function_wrapper(
                module,
                class_method,
                general_wrap(
                    endpoint,
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

        # Wrap async operations
        for module, class_method, endpoint in PINECONE_ASYNC_OPERATIONS:
            wrap_function_wrapper(
                module,
                class_method,
                async_general_wrap(
                    endpoint,
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
