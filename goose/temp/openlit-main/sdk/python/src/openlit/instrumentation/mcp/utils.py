# pylint: disable=broad-exception-caught
"""
Utility functions for MCP instrumentation with optimized context caching and performance enhancements.
"""

import json
import time
import inspect
from typing import Dict, Any, Optional

from opentelemetry import context, propagate
from opentelemetry.sdk.resources import SERVICE_NAME, DEPLOYMENT_ENVIRONMENT
from opentelemetry.trace.status import Status, StatusCode

from openlit.semcov import SemanticConvention
from openlit.__helpers import (
    handle_exception,
    record_mcp_metrics,
)


class MCPInstrumentationContext:
    """
    Context object to cache expensive extractions and reduce performance overhead.
    Uses __slots__ for memory optimization following OpenLIT patterns.
    """

    __slots__ = (
        "instance",
        "args",
        "kwargs",
        "version",
        "environment",
        "application_name",
        "pricing_info",
        "capture_message_content",
        "_method_name",
        "_message_id",
        "_tool_name",
        "_resource_uri",
        "_server_name",
        "_transport_type",
        "_error_info",
        "_params",
        "_result",
        "_server_info",
        "_wrapped_function_name",
        "_endpoint_method",
        "_request_payload",
        "_response_payload",
    )

    def __init__(
        self,
        instance,
        args,
        kwargs,
        version,
        environment,
        application_name,
        pricing_info,
        capture_message_content,
    ):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name
        self.pricing_info = pricing_info
        self.capture_message_content = capture_message_content

        # Cache expensive operations with lazy loading
        self._method_name = None
        self._message_id = None
        self._tool_name = None
        self._resource_uri = None
        self._server_name = None
        self._transport_type = None
        self._error_info = None
        self._wrapped_function_name = None
        self._endpoint_method = None
        self._params = None
        self._result = None
        self._server_info = None
        self._request_payload = None
        self._response_payload = None

    @property
    def method_name(self) -> str:
        """Get MCP method name with caching."""
        if self._method_name is None:
            self._method_name = self._extract_method_name()
        return self._method_name

    @property
    def message_id(self) -> Optional[str]:
        """Get MCP message ID with caching."""
        if self._message_id is None:
            self._message_id = self._extract_message_id()
        return self._message_id

    @property
    def tool_name(self) -> Optional[str]:
        """Get tool name with caching."""
        if self._tool_name is None:
            self._tool_name = self._extract_tool_name()
        return self._tool_name

    @property
    def resource_uri(self) -> Optional[str]:
        """Get resource URI with caching."""
        if self._resource_uri is None:
            self._resource_uri = self._extract_resource_uri()
        return self._resource_uri

    @property
    def transport_type(self) -> str:
        """Get transport type with caching."""
        if self._transport_type is None:
            self._transport_type = self._extract_transport_type()
        return self._transport_type

    @property
    def server_info(self) -> Dict[str, Any]:
        """Get server info with caching."""
        if self._server_info is None:
            self._server_info = self._extract_server_info()
        return self._server_info

    @property
    def request_payload(self) -> Optional[str]:
        """Get request payload with caching."""
        if self._request_payload is None:
            self._request_payload = self._extract_request_payload()
        return self._request_payload

    @property
    def response_payload(self) -> Optional[str]:
        """Get response payload with caching."""
        return self._response_payload

    def set_response_payload(self, response_payload: str) -> None:
        """Set response payload after execution."""
        self._response_payload = response_payload

    def _extract_method_name(self) -> str:
        """Extract MCP method name from request or context."""
        try:
            # Try to detect from function being wrapped first (most reliable)
            if hasattr(self, "_wrapped_function_name"):
                func_name = self._wrapped_function_name
                if func_name in [
                    "run",
                    "list_tools",
                    "call_tool",
                    "list_resources",
                    "read_resource",
                    "list_prompts",
                    "get_prompt",
                    "initialize",
                    "send_request",
                ]:
                    return func_name

            # Try to detect from endpoint method
            if hasattr(self, "_endpoint_method"):
                return self._endpoint_method

            # Check for MCP-specific method patterns
            # For call_tool operations, try to get tool name
            if "name" in self.kwargs:
                tool_name = self.kwargs["name"]
                if isinstance(tool_name, str):
                    return tool_name

            # Try to get from method parameter in kwargs
            if "method" in self.kwargs:
                return str(self.kwargs["method"])

            # Try to get from args - check for tool name in call_tool
            if self.args and (len(self.args) >= 1 and isinstance(self.args[0], str)):
                first_arg = self.args[0]
                if first_arg in [
                    "calculator",
                    "text_analyzer",
                    "data_processor",
                    "add",
                    "analyze",
                    "list_tools",
                    "call_tool",
                    "list_resources",
                    "read_resource",
                    "list_prompts",
                    "get_prompt",
                    "run",
                    "initialize",
                ]:
                    return first_arg

            # Try to get from instance method name
            if hasattr(self.instance, "__name__"):
                instance_name = self.instance.__name__
                if instance_name != "<lambda>":  # Avoid lambda names
                    return instance_name

            # Last resort - try to infer from instance class
            if self.instance and hasattr(self.instance, "__class__"):
                class_name = self.instance.__class__.__name__.lower()
                if "server" in class_name:
                    return "server_operation"
                elif "client" in class_name:
                    return "client_operation"

            return "unknown"
        except Exception:
            return "unknown"

    def _extract_message_id(self) -> Optional[str]:
        """Extract message ID from MCP request."""
        try:
            # Check kwargs for id
            if "id" in self.kwargs:
                return str(self.kwargs["id"])

            # Check if first arg is a message object with id
            if self.args and hasattr(self.args[0], "id"):
                return str(self.args[0].id)

            return None
        except Exception:
            return None

    def _extract_tool_name(self) -> Optional[str]:
        """Extract tool name for tool operations."""
        try:
            # Check kwargs
            if "name" in self.kwargs:
                return str(self.kwargs["name"])

            # Check params for tool name
            if "params" in self.kwargs and isinstance(self.kwargs["params"], dict):
                params = self.kwargs["params"]
                if "name" in params:
                    return str(params["name"])

            return None
        except Exception:
            return None

    def _extract_resource_uri(self) -> Optional[str]:
        """Extract resource URI for resource operations."""
        try:
            # Check kwargs
            if "uri" in self.kwargs:
                return str(self.kwargs["uri"])

            # Check params for URI
            if "params" in self.kwargs and isinstance(self.kwargs["params"], dict):
                params = self.kwargs["params"]
                if "uri" in params:
                    return str(params["uri"])

            return None
        except Exception:
            return None

    def _extract_transport_type(self) -> str:
        """Extract transport type from instance."""
        try:
            # Check instance class name for transport type
            if self.instance:
                instance_class = self.instance.__class__.__name__.lower()
                instance_module = getattr(
                    self.instance.__class__, "__module__", ""
                ).lower()

                # Check for stdio transport patterns
                if (
                    "stdio" in instance_class
                    or "stdio" in instance_module
                    or "clientsession" in instance_class
                ):
                    return SemanticConvention.MCP_TRANSPORT_STDIO
                # Check for SSE transport
                elif "sse" in instance_class or "sse" in instance_module:
                    return SemanticConvention.MCP_TRANSPORT_SSE
                # Check for WebSocket transport
                elif "websocket" in instance_class or "ws" in instance_class:
                    return SemanticConvention.MCP_TRANSPORT_WEBSOCKET

            # Check kwargs for transport hints
            if self.kwargs:
                # Look for transport-related parameters
                if any(key in self.kwargs for key in ["command", "args", "stdio"]):
                    return SemanticConvention.MCP_TRANSPORT_STDIO
                elif any(
                    key in self.kwargs for key in ["url", "endpoint"]
                ) and "sse" in str(self.kwargs):
                    return SemanticConvention.MCP_TRANSPORT_SSE
                elif any(key in self.kwargs for key in ["ws_url", "websocket"]):
                    return SemanticConvention.MCP_TRANSPORT_WEBSOCKET

            # Default to stdio for most MCP implementations
            return SemanticConvention.MCP_TRANSPORT_STDIO
        except Exception:
            return "unknown"

    def _extract_server_info(self) -> Dict[str, Any]:
        """Extract server information."""
        try:
            server_info = {}

            # Try to get server name from instance
            if hasattr(self.instance, "name"):
                server_info["name"] = str(self.instance.name)
            elif hasattr(self.instance, "server_name"):
                server_info["name"] = str(self.instance.server_name)

            # Try to get server version
            if hasattr(self.instance, "version"):
                server_info["version"] = str(self.instance.version)

            return server_info
        except Exception:
            return {}

    def _extract_request_payload(self) -> Optional[str]:
        """Extract request payload from args/kwargs in JSON format."""
        try:
            payload = {}

            # Try to get method name
            method = self.method_name if self.method_name != "unknown" else None
            if method:
                payload["method"] = method

            # Extract parameters from kwargs and args
            params = {}

            # Get common MCP parameters
            if "name" in self.kwargs:
                params["name"] = self.kwargs["name"]
            if "arguments" in self.kwargs:
                params["arguments"] = self.kwargs["arguments"]
            if "uri" in self.kwargs:
                params["uri"] = self.kwargs["uri"]

            # Add other kwargs that look like MCP parameters
            for key, value in self.kwargs.items():
                if key in [
                    "topic",
                    "meta",
                    "protocolVersion",
                    "capabilities",
                    "clientInfo",
                ]:
                    params[key] = value

            # If we have args and no specific params, include args
            if not params and self.args:
                if len(self.args) == 1:
                    params = (
                        self.args[0]
                        if isinstance(self.args[0], dict)
                        else {"value": self.args[0]}
                    )
                else:
                    params = {"args": list(self.args)}

            if params:
                payload["params"] = params
            elif not payload:
                # Fallback if no clear structure
                return None

            return json.dumps(payload, default=str)

        except Exception:
            return None

    def get_enhanced_span_name(self, operation_type: str) -> str:
        """Generate span names following operation_type operation_name convention (e.g., 'mcp tools/list', 'mcp resources/read')."""
        method = self.method_name or "unknown"

        # Map operations to operation_type operation_name convention
        if "list_tools" in method or (operation_type == "tool" and "list" in method):
            return "mcp tools/list"
        elif "call_tool" in method or (
            operation_type == "tool"
            and any(
                x in method for x in ["calculator", "text_analyzer", "data_processor"]
            )
        ):
            return "mcp tools/call"
        elif "list_resources" in method or (
            operation_type == "resource" and "list" in method
        ):
            return "mcp resources/list"
        elif "read_resource" in method or (
            operation_type == "resource" and "read" in method
        ):
            return "mcp resources/read"
        elif "list_prompts" in method or (
            operation_type == "prompt" and "list" in method
        ):
            return "mcp prompts/list"
        elif "get_prompt" in method or (operation_type == "prompt" and "get" in method):
            return "mcp prompts/get"
        elif "initialize" in method:
            return "mcp initialize"
        elif "run" in method:
            return "mcp server/run"
        elif "send_request" in method:
            return "mcp transport/request"
        elif "stdio_client" in method:
            return "mcp transport/stdio_client"
        elif "stdio_server" in method:
            return "mcp transport/stdio_server"
        elif "sse_client" in method:
            return "mcp transport/sse_client"
        elif "sse_server" in method or "connect_sse" in method:
            return "mcp transport/sse_server"
        elif "http_client" in method or "streamablehttp_client" in method:
            return "mcp transport/http_client"
        elif "http_server" in method or "StreamableHTTPServerTransport" in method:
            return "mcp transport/http_server"
        elif "__init__" in method and operation_type == "session":
            return "mcp session/init"
        # FastMCP Framework operations
        elif "fastmcp" in method:
            if "run" in method:
                return "mcp fastmcp/run"
            elif "add_tool" in method:
                return "mcp fastmcp/add_tool"
            elif "add_resource" in method:
                return "mcp fastmcp/add_resource"
            elif "add_prompt" in method:
                return "mcp fastmcp/add_prompt"
            elif "call_tool" in method:
                return "mcp fastmcp/call_tool"
            elif "read_resource" in method:
                return "mcp fastmcp/read_resource"
            elif "get_prompt" in method:
                return "mcp fastmcp/get_prompt"
            else:
                return "mcp fastmcp/operation"
        # Manager-level operations
        elif "manager" in method:
            if "tool_manager" in method:
                return "mcp managers/tool"
            elif "resource_manager" in method:
                return "mcp managers/resource"
            elif "prompt_manager" in method:
                return "mcp managers/prompt"
            else:
                return "mcp managers/operation"
        # WebSocket transport operations
        elif "websocket" in method:
            if "websocket_client" in method:
                return "mcp transport/websocket_client"
            elif "websocket_server" in method:
                return "mcp transport/websocket_server"
            else:
                return "mcp transport/websocket"
        # Authentication & security operations
        elif "auth" in method:
            if "authorize" in method:
                return "mcp auth/authorize"
            elif "exchange_code" in method:
                return "mcp auth/exchange_code"
            elif "verify_token" in method:
                return "mcp auth/verify_token"
            else:
                return "mcp auth/operation"
        # Advanced client operations
        elif "ping" in method:
            return "mcp client/ping"
        elif "set_logging" in method:
            return "mcp client/set_logging"
        elif "progress" in method and "client" in operation_type:
            return "mcp client/progress"
        elif "complete" in method:
            return "mcp client/complete"
        # Memory & progress operations
        elif "memory" in method:
            if "connect" in method:
                return "mcp memory/connect"
            else:
                return "mcp memory/operation"
        elif "progress_context" in method or "ProgressContext" in method:
            return "mcp progress/update"
        elif method == "unknown":
            # Better fallback for unknown methods - use operation_type if available
            if operation_type and operation_type != "mcp":
                return f"mcp {operation_type}/operation"
            else:
                return "mcp operation"
        else:
            # Fallback to operation_type operation_name convention
            return f"mcp {operation_type}/{method}"


def _simplify_operation_name(operation_name: str) -> str:
    """Simplify operation names for span attributes (e.g., 'server call_tool' -> 'tools_call')."""
    # Remove server/client prefixes
    simplified = operation_name.replace("server ", "").replace("client ", "")
    simplified = (
        simplified.replace("fastmcp ", "")
        .replace("manager ", "")
        .replace("transport ", "")
    )

    # Map to standard operation names
    if "call_tool" in simplified:
        return "tools_call"
    elif "list_tools" in simplified:
        return "tools_list"
    elif "list_resources" in simplified:
        return "resources_list"
    elif "read_resource" in simplified:
        return "resources_read"
    elif "list_prompts" in simplified:
        return "prompts_list"
    elif "get_prompt" in simplified:
        return "prompts_get"
    elif "initialize" in simplified:
        return "initialize"
    elif "run" in simplified:
        return "server_run"
    elif "send_request" in simplified:
        return "transport_request"
    elif "stdio_client" in simplified:
        return "transport_stdio_client"
    elif "stdio_server" in simplified:
        return "transport_stdio_server"
    elif "sse_client" in simplified:
        return "transport_sse_client"
    elif "sse_server" in simplified or "connect_sse" in simplified:
        return "transport_sse_server"
    elif "websocket_client" in simplified:
        return "transport_websocket_client"
    elif "websocket_server" in simplified:
        return "transport_websocket_server"
    elif "http_client" in simplified or "streamablehttp_client" in simplified:
        return "transport_http_client"
    elif "http_server" in simplified or "StreamableHTTPServerTransport" in simplified:
        return "transport_http_server"
    elif "add_tool" in simplified:
        return "fastmcp_add_tool"
    elif "add_resource" in simplified:
        return "fastmcp_add_resource"
    elif "add_prompt" in simplified:
        return "fastmcp_add_prompt"
    elif "authorize" in simplified:
        return "auth_authorize"
    elif "exchange_code" in simplified:
        return "auth_exchange_code"
    elif "verify_token" in simplified:
        return "auth_verify_token"
    elif "ping" in simplified:
        return "client_ping"
    elif "set_logging" in simplified:
        return "client_set_logging"
    elif "progress" in simplified:
        return "progress_update"
    elif "complete" in simplified:
        return "completion"
    elif "memory" in simplified and "connect" in simplified:
        return "memory_connect"
    else:
        # Fallback - just clean up the name
        return simplified.replace(" ", "_").lower()


def set_mcp_span_attributes(
    span,
    operation_name: str,
    ctx: MCPInstrumentationContext,
    endpoint: str,
    response: Any = None,
    error: Optional[Exception] = None,
    **kwargs,
):
    """
    Optimized attribute setting with context caching and comprehensive MCP attributes.
    """

    # Set core MCP attributes using cached context - simplify operation name
    simplified_operation = _simplify_operation_name(operation_name)
    span.set_attribute(SemanticConvention.MCP_OPERATION, simplified_operation)
    span.set_attribute(SemanticConvention.MCP_SYSTEM, "mcp")

    # Set environment attributes
    span.set_attribute(DEPLOYMENT_ENVIRONMENT, ctx.environment)
    span.set_attribute(SERVICE_NAME, ctx.application_name)
    span.set_attribute(SemanticConvention.MCP_SDK_VERSION, ctx.version)

    # Set MCP-specific attributes
    if ctx.method_name:  # Only set if not None to avoid OpenTelemetry warning
        span.set_attribute(SemanticConvention.MCP_METHOD, ctx.method_name)

    if ctx.request_payload and ctx.capture_message_content:
        span.set_attribute(SemanticConvention.MCP_REQUEST_PAYLOAD, ctx.request_payload)

    if ctx.message_id:
        span.set_attribute(SemanticConvention.MCP_MESSAGE_ID, ctx.message_id)

    span.set_attribute(SemanticConvention.MCP_TRANSPORT_TYPE, ctx.transport_type)

    # Set tool-specific attributes
    if ctx.tool_name:
        span.set_attribute(SemanticConvention.MCP_TOOL_NAME, ctx.tool_name)

    # Set resource-specific attributes
    if ctx.resource_uri:
        span.set_attribute(SemanticConvention.MCP_RESOURCE_URI, ctx.resource_uri)

    # Set server information
    server_info = ctx.server_info
    if server_info.get("name"):
        span.set_attribute(SemanticConvention.MCP_SERVER_NAME, server_info["name"])
    if server_info.get("version"):
        span.set_attribute(
            SemanticConvention.MCP_SERVER_VERSION, server_info["version"]
        )

    # Capture request/response content if enabled
    if ctx.capture_message_content:
        _capture_mcp_content(span, ctx, response, **kwargs)

    # Handle errors
    if error:
        _capture_mcp_error(span, error)

    # Extract new enhanced attributes
    _extract_enhanced_mcp_attributes(span, ctx, endpoint, response, **kwargs)


def _capture_mcp_content(span, ctx: MCPInstrumentationContext, response: Any, **kwargs):
    """Capture MCP request and response content with MIME types (OpenLIT enhancement)."""
    try:
        # Capture request parameters
        if "params" in ctx.kwargs:
            params_str = json.dumps(ctx.kwargs["params"], default=str)
            span.set_attribute(SemanticConvention.MCP_PARAMS, params_str)

        # Capture response content
        if response is not None:
            if hasattr(response, "result"):
                result_str = json.dumps(response.result, default=str)
                span.set_attribute(SemanticConvention.MCP_RESULT, result_str)
            elif isinstance(response, (dict, list, str, int, float, bool)):
                result_str = json.dumps(response, default=str)
                span.set_attribute(SemanticConvention.MCP_RESULT, result_str)

        # Capture tool arguments for tool calls
        if ctx.tool_name and "arguments" in ctx.kwargs:
            args_str = json.dumps(ctx.kwargs["arguments"], default=str)
            span.set_attribute(SemanticConvention.MCP_TOOL_ARGUMENTS, args_str)

    except Exception:
        # Silently ignore content capture errors
        pass


def _capture_mcp_error(span, error: Exception):
    """Capture MCP error information."""
    try:
        # Set error attributes
        span.set_attribute(SemanticConvention.MCP_ERROR_MESSAGE, str(error))

        # Try to extract MCP-specific error details
        if hasattr(error, "code"):
            span.set_attribute(SemanticConvention.MCP_ERROR_CODE, str(error.code))

        if hasattr(error, "data"):
            error_data = json.dumps(error.data, default=str)
            span.set_attribute(SemanticConvention.MCP_ERROR_DATA, error_data)

    except Exception:
        # Silently ignore error capture errors
        pass


def process_mcp_response(
    response,
    ctx: MCPInstrumentationContext,
    span,
    start_time: float,
    endpoint: str,
    metrics,
    disable_metrics: bool = False,
    **kwargs,
):
    """
    Process MCP response with comprehensive business intelligence tracking.
    """

    try:
        end_time = time.time()

        # Set basic response attributes
        set_mcp_span_attributes(span, endpoint, ctx, endpoint, response, **kwargs)

        # Calculate performance metrics (OpenLIT business intelligence)
        duration = end_time - start_time
        span.set_attribute(SemanticConvention.MCP_CLIENT_OPERATION_DURATION, duration)

        try:
            if response and ctx.capture_message_content:
                # Handle different response types intelligently
                if hasattr(response, "__await__"):
                    # It's a coroutine - extract meaningful info without awaiting
                    if hasattr(response, "cr_code"):
                        func_name = response.cr_code.co_name
                    else:
                        func_name = (
                            str(response).rsplit(".", maxsplit=1)[-1].split(" ")[0]
                        )

                    response_info = {
                        "type": "mcp_coroutine",
                        "function": func_name,
                        "awaitable": True,
                        "note": "Response data available after await",
                    }
                    response_json = json.dumps(response_info)
                elif hasattr(response, "__dict__"):
                    # It's a structured object - capture its content
                    try:
                        response_json = json.dumps(response.__dict__, default=str)
                    except Exception:
                        response_json = json.dumps(
                            {"type": type(response).__name__, "value": str(response)}
                        )
                else:
                    # Regular response
                    response_json = json.dumps(response, default=str)

                ctx.set_response_payload(response_json)
                span.set_attribute(
                    SemanticConvention.MCP_RESPONSE_PAYLOAD, response_json
                )
        except Exception:
            pass

        # Capture message size for performance analysis
        try:
            if response:
                response_size = len(json.dumps(response, default=str))
                span.set_attribute(SemanticConvention.MCP_RESPONSE_SIZE, response_size)
        except Exception:
            pass

        # Record MCP-specific metrics for business intelligence
        if metrics and not disable_metrics:
            # Extract MCP-specific information for enhanced metrics
            mcp_operation = _simplify_operation_name(endpoint)
            mcp_method = (
                ctx.method_name or endpoint.split()[-1] if " " in endpoint else endpoint
            )

            # Extract tool, resource, and prompt names from context
            tool_name = None
            resource_uri = None
            prompt_name = None

            # Enhanced extraction logic for MCP operations
            # Try to extract tool name from various sources
            if "tool_name" in kwargs:
                tool_name = kwargs["tool_name"]
            elif "name" in kwargs and (
                "tool" in endpoint.lower() or "call" in endpoint.lower()
            ):
                tool_name = kwargs["name"]
            elif len(ctx.args) > 0:
                # For MCP tool calls, tool name is typically the first argument
                if isinstance(ctx.args[0], str) and (
                    "tool" in endpoint.lower() or "call" in endpoint.lower()
                ):
                    tool_name = ctx.args[0]
                elif hasattr(ctx.args[0], "name"):
                    tool_name = ctx.args[0].name
                elif isinstance(ctx.args[0], dict) and "name" in ctx.args[0]:
                    tool_name = ctx.args[0]["name"]

            # Try to extract resource URI and name from various sources
            resource_name = None
            if "resource_uri" in kwargs:
                resource_uri = kwargs["resource_uri"]
            elif "uri" in kwargs:
                resource_uri = kwargs["uri"]
            elif len(ctx.args) > 0 and "resource" in endpoint.lower():
                # For MCP resource operations, URI is typically the first argument
                if isinstance(ctx.args[0], str):
                    resource_uri = ctx.args[0]
                elif hasattr(ctx.args[0], "uri"):
                    resource_uri = ctx.args[0].uri
                elif isinstance(ctx.args[0], dict) and "uri" in ctx.args[0]:
                    resource_uri = ctx.args[0]["uri"]

            # Extract resource name from kwargs, response, or derive from URI
            if "resource_name" in kwargs:
                resource_name = kwargs["resource_name"]
            elif "name" in kwargs and "resource" in endpoint.lower():
                resource_name = kwargs["name"]
            elif hasattr(response, "name"):
                resource_name = response.name
            elif resource_uri:
                # Derive resource name from URI (e.g., "resource://config" -> "config")
                resource_name = (
                    resource_uri.split("://")[-1].split("/")[-1]
                    if "://" in resource_uri
                    else resource_uri
                )

            # Try to extract prompt name from various sources
            if "prompt_name" in kwargs:
                prompt_name = kwargs["prompt_name"]
            elif "name" in kwargs and "prompt" in endpoint.lower():
                prompt_name = kwargs["name"]
            elif len(ctx.args) > 0 and "prompt" in endpoint.lower():
                # For MCP prompt operations, name is typically the first argument
                if isinstance(ctx.args[0], str):
                    prompt_name = ctx.args[0]
                elif hasattr(ctx.args[0], "name"):
                    prompt_name = ctx.args[0].name
                elif isinstance(ctx.args[0], dict) and "name" in ctx.args[0]:
                    prompt_name = ctx.args[0]["name"]

            # Calculate request/response sizes
            request_size = None
            response_size = None
            try:
                if len(ctx.args) > 0:
                    request_size = len(serialize_mcp_input(ctx.args[0]))
                if response:
                    response_size = len(serialize_mcp_input(response))
            except Exception:
                pass

            record_mcp_metrics(
                metrics=metrics,
                mcp_operation=mcp_operation,
                mcp_method=mcp_method,
                mcp_transport_type="stdio",  # Default transport
                mcp_tool_name=tool_name,
                mcp_resource_uri=resource_uri,
                mcp_resource_name=resource_name,  # Add resource name for enhanced BI
                mcp_prompt_name=prompt_name,
                environment=ctx.environment,
                application_name=ctx.application_name,
                start_time=start_time,
                end_time=end_time,
                request_size=request_size,
                response_size=response_size,
                is_error=False,
            )

    except Exception as e:
        handle_exception(span, e)

    return response


def create_mcp_scope(
    instance,
    args,
    kwargs,
    version,
    environment,
    application_name,
    pricing_info,
    capture_message_content,
    span,
    start_time: float,
):
    """
    Create optimized scope object for MCP operations following OpenLIT patterns.
    """

    # Create scope object using OpenLIT pattern
    scope = type("MCPScope", (), {})()
    scope._span = span
    scope._start_time = start_time
    scope._end_time = None

    # Create instrumentation context
    scope._context = MCPInstrumentationContext(
        instance=instance,
        args=args,
        kwargs=kwargs,
        version=version,
        environment=environment,
        application_name=application_name,
        pricing_info=pricing_info,
        capture_message_content=capture_message_content,
    )

    return scope


def serialize_mcp_input(request, depth=0, max_depth=4):
    """
    Serialize input args to MCP server into JSON.
    The function accepts input object and converts into JSON
    keeping depth in mind to prevent creating large nested JSON.
    """
    if depth > max_depth:
        return {}
    depth += 1

    def is_serializable(request):
        try:
            json.dumps(request)
            return True
        except Exception:
            return False

    if is_serializable(request):
        return json.dumps(request)

    # Handle complex objects
    if hasattr(request, "model_dump_json"):
        return request.model_dump_json()

    if not hasattr(request, "__dict__"):
        return json.dumps({})

    result = {}
    try:
        for attrib in request.__dict__:
            if attrib.startswith("_"):
                continue

            attr_value = request.__dict__[attrib]
            if type(attr_value) in [bool, str, int, float, type(None)]:
                result[str(attrib)] = attr_value
            else:
                result[str(attrib)] = serialize_mcp_input(attr_value, depth)
    except Exception:
        pass
    return json.dumps(result)


def inject_context_into_jsonrpc_request(request_data):
    """Inject OpenTelemetry context into JSONRPC request _meta field."""
    try:
        if hasattr(request_data, "params"):
            if not request_data.params:
                request_data.params = {}
            meta = request_data.params.setdefault("_meta", {})
            propagate.get_global_textmap().inject(meta)
    except Exception:
        pass  # Silently fail to avoid breaking requests

    return request_data


def extract_context_from_jsonrpc_request(request_data):
    """Extract OpenTelemetry context from JSONRPC request _meta field."""
    try:
        if (
            hasattr(request_data, "params")
            and request_data.params
            and isinstance(request_data.params, dict)
        ):
            meta = request_data.params.get("_meta")
            if meta:
                return propagate.extract(meta)
    except Exception:
        pass

    return None


def _extract_server_context(args):
    """Extract context from JSONRPC request objects in server-side operations."""
    try:
        # Look for JSONRPC request objects and extract context
        for arg in args:
            if hasattr(arg, "root") and hasattr(arg.root, "method"):
                extracted_ctx = extract_context_from_jsonrpc_request(arg.root)
                if extracted_ctx:
                    return extracted_ctx
    except Exception:
        pass
    return None


def create_context_propagating_wrapper(
    gen_ai_endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """Create a wrapper that handles context propagation for transport methods."""

    def wrapper(wrapped, instance, args, kwargs):
        """Wrapper that injects/extracts context for MCP transport operations."""

        # For outgoing requests (client side), inject context
        if "client" in gen_ai_endpoint.lower():
            try:
                # Look for JSONRPC request objects in args/kwargs and inject context
                for arg in args:
                    if hasattr(arg, "root") and hasattr(arg.root, "method"):
                        inject_context_into_jsonrpc_request(arg.root)
                        break
            except Exception:
                pass

        # For incoming requests (server side), extract and attach context
        elif "server" in gen_ai_endpoint.lower():
            extracted_ctx = _extract_server_context(args)
            if extracted_ctx:
                token = context.attach(extracted_ctx)
                try:
                    return wrapped(*args, **kwargs)
                finally:
                    context.detach(token)

        # Default execution without context manipulation
        return wrapped(*args, **kwargs)

    return wrapper


def create_jsonrpc_wrapper(
    endpoint,
    version,
    environment,
    application_name,
    tracer,
    pricing_info,
    capture_message_content,
    metrics,
    disable_metrics,
):
    """
    This wraps BaseSession.send_request to get proper async handling.
    """

    def wrapper(wrapped, instance, args, kwargs):
        async def async_wrapper(*args, **kwargs):
            # Extract method and params from JSONRPC request
            method = None
            params = None
            if (
                len(args) > 0
                and hasattr(args[0], "root")
                and hasattr(args[0].root, "method")
            ):
                method = args[0].root.method
            if (
                len(args) > 0
                and hasattr(args[0], "root")
                and hasattr(args[0].root, "params")
            ):
                params = args[0].root.params
            if params and hasattr(params, "meta"):
                pass  # Meta available but not currently used

            # Create span name following operation_type operation_name convention
            span_name = f"mcp {method}" if method else "mcp send_request"

            with tracer.start_as_current_span(span_name) as span:
                if capture_message_content and len(args) > 0:
                    input_serialized = serialize_mcp_input(args[0])
                    span.set_attribute(
                        SemanticConvention.MCP_REQUEST_PAYLOAD, input_serialized
                    )

                # Set method attribute
                if method:
                    span.set_attribute(SemanticConvention.MCP_METHOD, method)

                # Set OpenLIT business intelligence attributes
                span.set_attribute(SemanticConvention.MCP_SYSTEM, "mcp")
                span.set_attribute(SemanticConvention.MCP_SDK_VERSION, version)
                if environment:
                    span.set_attribute(DEPLOYMENT_ENVIRONMENT, environment)
                if application_name:
                    span.set_attribute(SERVICE_NAME, application_name)

                start_time = time.time()

                try:
                    # CRITICAL: Properly await the response
                    result = await wrapped(*args, **kwargs)

                    if result and capture_message_content:
                        output_serialized = serialize_mcp_input(result)
                        span.set_attribute(
                            SemanticConvention.MCP_RESPONSE_PAYLOAD, output_serialized
                        )

                    # Set success status
                    if (
                        hasattr(result, "isError")
                        and result.isError
                        and (hasattr(result, "content") and len(result.content) > 0)
                    ):
                        span.set_status(
                            Status(StatusCode.ERROR, f"{result.content[0].text}")
                        )

                    # OpenLIT business intelligence
                    end_time = time.time()
                    duration = end_time - start_time
                    span.set_attribute(
                        SemanticConvention.MCP_CLIENT_OPERATION_DURATION, duration
                    )

                    # Record MCP-specific metrics for business intelligence
                    if not disable_metrics and metrics:
                        # Extract tool, resource, and prompt info from method and params
                        tool_name = None
                        resource_uri = None
                        prompt_name = None

                        if method and method.startswith("tools/call") and params:
                            tool_name = (
                                str(getattr(params, "name", None))
                                if getattr(params, "name", None)
                                else None
                            )
                        elif method and method.startswith("resources/read") and params:
                            resource_uri = (
                                str(getattr(params, "uri", None))
                                if getattr(params, "uri", None)
                                else None
                            )
                        elif method and method.startswith("prompts/get") and params:
                            prompt_name = (
                                str(getattr(params, "name", None))
                                if getattr(params, "name", None)
                                else None
                            )

                        # Calculate payload sizes
                        request_size = (
                            len(serialize_mcp_input(args[0])) if len(args) > 0 else None
                        )
                        response_size = (
                            len(serialize_mcp_input(result)) if result else None
                        )

                        record_mcp_metrics(
                            metrics=metrics,
                            mcp_operation=method or "send_request",
                            mcp_method=method or "unknown",
                            mcp_transport_type="stdio",  # Default, could be extracted from context
                            mcp_tool_name=tool_name,
                            mcp_resource_uri=resource_uri,
                            mcp_resource_name=None,  # Not extracted in JSONRPC wrapper context
                            mcp_prompt_name=prompt_name,
                            environment=environment,
                            application_name=application_name,
                            start_time=start_time,
                            end_time=end_time,
                            request_size=request_size,
                            response_size=response_size,
                            is_error=False,
                        )

                    return result

                except Exception as e:
                    handle_exception(span, e)

                    # Record error metrics
                    if not disable_metrics and metrics:
                        end_time = time.time()

                        # Extract method info for error metrics
                        tool_name = None
                        resource_uri = None
                        prompt_name = None

                        if method and method.startswith("tools/call") and params:
                            tool_name = (
                                str(getattr(params, "name", None))
                                if getattr(params, "name", None)
                                else None
                            )
                        elif method and method.startswith("resources/read") and params:
                            resource_uri = (
                                str(getattr(params, "uri", None))
                                if getattr(params, "uri", None)
                                else None
                            )
                        elif method and method.startswith("prompts/get") and params:
                            prompt_name = (
                                str(getattr(params, "name", None))
                                if getattr(params, "name", None)
                                else None
                            )

                        request_size = (
                            len(serialize_mcp_input(args[0])) if len(args) > 0 else None
                        )

                        record_mcp_metrics(
                            metrics=metrics,
                            mcp_operation=method or "send_request",
                            mcp_method=method or "unknown",
                            mcp_transport_type="stdio",
                            mcp_tool_name=tool_name,
                            mcp_resource_uri=resource_uri,
                            mcp_resource_name=None,  # Not extracted in JSONRPC wrapper context
                            mcp_prompt_name=prompt_name,
                            environment=environment,
                            application_name=application_name,
                            start_time=start_time,
                            end_time=end_time,
                            request_size=request_size,
                            response_size=None,
                            is_error=True,
                        )

                    raise

        # Handle both sync and async contexts
        if inspect.iscoroutinefunction(wrapped):
            return async_wrapper(*args, **kwargs)
        else:
            # For sync calls, create an async wrapper and run it
            import asyncio

            try:
                asyncio.get_running_loop()
                # We're in an async context
                return asyncio.create_task(async_wrapper(*args, **kwargs))
            except RuntimeError:
                # No running loop
                return asyncio.run(async_wrapper(*args, **kwargs))

    return wrapper


def _extract_enhanced_mcp_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, response: Any = None, **kwargs
):
    """Extract all new enhanced MCP attributes based on endpoint and context data."""
    try:
        # Extract FastMCP Framework Attributes
        _extract_fastmcp_attributes(span, ctx, endpoint, **kwargs)

        # Extract Authentication & Security Attributes
        _extract_auth_security_attributes(span, ctx, endpoint, **kwargs)

        # Extract Advanced Session Attributes
        _extract_session_attributes(span, ctx, endpoint, **kwargs)

        # Extract WebSocket Attributes
        _extract_websocket_attributes(span, ctx, endpoint, **kwargs)

        # Extract Performance & Reliability Attributes
        _extract_performance_attributes(span, ctx, endpoint, response, **kwargs)

        # Extract Manager-Level Attributes
        _extract_manager_attributes(span, ctx, endpoint, **kwargs)

        # Extract Memory & Progress Attributes
        _extract_memory_progress_attributes(span, ctx, endpoint, **kwargs)

        # Extract Completion Attributes
        _extract_completion_attributes(span, ctx, endpoint, response, **kwargs)

        # Extract Advanced Operation Attributes
        _extract_advanced_operation_attributes(span, ctx, endpoint, response, **kwargs)

    except Exception:
        # Silently ignore attribute extraction errors to prevent breaking instrumentation
        pass


def _extract_fastmcp_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract FastMCP framework-specific attributes."""
    try:
        if "fastmcp" in endpoint:
            # Extract from instance if available
            instance = kwargs.get("instance")
            if instance and hasattr(instance, "settings"):
                settings = instance.settings
                span.set_attribute(
                    SemanticConvention.MCP_FASTMCP_SERVER_DEBUG_MODE,
                    str(getattr(settings, "debug", False)),
                )
                span.set_attribute(
                    SemanticConvention.MCP_FASTMCP_SERVER_LOG_LEVEL,
                    str(getattr(settings, "log_level", "INFO")),
                )
                span.set_attribute(
                    SemanticConvention.MCP_FASTMCP_SERVER_HOST,
                    str(getattr(settings, "host", "127.0.0.1")),
                )
                span.set_attribute(
                    SemanticConvention.MCP_FASTMCP_SERVER_PORT,
                    str(getattr(settings, "port", 8000)),
                )
                span.set_attribute(
                    SemanticConvention.MCP_FASTMCP_SERVER_TRANSPORT,
                    str(getattr(settings, "transport", "stdio")),
                )

                # Additional FastMCP settings
                if hasattr(settings, "json_response"):
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_JSON_RESPONSE,
                        str(settings.json_response),
                    )
                if hasattr(settings, "stateless_http"):
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_STATELESS_HTTP,
                        str(settings.stateless_http),
                    )
                if hasattr(settings, "mount_path"):
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_MOUNT_PATH,
                        str(settings.mount_path),
                    )

            # Extract tool annotations if available
            if "add_tool" in endpoint or "call_tool" in endpoint:
                annotations = kwargs.get("annotations")
                if annotations:
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_TOOL_ANNOTATIONS,
                        json.dumps(annotations, default=str),
                    )

                structured_output = kwargs.get("structured_output")
                if structured_output is not None:
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_TOOL_STRUCTURED_OUTPUT,
                        str(structured_output),
                    )

            # Extract resource MIME type
            if "add_resource" in endpoint or "read_resource" in endpoint:
                mime_type = kwargs.get("mime_type")
                if mime_type:
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_RESOURCE_MIME_TYPE,
                        str(mime_type),
                    )

            # Extract prompt arguments
            if "add_prompt" in endpoint or "get_prompt" in endpoint:
                arguments = kwargs.get("arguments")
                if arguments:
                    span.set_attribute(
                        SemanticConvention.MCP_FASTMCP_PROMPT_ARGUMENTS,
                        json.dumps(arguments, default=str),
                    )

    except Exception:
        pass


def _extract_auth_security_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract authentication and security-related attributes."""
    try:
        if "auth" in endpoint:
            # Extract client information
            client = kwargs.get("client")
            if client:
                if hasattr(client, "client_id"):
                    span.set_attribute(
                        SemanticConvention.MCP_AUTH_CLIENT_ID, str(client.client_id)
                    )
                if hasattr(client, "grant_types"):
                    span.set_attribute(
                        SemanticConvention.MCP_AUTH_GRANT_TYPE,
                        json.dumps(client.grant_types, default=str),
                    )

            # Extract authorization parameters
            params = kwargs.get("params")
            if params and hasattr(params, "scopes"):
                span.set_attribute(
                    SemanticConvention.MCP_AUTH_SCOPES,
                    json.dumps(params.scopes, default=str),
                )
            if params and hasattr(params, "state"):
                span.set_attribute(SemanticConvention.MCP_AUTH_STATE, str(params.state))
            if params and hasattr(params, "code_challenge"):
                span.set_attribute(
                    SemanticConvention.MCP_AUTH_CODE_CHALLENGE,
                    str(params.code_challenge),
                )
            if params and hasattr(params, "redirect_uri"):
                span.set_attribute(
                    SemanticConvention.MCP_AUTH_REDIRECT_URI, str(params.redirect_uri)
                )

            # Extract token information
            token = (
                kwargs.get("token")
                or kwargs.get("access_token")
                or kwargs.get("refresh_token")
            )
            if token:
                if hasattr(token, "token_type"):
                    span.set_attribute(
                        SemanticConvention.MCP_AUTH_TOKEN_TYPE, str(token.token_type)
                    )
                if hasattr(token, "expires_at"):
                    span.set_attribute(
                        SemanticConvention.MCP_AUTH_EXPIRES_AT, str(token.expires_at)
                    )
                if hasattr(token, "scopes"):
                    span.set_attribute(
                        SemanticConvention.MCP_AUTH_SCOPES,
                        json.dumps(token.scopes, default=str),
                    )

        # Extract transport security settings
        if "transport" in endpoint:
            instance = kwargs.get("instance")
            if instance and hasattr(instance, "security_settings"):
                span.set_attribute(
                    SemanticConvention.MCP_SECURITY_TRANSPORT_SECURITY, "enabled"
                )

    except Exception:
        pass


def _extract_session_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract advanced session-related attributes."""
    try:
        # Extract session configuration
        if "session" in endpoint or "initialize" in endpoint:
            # Extract timeout settings
            read_timeout = kwargs.get("read_timeout_seconds")
            if read_timeout:
                span.set_attribute(
                    SemanticConvention.MCP_SESSION_READ_TIMEOUT,
                    str(read_timeout.total_seconds()),
                )

            request_timeout = kwargs.get("request_read_timeout_seconds")
            if request_timeout:
                span.set_attribute(
                    SemanticConvention.MCP_SESSION_REQUEST_TIMEOUT,
                    str(request_timeout.total_seconds()),
                )

            # Extract capabilities
            capabilities = kwargs.get("capabilities")
            if capabilities:
                if hasattr(capabilities, "sampling"):
                    span.set_attribute(
                        SemanticConvention.MCP_SESSION_SAMPLING_SUPPORT,
                        str(capabilities.sampling is not None),
                    )
                if hasattr(capabilities, "elicitation"):
                    span.set_attribute(
                        SemanticConvention.MCP_SESSION_ELICITATION_SUPPORT,
                        str(capabilities.elicitation is not None),
                    )
                if hasattr(capabilities, "roots"):
                    span.set_attribute(
                        SemanticConvention.MCP_SESSION_ROOTS_SUPPORT,
                        str(capabilities.roots is not None),
                    )

            # Extract client info
            client_info = kwargs.get("clientInfo") or kwargs.get("client_info")
            if client_info:
                if hasattr(client_info, "name"):
                    span.set_attribute(
                        SemanticConvention.MCP_SESSION_CLIENT_INFO_NAME,
                        str(client_info.name),
                    )
                if hasattr(client_info, "version"):
                    span.set_attribute(
                        SemanticConvention.MCP_SESSION_CLIENT_INFO_VERSION,
                        str(client_info.version),
                    )

            # Extract server configuration flags
            stateless = kwargs.get("stateless")
            if stateless is not None:
                span.set_attribute(
                    SemanticConvention.MCP_SESSION_STATELESS, str(stateless)
                )

            raise_exceptions = kwargs.get("raise_exceptions")
            if raise_exceptions is not None:
                span.set_attribute(
                    SemanticConvention.MCP_SESSION_RAISE_EXCEPTIONS,
                    str(raise_exceptions),
                )

        # Extract progress token
        progress_token = kwargs.get("progress_token") or (
            kwargs.get("params", {}).get("_meta", {}).get("progressToken")
        )
        if progress_token:
            span.set_attribute(
                SemanticConvention.MCP_SESSION_PROGRESS_TOKEN, str(progress_token)
            )

    except Exception:
        pass


def _extract_websocket_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract WebSocket-specific attributes."""
    try:
        if "websocket" in endpoint:
            # Extract WebSocket URL
            url = kwargs.get("url")
            if url:
                span.set_attribute(SemanticConvention.MCP_WEBSOCKET_URL, str(url))

            # Extract subprotocol (typically "mcp")
            subprotocols = kwargs.get("subprotocols")
            if subprotocols:
                span.set_attribute(
                    SemanticConvention.MCP_WEBSOCKET_SUBPROTOCOL,
                    json.dumps(subprotocols, default=str),
                )
            elif "websocket" in endpoint:
                # Default MCP WebSocket subprotocol
                span.set_attribute(SemanticConvention.MCP_WEBSOCKET_SUBPROTOCOL, "mcp")

    except Exception:
        pass


def _extract_performance_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, response: Any = None, **kwargs
):
    """Extract performance and reliability metrics."""
    try:
        # Calculate execution time if we have timing data
        if hasattr(ctx, "_start_time") and hasattr(ctx, "_end_time"):
            execution_time = ctx._end_time - ctx._start_time

            if "call_tool" in endpoint:
                span.set_attribute(
                    SemanticConvention.MCP_TOOL_EXECUTION_TIME, execution_time
                )
            elif "read_resource" in endpoint:
                span.set_attribute(
                    SemanticConvention.MCP_RESOURCE_READ_TIME, execution_time
                )
            elif "render_prompt" in endpoint or "get_prompt" in endpoint:
                span.set_attribute(
                    SemanticConvention.MCP_PROMPT_RENDER_TIME, execution_time
                )
            elif "transport" in endpoint:
                span.set_attribute(
                    SemanticConvention.MCP_TRANSPORT_CONNECTION_TIME, execution_time
                )

        # Extract progress information
        if "progress" in endpoint:
            progress = kwargs.get("progress")
            if progress is not None:
                span.set_attribute(
                    SemanticConvention.MCP_PROGRESS_COMPLETION_PERCENTAGE, str(progress)
                )

            total = kwargs.get("total")
            if total is not None:
                span.set_attribute(SemanticConvention.MCP_PROGRESS_TOTAL, str(total))

            message = kwargs.get("message")
            if message:
                span.set_attribute(
                    SemanticConvention.MCP_PROGRESS_MESSAGE, str(message)
                )

        # Extract elicitation action
        if "elicit" in endpoint and response:
            if hasattr(response, "action"):
                span.set_attribute(
                    SemanticConvention.MCP_ELICITATION_ACTION, str(response.action)
                )

        # Extract sampling parameters
        if "sampling" in endpoint or "create_message" in endpoint:
            max_tokens = kwargs.get("max_tokens")
            if max_tokens:
                span.set_attribute(
                    SemanticConvention.MCP_SAMPLING_MAX_TOKENS, str(max_tokens)
                )

            messages = kwargs.get("messages")
            if messages:
                span.set_attribute(
                    SemanticConvention.MCP_SAMPLING_MESSAGES, str(len(messages))
                )

    except Exception:
        pass


def _extract_manager_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract manager-level attributes for business intelligence."""
    try:
        if "manager" in endpoint:
            instance = kwargs.get("instance")
            if instance:
                # Extract manager type and operation count
                if "tool_manager" in endpoint:
                    span.set_attribute(SemanticConvention.MCP_MANAGER_TYPE, "tool")
                    if hasattr(instance, "_tools"):
                        span.set_attribute(
                            SemanticConvention.MCP_TOOL_MANAGER_TOOL_COUNT,
                            str(len(instance._tools)),
                        )
                    if hasattr(instance, "warn_on_duplicate_tools"):
                        span.set_attribute(
                            SemanticConvention.MCP_TOOL_MANAGER_WARN_DUPLICATES,
                            str(instance.warn_on_duplicate_tools),
                        )

                elif "resource_manager" in endpoint:
                    span.set_attribute(SemanticConvention.MCP_MANAGER_TYPE, "resource")
                    if hasattr(instance, "_resources"):
                        span.set_attribute(
                            SemanticConvention.MCP_RESOURCE_MANAGER_RESOURCE_COUNT,
                            str(len(instance._resources)),
                        )
                    if hasattr(instance, "warn_on_duplicate_resources"):
                        span.set_attribute(
                            SemanticConvention.MCP_RESOURCE_MANAGER_WARN_DUPLICATES,
                            str(instance.warn_on_duplicate_resources),
                        )

                elif "prompt_manager" in endpoint:
                    span.set_attribute(SemanticConvention.MCP_MANAGER_TYPE, "prompt")
                    if hasattr(instance, "_prompts"):
                        span.set_attribute(
                            SemanticConvention.MCP_PROMPT_MANAGER_PROMPT_COUNT,
                            str(len(instance._prompts)),
                        )
                    if hasattr(instance, "warn_on_duplicate_prompts"):
                        span.set_attribute(
                            SemanticConvention.MCP_PROMPT_MANAGER_WARN_DUPLICATES,
                            str(instance.warn_on_duplicate_prompts),
                        )

    except Exception:
        pass


def _extract_memory_progress_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, **kwargs
):
    """Extract memory transport and progress context attributes."""
    try:
        if "memory" in endpoint:
            span.set_attribute(SemanticConvention.MCP_MEMORY_TRANSPORT_TYPE, "memory")

            # Extract connection session info
            if "connect" in endpoint:
                span.set_attribute(
                    SemanticConvention.MCP_MEMORY_CLIENT_SERVER_SESSION, "created"
                )

        if "progress_context" in endpoint or "ProgressContext" in str(
            type(kwargs.get("instance", ""))
        ):
            instance = kwargs.get("instance")
            if instance:
                if hasattr(instance, "current"):
                    span.set_attribute(
                        SemanticConvention.MCP_PROGRESS_CONTEXT_CURRENT,
                        str(instance.current),
                    )
                if hasattr(instance, "total"):
                    span.set_attribute(
                        SemanticConvention.MCP_PROGRESS_CONTEXT_TOTAL,
                        str(instance.total),
                    )

    except Exception:
        pass


def _extract_completion_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, response: Any = None, **kwargs
):
    """Extract completion-related attributes."""
    try:
        if "complete" in endpoint:
            # Extract completion request parameters
            ref = kwargs.get("ref")
            if ref:
                if hasattr(ref, "type"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_REF_TYPE, str(ref.type)
                    )

            argument = kwargs.get("argument")
            if argument:
                if hasattr(argument, "name"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_ARGUMENT_NAME,
                        str(argument.name),
                    )
                if hasattr(argument, "value"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_ARGUMENT_VALUE,
                        str(argument.value),
                    )

            context_args = kwargs.get("context_arguments")
            if context_args:
                span.set_attribute(
                    SemanticConvention.MCP_COMPLETION_CONTEXT_ARGUMENTS,
                    json.dumps(context_args, default=str),
                )

            # Extract completion response
            if response and hasattr(response, "completion"):
                completion = response.completion
                if hasattr(completion, "values"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_VALUES,
                        str(len(completion.values)),
                    )
                if hasattr(completion, "total"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_TOTAL, str(completion.total)
                    )
                if hasattr(completion, "hasMore"):
                    span.set_attribute(
                        SemanticConvention.MCP_COMPLETION_HAS_MORE,
                        str(completion.hasMore),
                    )

    except Exception:
        pass


def _extract_advanced_operation_attributes(
    span, ctx: MCPInstrumentationContext, endpoint: str, response: Any = None, **kwargs
):
    """Extract advanced operation-specific attributes."""
    try:
        # Extract ping response time
        if "ping" in endpoint:
            if hasattr(ctx, "_start_time") and hasattr(ctx, "_end_time"):
                response_time = ctx._end_time - ctx._start_time
                span.set_attribute(
                    SemanticConvention.MCP_PING_RESPONSE_TIME, response_time
                )

        # Extract logging level changes
        if "set_logging" in endpoint:
            level = kwargs.get("level")
            if level:
                span.set_attribute(SemanticConvention.MCP_LOGGING_LEVEL_SET, str(level))

        # Extract notification details
        if "notification" in endpoint:
            notification_type = "progress" if "progress" in endpoint else "general"
            span.set_attribute(
                SemanticConvention.MCP_NOTIFICATION_TYPE, notification_type
            )

            related_request_id = kwargs.get("related_request_id")
            if related_request_id:
                span.set_attribute(
                    SemanticConvention.MCP_NOTIFICATION_RELATED_REQUEST_ID,
                    str(related_request_id),
                )

    except Exception:
        pass
