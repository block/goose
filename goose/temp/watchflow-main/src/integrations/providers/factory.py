"""
Provider Factory.

This module provides factory functions to create the appropriate
provider based on configuration using a simple mapping approach.
"""

from __future__ import annotations

from typing import Any

from src.core.config import config
from src.integrations.providers.base import BaseProvider
from src.integrations.providers.bedrock_provider import BedrockProvider
from src.integrations.providers.openai_provider import OpenAIProvider
from src.integrations.providers.vertex_ai_provider import VertexAIProvider

# Provider mapping - canonical names to provider classes
PROVIDER_MAP: dict[str, type[BaseProvider]] = {
    "openai": OpenAIProvider,
    "bedrock": BedrockProvider,
    "vertex_ai": VertexAIProvider,
    # Legacy aliases for backward compatibility
    "garden": VertexAIProvider,
    "model_garden": VertexAIProvider,
    "gcp": VertexAIProvider,
    "vertex": VertexAIProvider,
    "vertexai": VertexAIProvider,
}


def get_provider(
    provider: str | None = None,
    model: str | None = None,
    max_tokens: int | None = None,
    temperature: float | None = None,
    agent: str | None = None,
    **kwargs: Any,
) -> BaseProvider:
    """
    Get the appropriate provider based on configuration.

    Args:
        provider: Provider name (openai, bedrock, vertex_ai)
        model: Model name/ID
        max_tokens: Maximum tokens to generate
        temperature: Sampling temperature
        agent: Agent name for per-agent configuration
        **kwargs: Additional provider-specific parameters

    Returns:
        Configured provider instance

    Raises:
        ValueError: If provider is not supported
    """
    # Use config defaults if not provided
    provider_name = provider or config.ai.provider or "openai"
    provider_name = provider_name.lower()

    # Get provider class from mapping
    provider_class = PROVIDER_MAP.get(provider_name)
    if not provider_class:
        supported = ", ".join(
            sorted(set(PROVIDER_MAP.keys()) - {"garden", "model_garden", "gcp", "vertex", "vertexai"})
        )
        raise ValueError(f"Unsupported provider: {provider_name}. Supported: {supported}")

    # Get model with fallbacks handled by config
    if not model:
        # Normalize provider name for config lookup (use canonical name)
        canonical_provider = (
            "vertex_ai" if provider_name in ["garden", "model_garden", "gcp", "vertex", "vertexai"] else provider_name
        )
        model = config.ai.get_model_for_provider(canonical_provider)

    # Determine tokens and temperature with precedence: explicit params > agent config > global config
    tokens = max_tokens if max_tokens is not None else config.ai.get_max_tokens_for_agent(agent)
    temp = temperature if temperature is not None else config.ai.get_temperature_for_agent(agent)

    # Prepare provider-specific kwargs
    provider_kwargs = kwargs.copy()

    # Add provider-specific config
    if provider_class == OpenAIProvider:
        provider_kwargs["api_key"] = config.ai.api_key

    # Instantiate provider
    return provider_class(
        model=model,
        max_tokens=tokens,
        temperature=temp,
        **provider_kwargs,
    )


def get_chat_model(
    provider: str | None = None,
    model: str | None = None,
    max_tokens: int | None = None,
    temperature: float | None = None,
    agent: str | None = None,
    **kwargs: Any,
) -> Any:
    """
    Get a chat model instance using the appropriate provider.

    This is a convenience function that creates a provider and returns its chat model.

    Args:
        provider: Provider name (openai, bedrock, vertex_ai)
        model: Model name/ID
        max_tokens: Maximum tokens to generate
        temperature: Sampling temperature
        agent: Agent name for per-agent configuration
        **kwargs: Additional provider-specific parameters

    Returns:
        Ready-to-use chat model instance
    """
    provider_instance = get_provider(
        provider=provider,
        model=model,
        max_tokens=max_tokens,
        temperature=temperature,
        agent=agent,
        **kwargs,
    )

    return provider_instance.get_chat_model()


# Backward compatibility aliases
BaseAIProvider = BaseProvider
