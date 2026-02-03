# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""
Optimized Auto Instrumentation of MCP (Model Context Protocol) Functions following OpenLIT Framework Guide.
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.mcp.mcp import mcp_wrap
from openlit.instrumentation.mcp.async_mcp import async_mcp_wrap
from openlit.instrumentation.mcp.utils import (
    create_jsonrpc_wrapper,
    create_context_propagating_wrapper,
)

_instruments = ("mcp >= 0.1.0",)

# This ensures proper async handling and response capture
CORE_JSONRPC_METHODS = [
    {
        "package": "mcp.shared.session",
        "object": "BaseSession.send_request",
        "endpoint": "jsonrpc send_request",
        "priority": "critical",
        "wrapper_type": "jsonrpc",
    },
]

# Updated method configuration for current MCP API (v1.13.1+)
SYNC_METHODS = [
    # === CRITICAL SERVER OPERATIONS ===
    {
        "package": "mcp.server",
        "object": "Server.run",
        "endpoint": "server run",
        "priority": "critical",
    },
    {
        "package": "mcp.server",
        "object": "Server.call_tool",
        "endpoint": "server call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server",
        "object": "Server.list_tools",
        "endpoint": "server list_tools",
        "priority": "high",
    },
    {
        "package": "mcp.server",
        "object": "Server.read_resource",
        "endpoint": "server read_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server",
        "object": "Server.list_resources",
        "endpoint": "server list_resources",
        "priority": "high",
    },
    # === CRITICAL CLIENT OPERATIONS ===
    {
        "package": "mcp.client.session",
        "object": "ClientSession.initialize",
        "endpoint": "client initialize",
        "priority": "critical",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.call_tool",
        "endpoint": "client call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.list_tools",
        "endpoint": "client list_tools",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.read_resource",
        "endpoint": "client read_resource",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.list_resources",
        "endpoint": "client list_resources",
        "priority": "high",
    },
    # === TRANSPORT LAYER OPERATIONS ===
    {
        "package": "mcp.client.stdio",
        "object": "stdio_client",
        "endpoint": "transport stdio_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.stdio",
        "object": "stdio_server",
        "endpoint": "transport stdio_server",
        "priority": "high",
    },
    {
        "package": "mcp.client.sse",
        "object": "sse_client",
        "endpoint": "transport sse_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.sse",
        "object": "SseServerTransport.connect_sse",
        "endpoint": "transport sse_server",
        "priority": "high",
    },
    {
        "package": "mcp.client.streamable_http",
        "object": "streamablehttp_client",
        "endpoint": "transport http_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.streamable_http",
        "object": "StreamableHTTPServerTransport.connect",
        "endpoint": "transport http_server",
        "priority": "high",
    },
    # === SERVER SESSION OPERATIONS ===
    {
        "package": "mcp.server.session",
        "object": "ServerSession.__init__",
        "endpoint": "session init",
        "priority": "high",
    },
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_request",
        "endpoint": "server send_request",
        "priority": "high",
    },
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_notification",
        "endpoint": "server send_notification",
        "priority": "medium",
    },
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_log_message",
        "endpoint": "server send_log",
        "priority": "medium",
    },
    # === FASTMCP FRAMEWORK OPERATIONS ===
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.run",
        "endpoint": "fastmcp run",
        "priority": "critical",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.add_tool",
        "endpoint": "fastmcp add_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.add_resource",
        "endpoint": "fastmcp add_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.add_prompt",
        "endpoint": "fastmcp add_prompt",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.call_tool",
        "endpoint": "fastmcp call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.read_resource",
        "endpoint": "fastmcp read_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.get_prompt",
        "endpoint": "fastmcp get_prompt",
        "priority": "high",
    },
    # === MANAGER-LEVEL OPERATIONS ===
    {
        "package": "mcp.server.fastmcp.tools.tool_manager",
        "object": "ToolManager.call_tool",
        "endpoint": "manager call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.resources.resource_manager",
        "object": "ResourceManager.get_resource",
        "endpoint": "manager get_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.prompts.manager",
        "object": "PromptManager.render_prompt",
        "endpoint": "manager render_prompt",
        "priority": "high",
    },
    # === WEBSOCKET TRANSPORT OPERATIONS ===
    {
        "package": "mcp.client.websocket",
        "object": "websocket_client",
        "endpoint": "transport websocket_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.websocket",
        "object": "websocket_server",
        "endpoint": "transport websocket_server",
        "priority": "high",
    },
    # === AUTHENTICATION & SECURITY OPERATIONS ===
    {
        "package": "mcp.server.auth.provider",
        "object": "OAuthAuthorizationServerProvider.authorize",
        "endpoint": "auth authorize",
        "priority": "high",
    },
    {
        "package": "mcp.server.auth.provider",
        "object": "OAuthAuthorizationServerProvider.exchange_authorization_code",
        "endpoint": "auth exchange_code",
        "priority": "high",
    },
    {
        "package": "mcp.server.auth.provider",
        "object": "TokenVerifier.verify_token",
        "endpoint": "auth verify_token",
        "priority": "high",
    },
    # === ADVANCED CLIENT OPERATIONS ===
    {
        "package": "mcp.client.session",
        "object": "ClientSession.send_ping",
        "endpoint": "client ping",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.set_logging_level",
        "endpoint": "client set_logging",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.send_progress_notification",
        "endpoint": "client progress",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.complete",
        "endpoint": "client complete",
        "priority": "medium",
    },
    # === MEMORY & PROGRESS OPERATIONS ===
    {
        "package": "mcp.shared.memory",
        "object": "create_connected_server_and_client_session",
        "endpoint": "memory connect",
        "priority": "medium",
    },
    {
        "package": "mcp.shared.progress",
        "object": "ProgressContext.progress",
        "endpoint": "progress update",
        "priority": "low",
    },
]

ASYNC_METHODS = [
    # === CRITICAL ASYNC CLIENT OPERATIONS ===
    {
        "package": "mcp.client.session",
        "object": "ClientSession.call_tool",
        "endpoint": "client call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.list_tools",
        "endpoint": "client list_tools",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.read_resource",
        "endpoint": "client read_resource",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.list_resources",
        "endpoint": "client list_resources",
        "priority": "high",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.initialize",
        "endpoint": "client initialize",
        "priority": "critical",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.send_request",
        "endpoint": "client send_request",
        "priority": "critical",
    },
    # === ASYNC TRANSPORT LAYER OPERATIONS ===
    {
        "package": "mcp.client.stdio",
        "object": "stdio_client",
        "endpoint": "transport stdio_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.stdio",
        "object": "stdio_server",
        "endpoint": "transport stdio_server",
        "priority": "high",
    },
    {
        "package": "mcp.client.sse",
        "object": "sse_client",
        "endpoint": "transport sse_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.sse",
        "object": "SseServerTransport.connect_sse",
        "endpoint": "transport sse_server",
        "priority": "high",
    },
    {
        "package": "mcp.client.streamable_http",
        "object": "streamablehttp_client",
        "endpoint": "transport http_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.streamable_http",
        "object": "StreamableHTTPServerTransport.connect",
        "endpoint": "transport http_server",
        "priority": "high",
    },
    # === CRITICAL ASYNC SERVER OPERATIONS ===
    {
        "package": "mcp.server",
        "object": "Server.run",
        "endpoint": "server run",
        "priority": "critical",
    },
    # === ASYNC SERVER SESSION OPERATIONS ===
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_request",
        "endpoint": "server send_request",
        "priority": "high",
    },
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_notification",
        "endpoint": "server send_notification",
        "priority": "medium",
    },
    {
        "package": "mcp.server.session",
        "object": "ServerSession.send_log_message",
        "endpoint": "server send_log",
        "priority": "medium",
    },
    # === ASYNC FASTMCP FRAMEWORK OPERATIONS ===
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.call_tool",
        "endpoint": "fastmcp call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.read_resource",
        "endpoint": "fastmcp read_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.server",
        "object": "FastMCP.get_prompt",
        "endpoint": "fastmcp get_prompt",
        "priority": "high",
    },
    # === ASYNC MANAGER-LEVEL OPERATIONS ===
    {
        "package": "mcp.server.fastmcp.tools.tool_manager",
        "object": "ToolManager.call_tool",
        "endpoint": "manager call_tool",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.resources.resource_manager",
        "object": "ResourceManager.get_resource",
        "endpoint": "manager get_resource",
        "priority": "high",
    },
    {
        "package": "mcp.server.fastmcp.prompts.manager",
        "object": "PromptManager.render_prompt",
        "endpoint": "manager render_prompt",
        "priority": "high",
    },
    # === ASYNC WEBSOCKET TRANSPORT OPERATIONS ===
    {
        "package": "mcp.client.websocket",
        "object": "websocket_client",
        "endpoint": "transport websocket_client",
        "priority": "high",
    },
    {
        "package": "mcp.server.websocket",
        "object": "websocket_server",
        "endpoint": "transport websocket_server",
        "priority": "high",
    },
    # === ASYNC AUTHENTICATION & SECURITY OPERATIONS ===
    {
        "package": "mcp.server.auth.provider",
        "object": "OAuthAuthorizationServerProvider.authorize",
        "endpoint": "auth authorize",
        "priority": "high",
    },
    {
        "package": "mcp.server.auth.provider",
        "object": "OAuthAuthorizationServerProvider.exchange_authorization_code",
        "endpoint": "auth exchange_code",
        "priority": "high",
    },
    {
        "package": "mcp.server.auth.provider",
        "object": "TokenVerifier.verify_token",
        "endpoint": "auth verify_token",
        "priority": "high",
    },
    # === ASYNC ADVANCED CLIENT OPERATIONS ===
    {
        "package": "mcp.client.session",
        "object": "ClientSession.send_ping",
        "endpoint": "client ping",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.set_logging_level",
        "endpoint": "client set_logging",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.send_progress_notification",
        "endpoint": "client progress",
        "priority": "medium",
    },
    {
        "package": "mcp.client.session",
        "object": "ClientSession.complete",
        "endpoint": "client complete",
        "priority": "medium",
    },
    # === ASYNC MEMORY & PROGRESS OPERATIONS ===
    {
        "package": "mcp.shared.memory",
        "object": "create_connected_server_and_client_session",
        "endpoint": "memory connect",
        "priority": "medium",
    },
    {
        "package": "mcp.shared.progress",
        "object": "ProgressContext.progress",
        "endpoint": "progress update",
        "priority": "low",
    },
]


class MCPInstrumentor(BaseInstrumentor):
    """Optimized instrumentor for MCP's client and server libraries."""

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Optimized instrumentation with performance considerations and detailed tracing support."""
        # Extract configuration
        application_name = kwargs.get("application_name")
        environment = kwargs.get("environment")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info")
        capture_message_content = kwargs.get("capture_message_content")
        metrics = kwargs.get("metrics_dict")
        disable_metrics = kwargs.get("disable_metrics")
        detailed_tracing = kwargs.get("detailed_tracing", True)

        # Cache version lookup
        try:
            version = importlib.metadata.version("mcp")
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
            metrics,
            disable_metrics,
        )

        # Always instrument critical server/client methods first
        critical_methods = [m for m in SYNC_METHODS if m["priority"] == "critical"]
        async_critical_methods = [
            m for m in ASYNC_METHODS if m["priority"] == "critical"
        ]

        self._wrap_methods(critical_methods, mcp_wrap, wrapper_args)
        self._wrap_methods(async_critical_methods, async_mcp_wrap, wrapper_args)

        # Then try JSONRPC wrapper for BaseSession.send_request
        try:
            wrap_function_wrapper(
                "mcp.shared.session",
                "BaseSession.send_request",
                create_jsonrpc_wrapper(
                    "jsonrpc send_request",  # endpoint
                    *wrapper_args,
                ),
            )
        except (ImportError, AttributeError):
            pass

        # Separate transport methods for context propagation
        transport_methods = [
            m
            for m in SYNC_METHODS
            if m["priority"] == "high" and "transport" in m["endpoint"]
        ]
        async_transport_methods = [
            m
            for m in ASYNC_METHODS
            if m["priority"] == "high" and "transport" in m["endpoint"]
        ]

        # Regular high-priority methods (non-transport)
        high_priority_methods = [
            m
            for m in SYNC_METHODS
            if m["priority"] == "high" and "transport" not in m["endpoint"]
        ]
        async_high_priority_methods = [
            m
            for m in ASYNC_METHODS
            if m["priority"] == "high" and "transport" not in m["endpoint"]
        ]

        # Instrument transport methods with context propagation
        self._wrap_transport_methods(transport_methods, wrapper_args)
        self._wrap_transport_methods(async_transport_methods, wrapper_args)

        # Instrument regular methods with standard wrappers
        self._wrap_methods(high_priority_methods, mcp_wrap, wrapper_args)
        self._wrap_methods(async_high_priority_methods, async_mcp_wrap, wrapper_args)

        # Only instrument component operations if detailed_tracing is enabled
        if detailed_tracing:
            component_methods = [
                m for m in SYNC_METHODS if m["priority"] in ["medium", "low"]
            ]
            async_component_methods = [
                m for m in ASYNC_METHODS if m["priority"] in ["medium", "low"]
            ]

            self._wrap_methods(component_methods, mcp_wrap, wrapper_args)
            self._wrap_methods(async_component_methods, async_mcp_wrap, wrapper_args)

    def _wrap_methods(self, methods, wrapper_func, wrapper_args):
        """Efficiently wrap methods with error handling following OpenLIT patterns."""
        for method_config in methods:
            try:
                wrap_function_wrapper(
                    method_config["package"],
                    method_config["object"],
                    wrapper_func(method_config["endpoint"], *wrapper_args),
                )
            except (ImportError, AttributeError):
                # Gracefully handle missing methods in different MCP versions
                pass

    def _wrap_transport_methods(self, methods, wrapper_args):
        """Wrap transport methods with context propagation for cross-client-server span linking."""
        for method_config in methods:
            try:
                wrap_function_wrapper(
                    method_config["package"],
                    method_config["object"],
                    create_context_propagating_wrapper(
                        method_config["endpoint"], *wrapper_args
                    ),
                )
            except (ImportError, AttributeError):
                # Gracefully handle missing transport methods in different MCP versions
                pass

    def _uninstrument(self, **kwargs):
        """Uninstrument MCP."""
        # Currently no cleanup needed
