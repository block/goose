"""
Google Vertex AI Provider implementation.

This provider handles Google Cloud Platform Vertex AI (Model Garden) API interactions
for AI model access, supporting both Google (Gemini) and third-party (Claude) models.
All integration logic is consolidated here.
"""

from __future__ import annotations

import base64
import os
import tempfile
from typing import Any

from src.core.config import config
from src.integrations.providers.base import BaseProvider


class VertexAIProvider(BaseProvider):
    """Google Vertex AI Provider (Model Garden)."""

    def get_chat_model(self) -> Any:
        """Get Vertex AI chat model."""
        project_id = config.ai.gcp_project
        location = config.ai.gcp_location or "us-central1"
        service_account_key_base64 = config.ai.gcp_service_account_key_base64

        if not project_id:
            raise ValueError("GCP project ID required for Vertex AI. Set GCP_PROJECT_ID in config")

        # Handle base64 encoded service account key
        if service_account_key_base64:
            try:
                key_data = base64.b64decode(service_account_key_base64).decode("utf-8")
                with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
                    f.write(key_data)
                    credentials_path = f.name
                os.environ["GOOGLE_APPLICATION_CREDENTIALS"] = credentials_path
            except Exception as e:
                raise ValueError(f"Failed to decode GCP service account key: {e}") from e

        # Check if it's a Claude model
        if "claude" in self.model.lower():
            return self._get_claude_client(project_id, location, self.model)
        else:
            return self._get_gemini_client(project_id, location, self.model)

    def supports_structured_output(self) -> bool:
        """Vertex AI supports structured output."""
        return True

    def get_provider_name(self) -> str:
        """Get provider name."""
        return "vertex_ai"

    def _get_claude_client(self, project_id: str, location: str, model: str) -> Any:
        """Get Claude model via Vertex AI using Anthropic Vertex SDK."""
        try:
            from anthropic import AnthropicVertex
        except ImportError as e:
            raise RuntimeError(
                "Claude Vertex AI client requires 'anthropic[vertex]' package. "
                "Install with: pip install 'anthropic[vertex]'"
            ) from e

        client = AnthropicVertex(region=location, project_id=project_id)
        return self._ClaudeVertexWrapper(client, model)

    def _get_gemini_client(self, project_id: str, location: str, model: str) -> Any:
        """Get Gemini model via Vertex AI using LangChain."""
        try:
            from langchain_google_vertexai import ChatVertexAI
        except ImportError as e:
            raise RuntimeError(
                "Gemini Vertex AI client requires 'langchain-google-vertexai' package. "
                "Install with: pip install langchain-google-vertexai"
            ) from e

        # Try multiple Gemini model names in order of preference
        model_candidates = [model, "gemini-pro", "gemini-1.5-pro", "gemini-1.5-flash"]

        for candidate_model in model_candidates:
            try:
                return ChatVertexAI(
                    model=candidate_model,
                    project=project_id,
                    location=location,
                )
            except Exception as e:
                if "not found" in str(e).lower() or "404" in str(e):
                    continue
                else:
                    raise

        raise RuntimeError(
            f"None of the Gemini models are available in your GCP project. "
            f"Tried: {', '.join(model_candidates)}. "
            f"Please check your GCP project configuration and model access."
        )

    class _ClaudeVertexWrapper:
        """Wrapper for Claude Vertex AI client to match LangChain interface."""

        def __init__(self, client: Any, model: str):
            self.client = client
            self.model = model

        async def ainvoke(self, messages: list[Any], **kwargs: Any) -> Any:
            """Async invoke method."""
            from langchain_core.messages import AIMessage

            anthropic_messages = []
            for msg in messages:
                if hasattr(msg, "content"):
                    content = msg.content
                    role = "user" if msg.type == "human" else "assistant"
                else:
                    content = str(msg)
                    role = "user"

                anthropic_messages.append({"role": role, "content": content})

            response = self.client.messages.create(
                model=self.model,
                messages=anthropic_messages,
                max_tokens=kwargs.get("max_tokens", 4096),
                temperature=kwargs.get("temperature", 0.1),
            )

            return AIMessage(content=response.content[0].text)

        def invoke(self, messages: list[Any], **kwargs: Any) -> Any:
            """Sync invoke method."""
            import asyncio

            return asyncio.run(self.ainvoke(messages, **kwargs))

        def with_structured_output(self, schema: Any, **kwargs: Any) -> Any:
            """Structured output method."""
            self._output_schema = schema
            return self
