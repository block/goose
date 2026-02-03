"""
OpenLIT LangGraph Instrumentation

Provides comprehensive instrumentation for LangGraph applications including:
- Graph construction (StateGraph, compile, add_node, add_edge)
- Graph execution (invoke, ainvoke, stream, astream)
- State management (get_state, aget_state)
- Checkpointing (AsyncPostgresSaver, PostgresSaver)
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.langgraph.langgraph import (
    general_wrap,
    wrap_compile,
    wrap_add_node,
)
from openlit.instrumentation.langgraph.async_langgraph import (
    async_general_wrap,
    async_checkpoint_wrap,
)

_instruments = ("langgraph >= 0.0.1",)

# === GRAPH EXECUTION OPERATIONS (Always enabled) ===
# These are the core operations users will want to trace
EXECUTION_OPERATIONS = [
    # Sync execution
    ("langgraph.pregel", "Pregel.invoke", "graph_invoke", "sync"),
    ("langgraph.pregel", "Pregel.stream", "graph_stream", "sync"),
    ("langgraph.pregel", "Pregel.get_state", "graph_get_state", "sync"),
    # Async execution
    ("langgraph.pregel", "Pregel.ainvoke", "graph_ainvoke", "async"),
    ("langgraph.pregel", "Pregel.astream", "graph_astream", "async"),
    ("langgraph.pregel", "Pregel.aget_state", "graph_aget_state", "async"),
]

# Alternative module paths for different LangGraph versions
EXECUTION_OPERATIONS_ALT = [
    # Some versions use langgraph.pregel.main
    ("langgraph.pregel.main", "Pregel.invoke", "graph_invoke", "sync"),
    ("langgraph.pregel.main", "Pregel.stream", "graph_stream", "sync"),
    ("langgraph.pregel.main", "Pregel.get_state", "graph_get_state", "sync"),
    ("langgraph.pregel.main", "Pregel.ainvoke", "graph_ainvoke", "async"),
    ("langgraph.pregel.main", "Pregel.astream", "graph_astream", "async"),
    ("langgraph.pregel.main", "Pregel.aget_state", "graph_aget_state", "async"),
]

# === GRAPH CONSTRUCTION OPERATIONS (Detailed tracing only) ===
CONSTRUCTION_OPERATIONS = [
    ("langgraph.graph.state", "StateGraph.__init__", "graph_init", "sync"),
    ("langgraph.graph.state", "StateGraph.add_node", "graph_add_node", "special"),
    ("langgraph.graph.state", "StateGraph.add_edge", "graph_add_edge", "sync"),
    ("langgraph.graph.state", "StateGraph.compile", "graph_compile", "special"),
]

# === CHECKPOINTING OPERATIONS (Detailed tracing only) ===
CHECKPOINT_OPERATIONS = [
    # Async PostgresSaver
    (
        "langgraph.checkpoint.postgres.aio",
        "AsyncPostgresSaver.setup",
        "checkpoint_setup",
        "async",
    ),
    (
        "langgraph.checkpoint.postgres.aio",
        "AsyncPostgresSaver.aput",
        "checkpoint_write",
        "async",
    ),
    (
        "langgraph.checkpoint.postgres.aio",
        "AsyncPostgresSaver.aget_tuple",
        "checkpoint_read",
        "async",
    ),
    # Sync PostgresSaver
    (
        "langgraph.checkpoint.postgres",
        "PostgresSaver.setup",
        "checkpoint_setup",
        "sync",
    ),
    ("langgraph.checkpoint.postgres", "PostgresSaver.put", "checkpoint_write", "sync"),
    (
        "langgraph.checkpoint.postgres",
        "PostgresSaver.get_tuple",
        "checkpoint_read",
        "sync",
    ),
]


class LangGraphInstrumentor(BaseInstrumentor):
    """
    OpenLIT LangGraph instrumentor with comprehensive coverage.

    Features:
    - Graph execution tracing (invoke, stream, get_state)
    - Async support (ainvoke, astream, aget_state)
    - Per-node instrumentation via add_node wrapping
    - Graph structure capture on compile
    - Checkpointer instrumentation
    - Message and token tracking
    - LLM info extraction from node results
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Instrument LangGraph."""
        try:
            version = importlib.metadata.version("langgraph")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"

        environment = kwargs.get("environment", "default")
        application_name = kwargs.get("application_name", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics", False)
        detailed_tracing = kwargs.get("detailed_tracing", False)

        # === EXECUTION OPERATIONS (Always enabled) ===
        self._wrap_execution_operations(
            EXECUTION_OPERATIONS,
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )

        # Try alternative module paths
        self._wrap_execution_operations(
            EXECUTION_OPERATIONS_ALT,
            version,
            environment,
            application_name,
            tracer,
            pricing_info,
            capture_message_content,
            metrics,
            disable_metrics,
        )

        # === CONSTRUCTION OPERATIONS (Detailed tracing or always for key operations) ===
        if detailed_tracing:
            self._wrap_construction_operations(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            )
        else:
            # Always wrap compile and add_node for graph structure and per-node tracing
            self._wrap_key_construction_operations(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            )

        # === CHECKPOINTING OPERATIONS (Detailed tracing only) ===
        if detailed_tracing:
            self._wrap_checkpoint_operations(
                version,
                environment,
                application_name,
                tracer,
                pricing_info,
                capture_message_content,
                metrics,
                disable_metrics,
            )

    def _wrap_execution_operations(
        self,
        operations,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    ):
        """Wrap graph execution operations."""
        for module, method, operation_type, sync_type in operations:
            try:
                if sync_type == "async":
                    wrapper = async_general_wrap(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )
                else:
                    wrapper = general_wrap(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )

                wrap_function_wrapper(module, method, wrapper)
            except Exception:
                # Graceful degradation if module/method doesn't exist
                pass

    def _wrap_construction_operations(
        self,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    ):
        """Wrap all graph construction operations."""
        for module, method, operation_type, sync_type in CONSTRUCTION_OPERATIONS:
            try:
                if sync_type == "special":
                    if "compile" in method:
                        wrapper = wrap_compile(
                            operation_type,
                            version,
                            environment,
                            application_name,
                            tracer,
                            pricing_info,
                            capture_message_content,
                            metrics,
                            disable_metrics,
                        )
                    elif "add_node" in method:
                        wrapper = wrap_add_node(
                            operation_type,
                            version,
                            environment,
                            application_name,
                            tracer,
                            pricing_info,
                            capture_message_content,
                            metrics,
                            disable_metrics,
                        )
                    else:
                        wrapper = general_wrap(
                            operation_type,
                            version,
                            environment,
                            application_name,
                            tracer,
                            pricing_info,
                            capture_message_content,
                            metrics,
                            disable_metrics,
                        )
                else:
                    wrapper = general_wrap(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )

                wrap_function_wrapper(module, method, wrapper)
            except Exception:
                pass

    def _wrap_key_construction_operations(
        self,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    ):
        """Wrap only key construction operations (compile and add_node)."""
        key_operations = [
            (
                "langgraph.graph.state",
                "StateGraph.add_node",
                "graph_add_node",
                "special",
            ),
            ("langgraph.graph.state", "StateGraph.compile", "graph_compile", "special"),
        ]

        for module, method, operation_type, _ in key_operations:
            try:
                if "compile" in method:
                    wrapper = wrap_compile(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )
                elif "add_node" in method:
                    wrapper = wrap_add_node(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )
                else:
                    continue

                wrap_function_wrapper(module, method, wrapper)
            except Exception:
                pass

    def _wrap_checkpoint_operations(
        self,
        version,
        environment,
        application_name,
        tracer,
        pricing_info,
        capture_message_content,
        metrics,
        disable_metrics,
    ):
        """Wrap checkpointing operations."""
        for module, method, operation_type, sync_type in CHECKPOINT_OPERATIONS:
            try:
                if sync_type == "async":
                    wrapper = async_checkpoint_wrap(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )
                else:
                    wrapper = general_wrap(
                        operation_type,
                        version,
                        environment,
                        application_name,
                        tracer,
                        pricing_info,
                        capture_message_content,
                        metrics,
                        disable_metrics,
                    )

                wrap_function_wrapper(module, method, wrapper)
            except Exception:
                pass

    def _uninstrument(self, **kwargs):
        """Remove instrumentation."""
        # Note: Full uninstrumentation would require tracking all wrapped functions
        # For now, this is a placeholder
        pass
