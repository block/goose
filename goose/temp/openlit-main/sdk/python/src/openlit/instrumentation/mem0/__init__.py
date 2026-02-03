# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""
Optimized Auto Instrumentation of mem0 Functions following OpenLIT Framework Guide.
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.mem0.mem0 import mem0_wrap
from openlit.instrumentation.mem0.async_mem0 import async_mem0_wrap

_instruments = ("mem0ai >= 0.1.32",)

# Optimized method configuration with performance priorities
SYNC_METHODS = [
    # Memory initialization
    {
        "package": "mem0",
        "object": "Memory.__init__",
        "endpoint": "memory init",
        "priority": "critical",
    },
    # Core Memory operations
    {
        "package": "mem0",
        "object": "Memory.add",
        "endpoint": "memory add",
        "priority": "high",
    },
    {
        "package": "mem0",
        "object": "Memory.search",
        "endpoint": "memory search",
        "priority": "high",
    },
    {
        "package": "mem0",
        "object": "Memory.get",
        "endpoint": "memory get",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "Memory.get_all",
        "endpoint": "memory get_all",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "Memory.update",
        "endpoint": "memory update",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "Memory.delete",
        "endpoint": "memory delete",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "Memory.delete_all",
        "endpoint": "memory delete_all",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "Memory.history",
        "endpoint": "memory history",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "Memory.reset",
        "endpoint": "memory reset",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "Memory.chat",
        "endpoint": "memory chat",
        "priority": "medium",
    },
    # Internal operations for detailed tracing
    {
        "package": "mem0.memory.main",
        "object": "Memory._add_to_vector_store",
        "endpoint": "memory add_to_vector_store",
        "priority": "medium",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._add_to_graph",
        "endpoint": "memory add_to_graph",
        "priority": "medium",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._search_vector_store",
        "endpoint": "memory search_vector_store",
        "priority": "medium",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._get_all_from_vector_store",
        "endpoint": "memory get_all_from_vector_store",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._create_memory",
        "endpoint": "memory create_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._update_memory",
        "endpoint": "memory update_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._delete_memory",
        "endpoint": "memory delete_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "Memory._create_procedural_memory",
        "endpoint": "memory create_procedural_memory",
        "priority": "low",
    },
]

ASYNC_METHODS = [
    # AsyncMemory initialization
    {
        "package": "mem0",
        "object": "AsyncMemory.__init__",
        "endpoint": "memory init",
        "priority": "critical",
    },
    # Core AsyncMemory operations
    {
        "package": "mem0",
        "object": "AsyncMemory.add",
        "endpoint": "memory add",
        "priority": "high",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.search",
        "endpoint": "memory search",
        "priority": "high",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.get",
        "endpoint": "memory get",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.get_all",
        "endpoint": "memory get_all",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.update",
        "endpoint": "memory update",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.delete",
        "endpoint": "memory delete",
        "priority": "medium",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.delete_all",
        "endpoint": "memory delete_all",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.history",
        "endpoint": "memory history",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.reset",
        "endpoint": "memory reset",
        "priority": "low",
    },
    {
        "package": "mem0",
        "object": "AsyncMemory.chat",
        "endpoint": "memory chat",
        "priority": "medium",
    },
    # Internal async operations for detailed tracing
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._add_to_graph",
        "endpoint": "memory add_to_graph",
        "priority": "medium",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._search_vector_store",
        "endpoint": "memory search_vector_store",
        "priority": "medium",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._get_all_from_vector_store",
        "endpoint": "memory get_all_from_vector_store",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._create_memory",
        "endpoint": "memory create_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._update_memory",
        "endpoint": "memory update_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._delete_memory",
        "endpoint": "memory delete_memory",
        "priority": "low",
    },
    {
        "package": "mem0.memory.main",
        "object": "AsyncMemory._create_procedural_memory",
        "endpoint": "memory create_procedural_memory",
        "priority": "low",
    },
]


class Mem0Instrumentor(BaseInstrumentor):
    """Optimized instrumentor for mem0's client library."""

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Optimized instrumentation with performance considerations."""
        # Extract configuration
        application_name = kwargs.get("application_name")
        environment = kwargs.get("environment")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info")
        capture_message_content = kwargs.get("capture_message_content")

        # Cache version lookup
        try:
            version = importlib.metadata.version("mem0ai")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"

        # Pre-create wrapper args
        wrapper_args = (
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
        )

        # Instrument sync methods
        self._wrap_methods(SYNC_METHODS, mem0_wrap, wrapper_args)

        # Instrument async methods
        self._wrap_methods(ASYNC_METHODS, async_mem0_wrap, wrapper_args)

    def _wrap_methods(self, methods, wrapper_func, wrapper_args):
        """Efficiently wrap methods with error handling."""
        for method_config in methods:
            try:
                wrap_function_wrapper(
                    method_config["package"],
                    method_config["object"],
                    wrapper_func(method_config["endpoint"], *wrapper_args),
                )
            except (ImportError, AttributeError):
                # Gracefully handle missing methods in different mem0 versions
                pass

    @staticmethod
    def _uninstrument(self, **kwargs):
        """Uninstrument mem0."""
        # Currently no cleanup needed
