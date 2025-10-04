"""
Goose Python Client - Simple and Powerful

Basic usage:
    from goose_client import GooseClient

    client = GooseClient(api_key="your-key")
    response = client.chat("Hello!")
    print(response)

Advanced usage:
    from goose_client import GooseClient
    from goose_client.extensions import with_retry

    client = with_retry(GooseClient(api_key="your-key"))
"""

from .client import GooseClient
from .core import BaseClient

__version__ = "2.0.0"
__all__ = ["GooseClient", "BaseClient"]

# Make generated models available for advanced users
from ._generated._generated.models import (
    ChatRequest,
    Content,
    Message,
    Session,
    StartAgentRequest,
    Tool,
    ToolRequest,
    ToolResponse,
)

# Re-export commonly used models
__all__.extend(
    [
        "Message",
        "Session",
        "StartAgentRequest",
        "Content",
        "ChatRequest",
        "Tool",
        "ToolRequest",
        "ToolResponse",
    ]
)
