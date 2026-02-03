"""
OpenAI Provider implementation.

This provider handles OpenAI API integration directly.
"""

from typing import Any

from src.integrations.providers.base import BaseProvider


class OpenAIProvider(BaseProvider):
    """OpenAI Provider."""

    def get_chat_model(self) -> Any:
        """Get OpenAI chat model."""
        try:
            from langchain_openai import ChatOpenAI
        except ImportError as e:
            raise RuntimeError(
                "OpenAI provider requires 'langchain-openai' package. Install with: pip install langchain-openai"
            ) from e

        return ChatOpenAI(
            model=self.model,
            # mypy complains about max_tokens but it is valid for ChatOpenAI
            # type: ignore[call-arg]
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            api_key=self.kwargs.get("api_key"),
            **{k: v for k, v in self.kwargs.items() if k != "api_key"},
        )

    def supports_structured_output(self) -> bool:
        """OpenAI supports structured output."""
        return True

    def get_provider_name(self) -> str:
        """Get provider name."""
        return "openai"
