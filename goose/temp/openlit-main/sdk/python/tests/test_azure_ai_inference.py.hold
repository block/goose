# pylint: disable=duplicate-code, consider-using-with, no-name-in-module, reimported, wrong-import-order, wrong-import-position
"""
This module contains tests for Azure AI Inference functionality using the
Azure AI Inference Python library.

Tests cover various API endpoints, including completions, chat completions,
embeddings, fine-tuning job creation, image generation, image variation creation,
and audio speech generation. These tests validate integration with OpenLIT.

Environment Variables:
    - AZURE_AI_INFERENCE_API_TOKEN: Azure AI Inference API key for authentication.

Note: Ensure the environment is properly configured for Azure AI Inference access
and OpenLIT monitoring prior to running these tests.
"""

import os
import pytest
import openlit
from azure.ai.inference import ChatCompletionsClient, EmbeddingsClient
from azure.ai.inference.models import SystemMessage
from azure.ai.inference.models import UserMessage
from azure.core.credentials import AzureKeyCredential

# Initialize synchronous Azure AI Inference Chat client
sync_chat_client = ChatCompletionsClient(
    endpoint="https://models.github.ai/inference",
    credential=AzureKeyCredential(os.getenv("AZURE_AI_INFERENCE_API_TOKEN")),
)

# Initialize synchronous Azure AI Inference Chat client
sync_embed_client = EmbeddingsClient(
    endpoint="https://models.github.ai/inference",
    credential=AzureKeyCredential(os.getenv("AZURE_AI_INFERENCE_API_TOKEN")),
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")


def test_sync_chat_completions():
    """
    Tests synchronous chat completions with the "microsoft/Phi-4-mini-reasoning" model.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    response = sync_chat_client.complete(
        messages=[
            SystemMessage(content="You are a helpful assistant."),
            UserMessage(content="sync non-streaming"),
        ],
        model="microsoft/Phi-4-mini-reasoning",
        temperature=1.0,
        max_tokens=1000,
        top_p=1.0,
    )
    assert response["object"] == "chat.completion"


def test_sync_embeddings():
    """
    Tests synchronous embedding creation with the "openai/text-embedding-3-small" model.

    Raises:
        AssertionError: If the embedding response object is not as expected.
    """

    response = sync_embed_client.embed(
        input=["LLM Observability", "Monitor GPUs"],
        model="openai/text-embedding-3-small",
    )
    assert response.data[0]["object"] == "embedding"


from azure.ai.inference.aio import ChatCompletionsClient, EmbeddingsClient

async_chat_client = ChatCompletionsClient(
    endpoint="https://models.github.ai/inference",
    credential=AzureKeyCredential(os.getenv("AZURE_AI_INFERENCE_API_TOKEN")),
)

async_embed_client = EmbeddingsClient(
    endpoint="https://models.github.ai/inference",
    credential=AzureKeyCredential(os.getenv("AZURE_AI_INFERENCE_API_TOKEN")),
)


@pytest.mark.asyncio
async def test_async_chat_completions():
    """
    Tests synchronous chat completions with the "microsoft/Phi-4-mini-reasoning" model.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    response = await async_chat_client.complete(
        messages=[
            SystemMessage(content="You are a helpful assistant."),
            UserMessage(content="sync non-streaming"),
        ],
        model="microsoft/Phi-4-mini-reasoning",
        temperature=1.0,
        max_tokens=1000,
        top_p=1.0,
    )
    assert response["object"] == "chat.completion"


@pytest.mark.asyncio
async def test_async_embeddings():
    """
    Tests synchronous embedding creation with the "openai/text-embedding-3-small" model.

    Raises:
        AssertionError: If the embedding response object is not as expected.
    """

    response = await async_embed_client.embed(
        input=["LLM Observability", "Monitor GPUs"],
        model="openai/text-embedding-3-small",
    )
    assert response.data[0]["object"] == "embedding"
