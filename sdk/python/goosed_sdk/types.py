"""
goosed-sdk - Python SDK for goosed API

Type definitions for API objects.
"""

from dataclasses import dataclass, field
from typing import Any, Literal


@dataclass
class MessageMetadata:
    """Metadata for a message."""

    user_visible: bool = True
    agent_visible: bool = True


@dataclass
class MessageContent:
    """Content of a message."""

    type: str
    text: str | None = None
    data: str | None = None
    mime_type: str | None = None
    id: str | None = None
    tool_call: dict[str, Any] | None = None
    tool_result: dict[str, Any] | None = None


@dataclass
class Message:
    """A chat message."""

    role: Literal["user", "assistant"]
    created: int
    content: list[MessageContent]
    metadata: MessageMetadata
    id: str | None = None


@dataclass
class TokenState:
    """Token usage state."""

    input_tokens: int = 0
    output_tokens: int = 0
    total_tokens: int = 0
    accumulated_input_tokens: int = 0
    accumulated_output_tokens: int = 0
    accumulated_total_tokens: int = 0


@dataclass
class ExtensionConfig:
    """Configuration for an extension."""

    type: str
    name: str
    description: str | None = None
    bundled: bool | None = None


@dataclass
class Session:
    """A goosed session."""

    id: str
    name: str
    working_dir: str
    session_type: str
    created_at: str
    updated_at: str
    user_set_name: bool | None = None
    message_count: int | None = None
    total_tokens: int | None = None
    input_tokens: int | None = None
    output_tokens: int | None = None
    provider_name: str | None = None
    conversation: list[dict[str, Any]] | None = None


@dataclass
class ToolInfo:
    """Information about an available tool."""

    name: str
    description: str
    parameters: list[str] = field(default_factory=list)
    permission: str | None = None


@dataclass
class CallToolResponse:
    """Response from calling a tool."""

    content: list[dict[str, Any]]
    is_error: bool


@dataclass
class ExtensionResult:
    """Result of loading an extension."""

    name: str
    success: bool


@dataclass
class SystemInfo:
    """System information."""

    app_version: str
    os: str
    os_version: str
    architecture: str
    provider: str
    model: str
    enabled_extensions: list[str] = field(default_factory=list)


SSEEventType = Literal["Ping", "Message", "Finish", "Error", "ModelChange", "Notification"]


@dataclass
class SSEEvent:
    """Server-Sent Event from the chat stream."""

    type: SSEEventType
    message: dict[str, Any] | None = None
    token_state: TokenState | None = None
    reason: str | None = None
    error: str | None = None


@dataclass
class GoosedClientOptions:
    """Options for GoosedClient."""

    base_url: str = "http://127.0.0.1:3000"
    secret_key: str = "test"
    timeout: float = 30.0
