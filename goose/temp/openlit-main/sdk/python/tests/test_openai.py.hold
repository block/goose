# pylint: disable=duplicate-code, consider-using-with, no-name-in-module
"""
This module contains tests for OpenAI functionality using the OpenAI Python library.

Tests cover various API endpoints, including completions, chat completions,
embeddings, fine-tuning job creation, image generation, image variation creation,
and audio speech generation. These tests validate integration with OpenLIT.

Environment Variables:
    - OPENAI_API_KEY: OpenAI API key for authentication.

Note: Ensure the environment is properly configured for OpenAI access and OpenLIT monitoring
prior to running these tests.
"""

import pytest
from openai import OpenAI, AsyncOpenAI
from pydantic import BaseModel
import openlit

# Initialize synchronous OpenAI client
sync_client = OpenAI()

# Initialize asynchronous OpenAI client
async_client = AsyncOpenAI()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing", application_name="openlit-python-openai-test"
)


def test_sync_openai_chat_completions():
    """
    Tests synchronous chat completions with the 'gpt-3.5-turbo' model.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    chat_completions_resp = sync_client.chat.completions.create(
        model="gpt-3.5-turbo",
        max_tokens=1,
        messages=[{"role": "user", "content": "What is LLM Observability?"}],
    )
    assert chat_completions_resp.object == "chat.completion"


def test_sync_openai_chat_completions_parse():
    """
    Tests synchronous chat completions with the 'gpt-3.5-turbo' model.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    class User(BaseModel):
        """A model to represent a user's details."""

        name: str
        age: int

    chat_completions_resp = sync_client.beta.chat.completions.parse(
        model="gpt-4o-mini",
        messages=[
            {
                "role": "system",
                "content": "Extract the user's name and age from the following text.",
            },
            {
                "role": "user",
                "content": "The user's name is John Doe and he is 30 years old.",
            },
        ],
        response_format=User,  # Pass the Pydantic model as the response format
    )
    assert chat_completions_resp.object == "chat.completion"


def test_sync_openai_embeddings():
    """
    Tests synchronous embedding creation with the 'text-embedding-ada-002' model.

    Raises:
        AssertionError: If the embedding response object is not as expected.
    """

    embeddings_resp = sync_client.embeddings.create(
        model="text-embedding-ada-002",
        input="The quick brown fox jumped over the lazy dog",
        encoding_format="float",
    )
    assert embeddings_resp.data[0].object == "embedding"


# def test_sync_openai_fine_tuning_job_creation():
#     """
#     Test fine-tuning job creation.

#     Raises:
#         AssertionError: If the fine-tuning job response object is not as expected.
#     """
#     try:
#         fine_tuning_job_resp = sync_client.fine_tuning.jobs.create(
#             training_file="",
#             model="gpt-3.5-turbo-1106"
#         )
#         assert fine_tuning_job_resp.object == 'fine_tuning.job'

#     # pylint: disable=broad-exception-caught
#     except Exception as e:
#         if 'rate_limit_exceeded' in str(e):
#             error_json = e.response.json()
#             rate_limit_code = error_json['error']['code']
#             print(rate_limit_code)


def test_sync_openai_image_generation():
    """
    Tests synchronous image generation with the 'dall-e-2' model.

    Raises:
        AssertionError: If the image generation response created timestamp is not present.
    """

    image_generation_resp = sync_client.images.generate(
        model="dall-e-2", prompt="Generate an image of a cat.", size="256x256", n=1
    )
    assert image_generation_resp.created is not None


def test_sync_openai_image_variations():
    """
    Tests synchronous image variation creation with the 'dall-e-2' model

    Raises:
        AssertionError: If the image variation response created timestamp is not present.
    """

    image_variation_resp = sync_client.images.create_variation(
        image=open("tests/test-image-for-openai.png", "rb"),
        model="dall-e-2",
        n=1,
        size="256x256",
    )
    assert image_variation_resp.created is not None


def test_sync_openai_audio_speech_create():
    """
    Tests synchronous audio speech generation with the 'tts-1' model.

    Raises:
        AssertionError: If the audio speech response is not present or not an instance of an object.
    """

    audio_speech_resp = sync_client.audio.speech.create(
        model="tts-1", voice="alloy", input="LLM Observability!"
    )
    assert audio_speech_resp is not None and isinstance(audio_speech_resp, object)


@pytest.mark.asyncio
async def test_async_openai_chat_completions():
    """
    Tests asynchronous chat completions with the 'gpt-3.5-turbo' model.

    Raises:
        AssertionError: If the chat completion response object is not as expected.
    """

    chat_completions_resp = await async_client.chat.completions.create(
        model="gpt-3.5-turbo",
        max_tokens=1,
        messages=[{"role": "user", "content": "What is LLM Observability?"}],
    )
    assert chat_completions_resp.object == "chat.completion"


@pytest.mark.asyncio
async def test_async_openai_chat_completions_parse():
    """
    Tests asynchronous chat completions parse with the 'gpt-3.5-turbo' model.

    Raises:
        AssertionError: If the chat completion parse response object is not as expected.
    """

    class User(BaseModel):
        """A model to represent a user's details."""

        name: str
        age: int

    chat_completions_resp = await async_client.beta.chat.completions.parse(
        model="gpt-4o-mini",
        messages=[
            {
                "role": "system",
                "content": "Extract the user's name and age from the following text.",
            },
            {
                "role": "user",
                "content": "The user's name is John Doe and he is 30 years old.",
            },
        ],
        response_format=User,  # Pass the Pydantic model as the response format
    )
    assert chat_completions_resp.object == "chat.completion"


# @pytest.mark.asyncio
# async def test_async_openai_embeddings():
#     """
#     Tests asynchronous embedding creation with the 'text-embedding-ada-002' model.

#     Raises:
#         AssertionError: If the embedding response object is not as expected.
#     """

#     embeddings_resp = await async_client.embeddings.create(
#         model="text-embedding-ada-002",
#         input="The quick brown fox jumped over the lazy dog",
#         encoding_format="float"
#     )
#     assert embeddings_resp.data[0].object == 'embedding'

# @pytest.mark.asyncio
# async def test_async_openai_image_variations():
#     """
#     Tests asynchronous image variation creation with the 'dall-e-2' model.

#     Raises:
#         AssertionError: If the image variation response created timestamp is not present.
#     """

#     image_variation_resp = await async_client.images.create_variation(
#         image=open("tests/test-image-for-openai.png", "rb"),
#         model='dall-e-2',
#         n=1,
#         size="256x256"
#     )
#     assert image_variation_resp.created is not None

# @pytest.mark.asyncio
# async def test_async_openai_audio_speech_create():
#     """
#     Tests asynchronous audio speech generation with the 'tts-1' model.

#     Raises:
#         AssertionError: If the audio speech response is not present
#     """

#     audio_speech_resp = await async_client.audio.speech.create(
#         model='tts-1',
#         voice='alloy',
#         input='LLM Observability!')
#     assert audio_speech_resp is not None and isinstance(audio_speech_resp, object)
