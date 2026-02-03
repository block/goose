"""
Provider integrations for model services.

This package provides integrations for different provider services
including OpenAI, AWS Bedrock, and Google Vertex AI.

The main entry point is the factory functions:
- get_provider() - Get a provider instance
- get_chat_model() - Get a ready-to-use chat model
"""

from src.integrations.providers.factory import get_chat_model, get_provider

__all__ = [
    "get_provider",
    "get_chat_model",
]
