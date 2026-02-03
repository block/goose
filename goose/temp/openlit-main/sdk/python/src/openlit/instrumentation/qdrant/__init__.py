"""
OpenLIT Qdrant Instrumentation
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.qdrant.qdrant import general_wrap
from openlit.instrumentation.qdrant.async_qdrant import async_general_wrap

_instruments = ("qdrant-client >= 1.9.0",)

# Operations to wrap for both sync and async clients
QDRANT_OPERATIONS = [
    ("create_collection", "qdrant.create_collection"),
    ("delete_collection", "qdrant.delete_collection"),
    ("update_collection", "qdrant.update_collection"),
    ("upload_collection", "qdrant.upload_collection"),
    ("upsert", "qdrant.upsert"),
    ("set_payload", "qdrant.set_payload"),
    ("overwrite_payload", "qdrant.overwrite_payload"),
    ("clear_payload", "qdrant.clear_payload"),
    ("delete_payload", "qdrant.delete_payload"),
    ("upload_points", "qdrant.upload_points"),
    ("update_vectors", "qdrant.update_vectors"),
    ("delete_vectors", "qdrant.delete_vectors"),
    ("delete", "qdrant.delete"),
    ("retrieve", "qdrant.retrieve"),
    ("scroll", "qdrant.scroll"),
    ("search", "qdrant.search"),
    ("search_groups", "qdrant.search_groups"),
    ("recommend", "qdrant.recommend"),
    ("create_payload_index", "qdrant.create_payload_index"),
    ("query_points", "qdrant.query_points"),
]


class QdrantInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Qdrant client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("qdrant-client")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # Wrap sync operations
        for method_name, endpoint in QDRANT_OPERATIONS:
            wrap_function_wrapper(
                "qdrant_client",
                f"QdrantClient.{method_name}",
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
        for method_name, endpoint in QDRANT_OPERATIONS:
            wrap_function_wrapper(
                "qdrant_client",
                f"AsyncQdrantClient.{method_name}",
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
