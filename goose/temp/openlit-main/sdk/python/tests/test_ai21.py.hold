# pylint: disable=duplicate-code, no-name-in-module, import-error
"""
This module contains tests for AI21 functionality using the AI21 Python library.

Tests cover various API endpoints, including chat and embeddings.
These tests validate integration with OpenLIT.

Environment Variables:
    - AI21_API_KEY: AI21 API key for authentication.

Note: Ensure the environment is properly configured for AI21 access and OpenLIT monitoring
prior to running these tests.
"""

import pytest
from ai21 import AI21Client, AsyncAI21Client
from ai21.models.chat import ChatMessage
import openlit

# Initialize synchronous AI21 client
sync_client = AI21Client()

# Initialize synchronous AI21 client
async_client = AsyncAI21Client()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-ai21-test"
)

MESSAGES = [
    ChatMessage(content="hi", role="user"),
]


def test_sync_ai21_chat():
    """
    Tests synchronous chat.

    Raises:
        AssertionError: If the chat response object is not as expected.
    """

    response = sync_client.chat.completions.create(
        messages=MESSAGES,
        model="jamba-mini",
        max_tokens=1,
    )
    assert isinstance(response.id, str)


def test_sync_ai21_chat_stream():
    """
    Tests synchronous chat streaming.

    Raises:
        AssertionError: If the streaming chat response object is not as expected.
    """

    responses = sync_client.chat.completions.create(
        messages=MESSAGES,
        model="jamba-mini",
        stream=True,
        max_tokens=1,
    )
    for response in responses:
        assert isinstance(response.id, str)
        return


def test_sync_ai21_chat_rag():
    """
    Tests synchronous chat rag.

    Raises:
        AssertionError: If the streaming chat rag response object is not as expected.
    """

    response = sync_client.beta.conversational_rag.create(
        messages=MESSAGES,
        file_ids=[],
        max_segments=15,
        retrieval_strategy="segments",
        retrieval_similarity_threshold=0.8,
        max_neighbors=1,
        max_tokens=1,
    )
    assert isinstance(response.id, str)


@pytest.mark.asyncio
async def test_async_ai21_test():
    """
    Tests asynchronous chat stream and rag operations.

    Raises:
        AssertionError: If the response object is not as expected.
    """

    response = await async_client.chat.completions.create(
        messages=MESSAGES,
        model="jamba-mini",
        max_tokens=1,
    )
    assert isinstance(response.id, str)

    responses = await async_client.chat.completions.create(
        messages=MESSAGES,
        model="jamba-mini",
        stream=True,
        max_tokens=1,
    )
    async for text in responses:
        assert isinstance(text.id, str)
        return

    response = await async_client.beta.conversational_rag.create(
        messages=MESSAGES,
        file_ids=[],
        max_segments=15,
        retrieval_strategy="segments",
        retrieval_similarity_threshold=0.8,
        max_neighbors=1,
        max_tokens=1,
    )
    assert isinstance(response.id, str)
