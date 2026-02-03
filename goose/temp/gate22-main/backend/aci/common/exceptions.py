class ACICommonError(Exception):
    def __init__(
        self,
        title: str,
        message: str | None = None,
    ):
        super().__init__(title, message)
        self.title = title
        self.message = message

    def __str__(self) -> str:
        """
        String representation that combines title and message (if available)
        """
        if self.message:
            return f"{self.title}: {self.message}"
        return self.title


class OAuth2ManagerError(ACICommonError):
    """
    Exception raised when an OAuth2 manager error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(title="OAuth2 manager error", message=message)


class AuthCredentialsManagerError(ACICommonError):
    """
    Exception raised when an auth credentials manager error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(title="Auth credentials manager error", message=message)


class MCPToolSanitizationError(ACICommonError):
    """
    Exception raised when an MCP tool sanitization error occurs
    """

    def __init__(self, message: str | None = None):
        super().__init__(title="MCP tool sanitization error", message=message)


class OrganizationNotFoundError(ACICommonError):
    """
    Exception raised when an organization is not found
    """

    def __init__(self, message: str | None = None):
        super().__init__(title="Organization not found error", message=message)
