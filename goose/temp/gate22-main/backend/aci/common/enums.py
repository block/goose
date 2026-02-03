from enum import StrEnum


class UserIdentityProvider(StrEnum):
    GOOGLE = "google"  # google login
    EMAIL = "email"  # email/password login


class UserVerificationType(StrEnum):
    EMAIL_VERIFICATION = "email_verification"
    PASSWORD_RESET = "password_reset"


class OrganizationRole(StrEnum):
    ADMIN = "admin"  # e.g., manager MCP server configuration
    MEMBER = "member"  # e.g., developer who bundle and use MCP servers


class OrganizationInvitationStatus(StrEnum):
    PENDING = "pending"
    ACCEPTED = "accepted"
    REJECTED = "rejected"
    CANCELED = "canceled"


class TeamRole(StrEnum):
    # NOTE: currently we don't really need to differentiate between team members
    # keeping the enum structure for future use
    MEMBER = "member"


class AuthType(StrEnum):
    NO_AUTH = "no_auth"
    API_KEY = "api_key"
    OAUTH2 = "oauth2"


class ConnectedAccountOwnership(StrEnum):
    INDIVIDUAL = "individual"
    SHARED = "shared"
    OPERATIONAL = "operational"


class HttpLocation(StrEnum):
    PATH = "path"
    QUERY = "query"
    HEADER = "header"
    COOKIE = "cookie"
    BODY = "body"


class MCPServerTransportType(StrEnum):
    STREAMABLE_HTTP = "streamable_http"
    SSE = "sse"


class VirtualMCPToolType(StrEnum):
    """
    NOTE: only used by the virtual MCP service
    Same as the "Protocol" enum from the tool-calling platform
    """

    REST = "rest"
    CONNECTOR = "connector"


class HttpMethod(StrEnum):
    """
    NOTE: only used by the virtual MCP service
    Same as the "HttpMethod" enum from the tool-calling platform
    """

    GET = "GET"
    POST = "POST"
    PUT = "PUT"
    DELETE = "DELETE"
    PATCH = "PATCH"
    HEAD = "HEAD"
    OPTIONS = "OPTIONS"


class MCPToolSchemaFormat(StrEnum):
    """
    format for a MCP tool schema.
    """

    OPENAI = "openai"  # for openai chat completions API
    ANTHROPIC = "anthropic"  # for anthropic chat completions API
    OPENAI_RESPONSES = "openai_responses"  # for openai responses API


class Environment(StrEnum):
    LOCAL = "local"
    DEVELOPMENT = "development"
    PRODUCTION = "production"


class MCPToolCallStatus(StrEnum):
    SUCCESS = "SUCCESS"
    ERROR = "ERROR"
