"""
Base Provider interface.

This module defines the abstract base class that all providers must implement.
"""

from abc import ABC, abstractmethod
from typing import Any


class BaseProvider(ABC):
    """Base class for providers."""

    def __init__(self, model: str, max_tokens: int = 4096, temperature: float = 0.1, **kwargs: "Any") -> None:
        self.model = model
        self.max_tokens = max_tokens
        self.temperature = temperature
        self.kwargs = kwargs

    @abstractmethod
    def get_chat_model(self) -> Any:
        """Get the chat model instance."""
        pass

    @abstractmethod
    def supports_structured_output(self) -> bool:
        """Check if this provider supports structured output."""
        pass

    @abstractmethod
    def get_provider_name(self) -> str:
        """Get the provider name."""
        pass

    def get_model_info(self) -> dict[str, Any]:
        """Get model information."""
        return {
            "provider": self.get_provider_name(),
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        }


# Alias for backward compatibility
BaseAIProvider = BaseProvider
