"""
goosed-sdk - Python SDK for goosed API
"""

from .client import AsyncGoosedClient, GoosedClient
from .exceptions import (
    GoosedAgentNotInitializedError,
    GoosedAuthError,
    GoosedConnectionError,
    GoosedException,
    GoosedNotFoundError,
    GoosedServerError,
)
from .types import (
    CallToolResponse,
    ExtensionConfig,
    ExtensionResult,
    GoosedClientOptions,
    Message,
    MessageContent,
    MessageMetadata,
    Session,
    SSEEvent,
    SSEEventType,
    SystemInfo,
    TokenState,
    ToolInfo,
)

__version__ = "0.1.0"

__all__ = [
    # Clients
    "GoosedClient",
    "AsyncGoosedClient",
    # Exceptions
    "GoosedException",
    "GoosedAuthError",
    "GoosedNotFoundError",
    "GoosedAgentNotInitializedError",
    "GoosedServerError",
    "GoosedConnectionError",
    # Types
    "Session",
    "Message",
    "MessageContent",
    "MessageMetadata",
    "ToolInfo",
    "CallToolResponse",
    "SSEEvent",
    "SSEEventType",
    "TokenState",
    "SystemInfo",
    "ExtensionResult",
    "ExtensionConfig",
    "GoosedClientOptions",
]
