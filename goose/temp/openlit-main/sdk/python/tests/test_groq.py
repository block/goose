# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Groq functionality using the Groq Python library.

Tests cover various API endpoints, including chat.
These tests validate integration with OpenLIT.

Environment Variables:
    - GROQ_API_TOKEN: Groq API api_key for authentication.

Note: Ensure the environment is properly configured for Groq access and OpenLIT monitoring
prior to running these tests.
"""

import os
import pytest
from groq import Groq, AsyncGroq
import openlit

# Initialize synchronous Groq client
sync_client = Groq(api_key=os.getenv("GROQ_API_TOKEN"))

# Initialize asynchronous Groq client
async_client = AsyncGroq(api_key=os.getenv("GROQ_API_TOKEN"))

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")


def test_sync_groq_chat():
    """
    Tests synchronous Chat Completions.

    Raises:
        AssertionError: If the Chat Completions response object is not as expected.
    """

    try:
        chat_completions_resp = sync_client.chat.completions.create(
            messages=[
                {
                    "role": "user",
                    "content": "Monitor LLM Applications",
                }
            ],
            model="llama-3.1-8b-instant",
            max_tokens=1,
            stream=False,
        )
        assert chat_completions_resp.object == "chat.completion"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_groq_chat():
    """
    Tests synchronous Chat Completions with the 'claude-3-haiku-20240307' model.

    Raises:
        AssertionError: If the Chat Completions response object is not as expected.
    """

    try:
        chat_completions_resp = await async_client.chat.completions.create(
            messages=[
                {
                    "role": "user",
                    "content": "What is LLM Observability?",
                }
            ],
            model="llama-3.1-8b-instant",
            max_tokens=1,
            stream=False,
        )
        assert chat_completions_resp.object == "chat.completion"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
