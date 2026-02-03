# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""
Initializer of Auto Instrumentation of Letta Functions
Following OpenTelemetry patterns and comprehensive API coverage
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.letta.letta import (
    create_agent,
    send_message,
    memory_operation,
    tool_operation,
    context_operation,
    source_operation,
)
from openlit.instrumentation.letta.async_letta import (
    async_create_agent,
    async_send_message,
)

_instruments = ("letta-client >= 0.1.0",)

# Comprehensive Letta API operations based on https://docs.letta.com/api-reference/
LETTA_OPERATIONS = {
    # Core Agent Operations - always instrumented
    ("letta_client.agents.client", "AgentsClient.create", "agent"): create_agent,
    ("letta_client.agents.client", "AgentsClient.retrieve", "agent"): create_agent,
    ("letta_client.agents.client", "AgentsClient.modify", "agent"): create_agent,
    ("letta_client.agents.client", "AgentsClient.delete", "agent"): create_agent,
    ("letta_client.agents.client", "AgentsClient.list", "agent"): create_agent,
    # Message Operations - always instrumented (chat operations)
    (
        "letta_client.agents.messages.client",
        "MessagesClient.create_stream",
        "message",
    ): send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.create",
        "message",
    ): send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.list",
        "message",
    ): send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.modify",
        "message",
    ): send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.cancel",
        "message",
    ): send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.reset",
        "message",
    ): send_message,
    # Core Memory Operations - always instrumented
    (
        "letta_client.agents.core_memory.client",
        "CoreMemoryClient.retrieve",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.core_memory.client",
        "CoreMemoryClient.modify",
        "memory",
    ): memory_operation,
    # Memory Blocks Operations - always instrumented
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.list",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.retrieve",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.attach",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.detach",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.create",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.modify",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.blocks.client",
        "BlocksClient.delete",
        "memory",
    ): memory_operation,
}

# Extended operations for detailed_tracing=True
EXTENDED_OPERATIONS = {
    # Tool Operations
    ("letta_client.agents.tools.client", "ToolsClient.list", "tool"): tool_operation,
    ("letta_client.agents.tools.client", "ToolsClient.attach", "tool"): tool_operation,
    ("letta_client.agents.tools.client", "ToolsClient.detach", "tool"): tool_operation,
    ("letta_client.tools.client", "ToolsClient.list", "tool"): tool_operation,
    ("letta_client.tools.client", "ToolsClient.create", "tool"): tool_operation,
    ("letta_client.tools.client", "ToolsClient.retrieve", "tool"): tool_operation,
    ("letta_client.tools.client", "ToolsClient.modify", "tool"): tool_operation,
    ("letta_client.tools.client", "ToolsClient.delete", "tool"): tool_operation,
    # Context Operations
    (
        "letta_client.agents.context.client",
        "ContextClient.retrieve",
        "context",
    ): context_operation,
    (
        "letta_client.agents.context.client",
        "ContextClient.modify",
        "context",
    ): context_operation,
    # Source Operations
    (
        "letta_client.agents.sources.client",
        "SourcesClient.list",
        "source",
    ): source_operation,
    (
        "letta_client.agents.sources.client",
        "SourcesClient.attach",
        "source",
    ): source_operation,
    (
        "letta_client.agents.sources.client",
        "SourcesClient.detach",
        "source",
    ): source_operation,
    ("letta_client.sources.client", "SourcesClient.list", "source"): source_operation,
    ("letta_client.sources.client", "SourcesClient.create", "source"): source_operation,
    (
        "letta_client.sources.client",
        "SourcesClient.retrieve",
        "source",
    ): source_operation,
    ("letta_client.sources.client", "SourcesClient.modify", "source"): source_operation,
    ("letta_client.sources.client", "SourcesClient.delete", "source"): source_operation,
    # Passage Operations
    (
        "letta_client.agents.passages.client",
        "PassagesClient.list",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.passages.client",
        "PassagesClient.retrieve",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.passages.client",
        "PassagesClient.create",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.passages.client",
        "PassagesClient.modify",
        "memory",
    ): memory_operation,
    (
        "letta_client.agents.passages.client",
        "PassagesClient.delete",
        "memory",
    ): memory_operation,
    # Group Operations
    ("letta_client.agents.groups.client", "GroupsClient.list", "agent"): create_agent,
    ("letta_client.agents.groups.client", "GroupsClient.attach", "agent"): create_agent,
    ("letta_client.agents.groups.client", "GroupsClient.detach", "agent"): create_agent,
    # Template Operations
    (
        "letta_client.agents.templates.client",
        "TemplatesClient.list",
        "agent",
    ): create_agent,
    (
        "letta_client.agents.templates.client",
        "TemplatesClient.attach",
        "agent",
    ): create_agent,
    (
        "letta_client.agents.templates.client",
        "TemplatesClient.detach",
        "agent",
    ): create_agent,
}

# Async operations for detailed_tracing=True
ASYNC_OPERATIONS = {
    # Async Agent Operations
    (
        "letta_client.agents.client",
        "AgentsClient.create_async",
        "agent",
    ): async_create_agent,
    (
        "letta_client.agents.client",
        "AgentsClient.retrieve_async",
        "agent",
    ): async_create_agent,
    (
        "letta_client.agents.client",
        "AgentsClient.modify_async",
        "agent",
    ): async_create_agent,
    (
        "letta_client.agents.client",
        "AgentsClient.delete_async",
        "agent",
    ): async_create_agent,
    (
        "letta_client.agents.client",
        "AgentsClient.list_async",
        "agent",
    ): async_create_agent,
    # Async Message Operations
    (
        "letta_client.agents.messages.client",
        "MessagesClient.create_async",
        "message",
    ): async_send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.list_async",
        "message",
    ): async_send_message,
    (
        "letta_client.agents.messages.client",
        "MessagesClient.modify_async",
        "message",
    ): async_send_message,
}


class LettaInstrumentor(BaseInstrumentor):
    """
    Comprehensive Letta instrumentor following OpenTelemetry patterns.
    Provides full coverage of Letta API operations with proper semantic conventions.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        # Extract configuration
        application_name = kwargs.get("application_name", "default_application")
        environment = kwargs.get("environment", "default_environment")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        disable_metrics = kwargs.get("disable_metrics")

        try:
            version = importlib.metadata.version("letta-client")
        except importlib.metadata.PackageNotFoundError:
            version = "0.1.0"  # Fallback version

        # Instrument core operations (always enabled)
        for (
            module_name,
            method_name,
            _operation_type,
        ), wrapper_func in LETTA_OPERATIONS.items():
            try:
                wrap_function_wrapper(
                    module_name,
                    method_name,
                    wrapper_func(
                        f"letta.{method_name.rsplit('.', maxsplit=1)[-1]}",
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
                # Continue if specific operation fails to instrument
                pass

        # Instrument extended operations (detailed_tracing=True only)
        if kwargs.get("detailed_tracing", False):
            for (
                module_name,
                method_name,
                _operation_type,
            ), wrapper_func in EXTENDED_OPERATIONS.items():
                try:
                    wrap_function_wrapper(
                        module_name,
                        method_name,
                        wrapper_func(
                            f"letta.{method_name.rsplit('.', maxsplit=1)[-1]}",
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
                    # Continue if specific operation fails to instrument
                    pass

            # Instrument async operations
            for (
                module_name,
                method_name,
                _operation_type,
            ), wrapper_func in ASYNC_OPERATIONS.items():
                try:
                    wrap_function_wrapper(
                        module_name,
                        method_name,
                        wrapper_func(
                            f"letta.{method_name.rsplit('.', maxsplit=1)[-1]}",
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
                    # Continue if specific operation fails to instrument
                    pass

    def _uninstrument(self, **kwargs):
        """Uninstrumentation would restore original functions."""
        # No cleanup needed for this instrumentation
        return
