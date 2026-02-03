# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Anthropic functionality using the Anthropic Python library.

Tests cover various API endpoints, including chat.
These tests validate integration with OpenLIT.

Environment Variables:
    - ANTHROPIC_API_KEY: Anthropic API key for authentication.

Note: Ensure the environment is properly configured for Anthropic access and OpenLIT monitoring
prior to running these tests.
"""

import pytest
from anthropic import Anthropic, AsyncAnthropic
import openlit

# Initialize synchronous Anthropic client
sync_client = Anthropic()

# Initialize asynchronous Anthropic client
async_client = AsyncAnthropic()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing",
    application_name="openlit-python-anthropic-test",
)


def test_sync_anthropic_messages():
    """
    Tests synchronous messages with the 'claude-3-haiku-20240307' model.

    Raises:
        AssertionError: If the messages response object is not as expected.
    """

    try:
        message = sync_client.messages.create(
            max_tokens=1,
            messages=[
                {
                    "role": "user",
                    "content": "Hello, Claude",
                }
            ],
            model="claude-3-haiku-20240307",
        )
        assert message.type == "message"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_anthropic_messages():
    """
    Tests synchronous messages with the 'claude-3-haiku-20240307' model.

    Raises:
        AssertionError: If the messages response object is not as expected.
    """

    try:
        message = await async_client.messages.create(
            max_tokens=1,
            messages=[
                {
                    "role": "user",
                    "content": "Hello, Claude",
                }
            ],
            model="claude-3-haiku-20240307",
        )
        assert message.type == "message"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
