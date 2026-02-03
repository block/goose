"""
OpenLIT AstraDB Instrumentation
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.astra.astra import general_wrap
from openlit.instrumentation.astra.async_astra import async_general_wrap

_instruments = ("astrapy >= 1.5.2",)

# AstraDB sync operations
ASTRA_SYNC_OPERATIONS = [
    # Database operations
    ("astrapy.database", "Database.create_collection", "astra.create_collection"),
    ("astrapy.database", "Database.drop_collection", "astra.drop_collection"),
    # Collection operations
    ("astrapy.collection", "Collection.insert_one", "astra.insert_one"),
    ("astrapy.collection", "Collection.insert_many", "astra.insert_many"),
    ("astrapy.collection", "Collection.update_one", "astra.update_one"),
    ("astrapy.collection", "Collection.update_many", "astra.update_many"),
    (
        "astrapy.collection",
        "Collection.find_one_and_update",
        "astra.find_one_and_update",
    ),
    ("astrapy.collection", "Collection.find", "astra.find"),
    ("astrapy.collection", "Collection.replace_one", "astra.replace_one"),
    (
        "astrapy.collection",
        "Collection.find_one_and_delete",
        "astra.find_one_and_delete",
    ),
    ("astrapy.collection", "Collection.delete_one", "astra.delete_one"),
    ("astrapy.collection", "Collection.delete_many", "astra.delete_many"),
]

# AstraDB async operations
ASTRA_ASYNC_OPERATIONS = [
    # Async Database operations
    ("astrapy.database", "AsyncDatabase.create_collection", "astra.create_collection"),
    ("astrapy.database", "AsyncDatabase.drop_collection", "astra.drop_collection"),
    # Async Collection operations
    ("astrapy.collection", "AsyncCollection.insert_one", "astra.insert_one"),
    ("astrapy.collection", "AsyncCollection.insert_many", "astra.insert_many"),
    ("astrapy.collection", "AsyncCollection.update_one", "astra.update_one"),
    ("astrapy.collection", "AsyncCollection.update_many", "astra.update_many"),
    (
        "astrapy.collection",
        "AsyncCollection.find_one_and_update",
        "astra.find_one_and_update",
    ),
    ("astrapy.collection", "AsyncCollection.find", "astra.find"),
    ("astrapy.collection", "AsyncCollection.replace_one", "astra.replace_one"),
    (
        "astrapy.collection",
        "AsyncCollection.find_one_and_delete",
        "astra.find_one_and_delete",
    ),
    ("astrapy.collection", "AsyncCollection.delete_one", "astra.delete_one"),
    ("astrapy.collection", "AsyncCollection.delete_many", "astra.delete_many"),
]


class AstraInstrumentor(BaseInstrumentor):
    """
    An instrumentor for AstraDB's client library.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        version = importlib.metadata.version("astrapy")
        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")

        # Wrap sync operations
        for module, class_method, endpoint in ASTRA_SYNC_OPERATIONS:
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
        for module, class_method, endpoint in ASTRA_ASYNC_OPERATIONS:
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
