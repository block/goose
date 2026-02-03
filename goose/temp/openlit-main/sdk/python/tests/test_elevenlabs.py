# pylint: disable=duplicate-code, no-name-in-module, import-error
"""
This module contains tests for ElevenLabs functionality using the ElevenLabs Python library.

Tests cover various API endpoints.
These tests validate integration with OpenLIT.

Environment Variables:
    - ELEVEN_API_KEY: ElevenLabs API key for authentication.

Note: Ensure the environment is properly configured for ElevenLabs access and OpenLIT monitoring
prior to running these tests.
"""

import types
import pytest
from elevenlabs.client import ElevenLabs, AsyncElevenLabs
import openlit

# Initialize synchronous ElevenLabs client
sync_client = ElevenLabs()

async_client = AsyncElevenLabs()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing",
    application_name="openlit-python-elevenlabs-test",
)


def test_sync_elevenlabs_t2s():
    """
    Tests synchronous text-to-speech with the 'eleven_multilingual_v2' model.

    Raises:
        AssertionError: If the text-to-speech response object is not as expected.
    """

    try:
        audio = sync_client.text_to_speech.convert(
            voice_id="21m00Tcm4TlvDq8ikWAM",
            output_format="mp3_22050_32",
            text="Say Monitoring LLM Applications",
            model_id="eleven_multilingual_v2",
        )
        assert isinstance(audio, types.GeneratorType)

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise


@pytest.mark.asyncio
async def test_async_elevenlabs_t2s():
    """
    Tests synchronous text-to-speech with the 'eleven_multilingual_v2' model.

    Raises:
        AssertionError: If the text-to-speech response object is not as expected.
    """

    try:
        async_client.text_to_speech.convert(
            voice_id="21m00Tcm4TlvDq8ikWAM",
            output_format="mp3_22050_32",
            text="Say Monitoring LLM Applications",
            model_id="eleven_multilingual_v2",
        )

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
