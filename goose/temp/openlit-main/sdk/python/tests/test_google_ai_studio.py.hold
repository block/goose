# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Google AI Studio functionality using the google-generativeai
Python library.

Tests cover various API endpoints, including chat.
These tests validate integration with OpenLIT.

Environment Variables:
    - GOOGLE_AI_STUDIO_API_TOKEN: Google AI Studio API api_key for authentication.

Note: Ensure the environment is properly configured for Google AI Studio access and
OpenLIT monitoring prior to running these tests.
"""

import os
import pytest
from google import genai
from google.genai import types
import openlit

# Initialize Google AI Studio client
client = genai.Client(api_key=os.getenv("GOOGLE_AI_STUDIO_API_TOKEN"))
contents = [
    types.Content(
        role="user",
        parts=[
            types.Part.from_text(text="""Hi"""),
        ],
    ),
]
generate_content_config = types.GenerateContentConfig(
    response_mime_type="text/plain",
)

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-testing", application_name="openlit-python-gemini-test"
)


def test_sync_generate_content():
    """
    Tests synchronous Generate content with the "gemini-2.0-flash" model.

    Raises:
        AssertionError: If the generate content response object is not as expected.
    """

    try:
        response = client.models.generate_content(
            model="gemini-2.0-flash",
            contents=contents,
            config=generate_content_config,
        )
        assert isinstance(response.text, str)

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_generate_content():
    """
    Tests synchronous Generate content with the "gemini-2.0-flash" model.

    Raises:
        AssertionError: If the generate content response object is not as expected.
    """

    try:
        response = await client.aio.models.generate_content(
            model="gemini-2.0-flash",
            contents=contents,
            config=generate_content_config,
        )
        assert isinstance(response.text, str)

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
