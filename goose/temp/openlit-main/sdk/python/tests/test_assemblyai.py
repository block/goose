# pylint: disable=duplicate-code, no-name-in-module, import-error, no-member
"""
This module contains tests for Assembly AI functionality using the Assembly AI Python library.

Tests cover various API endpoints.
These tests validate integration with OpenLIT.

Environment Variables:
    - ASSEMBLYAI_API_KEY: Assembly AI API key for authentication.

Note: Ensure the environment is properly configured for Assembly AI access and OpenLIT monitoring
prior to running these tests.
"""

import assemblyai as aai
import openlit

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing",
    application_name="openlit-python-assemblyai-test",
)


def test_sync_assemblyai_transcribe():
    """
    Tests synchronous transcribe.

    Raises:
        AssertionError: If the response object is not as expected.
    """

    try:
        audio_file = "https://audio-samples.github.io/samples/mp3/blizzard_unconditional/sample-0.mp3"

        transcriber = aai.Transcriber()
        config = aai.TranscriptionConfig(speech_model=aai.SpeechModel.nano)

        transcript = transcriber.transcribe(audio_file, config=config)
        assert isinstance(transcript.text, str)

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
