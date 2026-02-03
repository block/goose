# pylint: disable=duplicate-code, no-name-in-module, import-error, no-member
"""
This module contains tests for Reka AI functionality using the Reka Python library.

Tests cover various API endpoints, including chat.
These tests validate integration with OpenLIT.

Environment Variables:
    - REKA_API_KEY: Reka API key for authentication.

Note: Ensure the environment is properly configured for Reka access and OpenLIT monitoring
prior to running these tests.
"""

import pytest
from reka import ChatMessage
from reka.client import Reka, AsyncReka
import openlit

# Initialize synchronous Reka client
sync_client = Reka()

# Initialize asynchronous Reka client
async_client = AsyncReka()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-reka-test"
)


def test_sync_reka_chat():
    """
    Tests synchronous Chat Create.

    Raises:
        AssertionError: If the Chat Create response object is not as expected.
    """

    try:
        response = sync_client.chat.create(
            messages=[
                ChatMessage(
                    content="How to monitor AI Agents?",
                    role="user",
                )
            ],
            model="reka-core",
            max_tokens=1,
        )
        assert response.model == "reka-core"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 429:
            print("Insufficient balance:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_reka_chat():
    """
    Tests asynchronous Chat Create

    Raises:
        AssertionError: If the Chat Create response object is not as expected.
    """

    try:
        response = await async_client.chat.create(
            messages=[
                ChatMessage(
                    content="How to monitor AI Agents?",
                    role="user",
                )
            ],
            model="reka-core",
            max_tokens=1,
        )
        assert response.model == "reka-core"

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 429:
            print("Insufficient balance:", e)
        else:
            raise
