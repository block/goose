# pylint: disable=duplicate-code, no-name-in-module, no-member, import-error
"""
This module contains tests for Vertex AI functionality using the Vertex AI Python library.

Tests cover various API endpoints, including chat. 
These tests validate integration with OpenLIT.

Environment Variables:
    - GCP_PROJECT_ID: GCP Project ID for authentication.

Note: Ensure the environment is properly configured for Vertex AI access and OpenLIT monitoring
prior to running these tests.
"""

import os
import pytest
import vertexai
from vertexai.generative_models import (
    GenerativeModel
)
from vertexai.language_models import (
    TextGenerationModel,
    ChatModel,
    TextEmbeddingModel
)
import openlit

# Initialize Vertex AI
vertexai.init(project=os.getenv("GCP_PROJECT_ID"), location="us-central1")


# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-testing", application_name="openlit-python-test")

def test_sync_vertexai_send_message():
    """
    Tests synchronous send_message with the 'gemini-1.0-pro-001' model.

    Raises:
        AssertionError: If the send_message response object is not as expected.
    """

    try:
        generation_config = {
            "max_output_tokens": 15,
            "temperature": 1,
            "top_p": 1,
        }
        model = GenerativeModel(model_name="gemini-1.0-pro-001")
        chat = model.start_chat()

        response = chat.send_message("Just say 'LLM Observability'",
                                           stream=False, generation_config=generation_config)

        assert response.candidates[0].content.role == 'model'


        works = False
        responses = chat.send_message("Just say 'LLM Monitoring'",
                                      stream=True, generation_config=generation_config)
        for response in responses:
            if response.candidates[0].content.role == 'model':
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

def test_sync_vertexai_generate_content():
    """
    Tests synchronous generate_content with the 'gemini-1.0-pro-001' model.

    Raises:
        AssertionError: If the generate_content response object is not as expected.
    """

    try:
        generation_config = {
            "max_output_tokens": 25,
            "temperature": 1,
            "top_p": 1,
        }
        model = GenerativeModel("gemini-1.0-pro-002")

        response = model.generate_content(
            ["""Say 'LLM Observability'"""],
            generation_config=generation_config,
            stream=False,
        )

        assert response.candidates[0].content.role == 'model'


        works = False
        responses = model.generate_content(
            ["""Say 'LLM Observability'"""],
            generation_config=generation_config,
            stream=True,
        )
        for response in responses:
            if response.candidates[0].content.role == 'model':
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

def test_sync_vertexai_predict():
    """
    Tests synchronous predict with the text-biason' model.

    Raises:
        AssertionError: If the predict response object is not as expected.
    """

    try:
        parameters = {
            "max_output_tokens": 1,
        }
        model = TextGenerationModel.from_pretrained("text-bison@002")

        response = model.predict(
            "Just say 'LLM Observability'",
            **parameters,
        )

        assert isinstance(response.text, str)

        works = False
        responses = model.predict_streaming(
            "Just say 'LLM Observability'",
            **parameters,
        )
        for response in responses:
            if response.is_blocked is False:
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

def test_sync_vertexai_start_chat():
    """
    Tests synchronous start_chat with the chat-biason' model.

    Raises:
        AssertionError: If the start_chat response object is not as expected.
    """

    try:
        parameters = {
            "max_output_tokens": 1,
        }
        chat_model = ChatModel.from_pretrained("chat-bison@002")
        chat = chat_model.start_chat(
                context="LLM Monitoring",
            )

        response = chat.send_message(
            "What is LLM Observability?", **parameters
        )

        assert isinstance(response.text, str)

        works = False
        responses = chat.send_message_streaming(
            "What is LLM Observability?", **parameters
        )
        for response in responses:
            if response.is_blocked is False:
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

def test_sync_vertexai_get_embeddings():
    """
    Tests synchronous get_embeddings with the textembedding-gecko@001 model.

    Raises:
        AssertionError: If the get_embeddings response object is not as expected.
    """

    try:
        model = TextEmbeddingModel.from_pretrained("textembedding-gecko@001")
        response = model.get_embeddings(["What is LLM Observability?"])

        assert response[0].statistics.truncated is False

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_vertexai_send_message():
    """
    Tests asynchronous send_message with the 'gemini-1.0-pro-001' model.

    Raises:
        AssertionError: If the send_message response object is not as expected.
    """

    try:
        generation_config = {
            "max_output_tokens": 15,
            "temperature": 1,
            "top_p": 1,
        }
        model = GenerativeModel(model_name="gemini-1.0-pro-001")
        chat = model.start_chat()

        response = await chat.send_message_async("Just say 'LLM Observability'",
                                                 stream=False, generation_config=generation_config)

        assert response.candidates[0].content.role == 'model'

        works = False
        responses = await chat.send_message_async("Just say 'LLM Observability'",
                                                  stream=True, generation_config=generation_config)
        async for response in responses:
            if response.candidates[0].content.role == 'model':
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_vertexai_generate_content():
    """
    Tests asynchronous generate_content with the 'gemini-1.0-pro-001' model.

    Raises:
        AssertionError: If the generate_content response object is not as expected.
    """

    try:
        generation_config = {
            "max_output_tokens": 25,
            "temperature": 1,
            "top_p": 1,
        }
        model = GenerativeModel("gemini-1.0-pro-002")

        response = await model.generate_content_async(
            ["""Say 'LLM Observability'"""],
            generation_config=generation_config,
            stream=False,
        )

        assert response.candidates[0].content.role == 'model'


        works = False
        responses = await model.generate_content_async(
            ["""Say 'LLM Observability'"""],
            generation_config=generation_config,
            stream=True,
        )
        for response in responses:
            if response.candidates[0].content.role == 'model':
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_vertexai_predict():
    """
    Tests asynchronous predict with the text-biason' model.

    Raises:
        AssertionError: If the predict response object is not as expected.
    """

    try:
        parameters = {
            "max_output_tokens": 1,
        }
        model = TextGenerationModel.from_pretrained("text-bison@002")
        response = await model.predict_async(
            "Just say 'LLM Observability'",
            **parameters,
        )

        assert isinstance(response.text, str)

        works = False
        responses = await model.predict_streaming_async(
            "Just say 'LLM Observability'",
            **parameters,
        )
        async for response in responses:
            if response.is_blocked is False:
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_vertexai_start_chat():
    """
    Tests asynchronous start_chat with the chat-biason' model.

    Raises:
        AssertionError: If the start_chat response object is not as expected.
    """

    try:
        parameters = {
            "max_output_tokens": 1,
        }
        chat_model = ChatModel.from_pretrained("chat-bison@002")
        chat = chat_model.start_chat(
                context="LLM Monitoring",
            )

        response = await chat.send_message_async(
            "What is LLM Observability?", **parameters
        )

        assert isinstance(response.text, str)

        works = False
        responses = await chat.send_message_streaming_async(
            "What is LLM Observability?", **parameters
        )
        async for response in responses:
            if response.is_blocked is False:
                works = True

        assert works is True

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise

@pytest.mark.asyncio
async def test_async_vertexai_get_embeddings():
    """
    Tests synchronous get_embeddings with the textembedding-gecko@001 model.

    Raises:
        AssertionError: If the get_embeddings response object is not as expected.
    """

    try:
        model = TextEmbeddingModel.from_pretrained("textembedding-gecko@001")
        response = await model.get_embeddings_async(["What is LLM Observability?"])

        assert response[0].statistics.truncated is False

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "rate limit" in str(e).lower():
            print("Rate Limited:", e)
        else:
            raise
