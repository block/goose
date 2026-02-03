from fastapi import status


class VirtualMCPException(Exception):  # noqa: N818
    """
    Base class for all Virtual MCP exceptions with consistent structure.

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


class UnexpectedError(VirtualMCPException):
    """
    Exception raised when an unexpected error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(
            title="Unexpected error",
            message=message,
            error_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        )


class MCPServerNotFound(VirtualMCPException):
    """
    Exception raised when an mcp server is not found
    """

    def __init__(self, name: str):
        super().__init__(
            title="MCP server not found",
            message=name,
            error_code=status.HTTP_404_NOT_FOUND,
        )
        self.name = name


class MCPToolNotFound(VirtualMCPException):
    """
    Exception raised when an mcp tool is not found
    """

    def __init__(self, name: str):
        super().__init__(
            title="MCP tool not found",
            message=name,
            error_code=status.HTTP_404_NOT_FOUND,
        )
        self.name = name


class UnsupportedJSONRPCMethodError(VirtualMCPException):
    def __init__(self, method: str, id: int | str | None = None):
        super().__init__(title="Unsupported jsonrpc method", message=method)
        self.method = method
        self.id = id


class InvalidJSONRPCPayloadError(VirtualMCPException):
    def __init__(self, message: str | None = None, id: int | str | None = None):
        super().__init__(title="Invalid jsonrpc payload", message=message)
        self.id = id


class InvalidAuthTokenError(VirtualMCPException):
    def __init__(self, message: str | None = None):
        super().__init__(
            title="Invalid auth token",
            message=message,
            error_code=status.HTTP_401_UNAUTHORIZED,
        )
