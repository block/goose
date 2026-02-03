# pylint: disable=duplicate-code, consider-using-with, no-name-in-module, no-member
"""
This module contains tests for xAI functionality using the OpenAI Python library.

Tests cover various API endpoints, including completions, chat completions,
embeddings, fine-tuning job creation, image generation, image variation creation,
and audio speech generation. These tests validate integration with OpenLIT.

Environment Variables:
    - XAI_API_KEY: xAI API key for authentication.

Note: Ensure the environment is properly configured for xAI access and OpenLIT monitoring
prior to running these tests.
"""

import os
import pytest
from openai import OpenAI, AsyncOpenAI
import openlit

# Initialize synchronous OpenAI client
sync_client = OpenAI(
    api_key=os.getenv("XAI_API_KEY"),
    base_url="https://api.x.ai/v1",
)

# Initialize asynchronous OpenAI client
async_client = AsyncOpenAI(
    api_key=os.getenv("XAI_API_KEY"),
    base_url="https://api.x.ai/v1",
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-xai-test"
)


def test_sync_xai_chat_completions():
    """
    Tests synchronous chat completions

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    try:
        response = sync_client.chat.completions.create(
            model="grok-beta",
            messages=[
                {"role": "user", "content": "Hi"},
            ],
            max_tokens=1,
        )
        assert response.object == "chat.completion"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 429:
            print("Insufficient balance:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_xai_chat_completions():
    """
    Tests asynchronous chat completions.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    try:
        response = await async_client.chat.completions.create(
            model="grok-beta",
            messages=[
                {"role": "user", "content": "Hi"},
            ],
            max_tokens=1,
        )
        assert response.object == "chat.completion"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 429:
            print("Insufficient balance:", e)
        else:
            raise
