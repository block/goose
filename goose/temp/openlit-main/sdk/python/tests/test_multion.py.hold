# pylint: disable=duplicate-code, no-name-in-module, import-error, no-member
"""
This module contains tests for MultiOn functionality using the MultiOn Python library.

Tests cover various API endpoints.
These tests validate integration with OpenLIT.

Environment Variables:
    - MULTION_API_KEY: MultiOn API key for authentication.

Note: Ensure the environment is properly configured for MultiOn access and OpenLIT monitoring
prior to running these tests.
"""

import pytest
from multion.client import MultiOn, AsyncMultiOn
import openlit

# Initialize synchronous MultiOn client
sync_client = MultiOn()

# Initialize asynchronous MultiOn client
async_client = AsyncMultiOn()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-multion-test")

def test_sync_multion_browse():
    """
    Tests synchronous Agent Browse.

    Raises:
        AssertionError: If the Agent Browse response object is not as expected.
    """

    try:
        response = sync_client.browse(
            url="https://openlit.io",
            cmd="say hi"
        )
        assert response.status in ['DONE', 'ASK_USER']

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 402:
            print("Insufficient balance:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_multion_browse():
    """
    Tests asynchronous Agent Browse

    Raises:
        AssertionError: If the Agent Browse response object is not as expected.
    """

    try:
        response = await async_client.browse(
            url="https://docs.openlit.io",
            cmd="say hi"
        )
        assert response.status in ['DONE', 'ASK_USER']

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if e.status_code == 402:
            print("Insufficient balance:", e)
        else:
            raise
