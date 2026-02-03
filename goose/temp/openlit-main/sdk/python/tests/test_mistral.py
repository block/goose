# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Mistral functionality using the Mistral Python library.

Tests cover various API endpoints, including chat and embeddings.
These tests validate integration with OpenLIT.

Environment Variables:
    - MISTRAL_API_KEY: Mistral API key for authentication.

Note: Ensure the environment is properly configured for Mistral access and OpenLIT monitoring
prior to running these tests.
"""

import os
import pytest
from mistralai import Mistral
import openlit

# Initialize synchronous Mistral client
client = Mistral(
    api_key=os.getenv("MISTRAL_API_KEY"),
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-mistral-test"
)


def test_sync_mistral_chat():
    """
    Tests synchronous chat with the 'open-mistral-7b' model.

    Raises:
        AssertionError: If the chat response object is not as expected.
    """

    messages = [
        {
            "role": "user",
            "content": "sync: What is LLM Observability?",
        },
    ]

    message = client.chat.complete(
        model="open-mistral-7b",
        messages=messages,
        max_tokens=1,
    )
    assert message.object == "chat.completion"


def test_sync_mistral_embeddings():
    """
    Tests synchronous embedding creation with the 'mistral-embed' model.

    Raises:
        AssertionError: If the embedding response object is not as expected.
    """

    response = client.embeddings.create(
        model="mistral-embed",
        inputs=["Embed this sentence.", "OpenTelemetry LLM Observability"],
    )
    assert response.object == "list"


@pytest.mark.asyncio
async def test_async_mistral():
    """
    Tests asynchronous Mistral.

    Raises:
        AssertionError: If the chat response object is not as expected.
    """

    #  Tests synchronous chat with the 'open-mistral-7b' model.
    messages = [
        {
            "role": "user",
            "content": "sync: What is LLM Observability?",
        },
    ]

    message = await client.chat.complete_async(
        model="open-mistral-7b",
        messages=messages,
        max_tokens=1,
    )
    assert message.object == "chat.completion"

    # Tests asynchronous embedding creation with the 'mistral-embed' model.
    response = await client.embeddings.create_async(
        model="mistral-embed",
        inputs=["Embed this sentence.", "Monitor LLM Applications"],
    )
    assert response.object == "list"
