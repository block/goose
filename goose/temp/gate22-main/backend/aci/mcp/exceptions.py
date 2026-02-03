from fastapi import status


class RemoteMCPException(Exception):  # noqa: N818
    """
    Base class for all Remote MCP exceptions with consistent structure.

    Attributes:
        title (str): error title.
        message (str): an optional detailed error message.
        error_code (int): HTTP status code to identify the error type.
    """

    def __init__(
        self,
        title: str,
        message: str | None = None,
        error_code: int = status.HTTP_400_BAD_REQUEST,
    ):
        super().__init__(title, message, error_code)
        self.title = title
        self.message = message
        self.error_code = error_code

    def __str__(self) -> str:
        """
        String representation that combines title and message (if available)
        """
        if self.message:
            return f"{self.title}: {self.message}"
        return self.title


class OAuth2Error(RemoteMCPException):
    """
    Exception raised when an OAuth2 error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="OAuth2 error",
            message=message,
            error_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        )


class NoImplementationFound(RemoteMCPException):
    """
    Exception raised when a feature or function is not implemented
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="No implementation found",
            message=message,
            error_code=status.HTTP_501_NOT_IMPLEMENTED,
        )


class MCPServerConfigurationNotFound(RemoteMCPException):
    """
    Exception raised when an mcp server configuration is not found
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="MCP server configuration not found",
            message=message,
            error_code=status.HTTP_404_NOT_FOUND,
        )


class NotPermittedError(RemoteMCPException):
    """
    Exception raised when a user is not permitted to act as the requested organization and role.
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="Not permitted",
            message=message,
            error_code=status.HTTP_403_FORBIDDEN,
        )


class UnexpectedError(RemoteMCPException):
    """
    Exception raised when an unexpected error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="Unexpected error",
            message=message,
            error_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        )


class MCPToolNotFound(RemoteMCPException):
    """
    Exception raised when an mcp tool is not found
    """

    def __init__(self, name: str):
        super().__init__(title="MCP tool not found", message=f"MCP tool {name} not found")


class MCPServerNotConfigured(RemoteMCPException):
    """
    Exception raised when an mcp server is not configured
    """

    def __init__(self, mcp_server_name: str):
        super().__init__(
            title="MCP server not configured",
            message=f"MCP server {mcp_server_name} not configured",
        )


class MCPToolNotEnabled(RemoteMCPException):
    """
    Exception raised when an mcp tool is not enabled in the mcp server configuration
    """

    def __init__(self, tool_name: str):
        super().__init__(title="MCP tool not enabled", message=f"MCP tool {tool_name} not enabled")


class UnsupportedJSONRPCMethodError(RemoteMCPException):
    def __init__(self, method: str, id: int | str | None = None):
        super().__init__(title="Unsupported jsonrpc method", message=method)
        self.method = method
        self.id = id


class InvalidJSONRPCPayloadError(RemoteMCPException):
    def __init__(self, message: str | None = None, id: int | str | None = None):
        super().__init__(title="Invalid jsonrpc payload", message=message)
        self.id = id


class UsageExceeded(RemoteMCPException):
    """
    Exception raised when a usage exceeds the entitlement
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="Usage exceeded",
            message=message,
            error_code=status.HTTP_429_TOO_MANY_REQUESTS,
        )
