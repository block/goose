# pylint: disable=duplicate-code, consider-using-with, no-name-in-module
"""
This module contains tests for Letta functionality using the Letta Python library.

Tests cover various API endpoints, including completions, chat completions,
embeddings, fine-tuning job creation, image generation, image variation creation,
and audio speech generation. These tests validate integration with OpenLIT.

Environment Variables:
    - OPENAI_API_KEY: OpenAI API key for authentication.

Note: Ensure the environment is properly configured for OpenAI access and OpenLIT monitoring
prior to running these tests.
"""

from letta import EmbeddingConfig, LLMConfig, create_client
import openlit

# Initialize synchronous Letta client
sync_client = create_client()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-letta-test")

def test_sync_letta_agent_ops():
    """
    Tests synchronous agent creation and message

    Raises:
        AssertionError: If the response object is not as expected.
    """

    # set automatic defaults for LLM/embedding config
    sync_client.set_default_llm_config(LLMConfig.default_config(model_name="gpt-4"))
    sync_client.set_default_embedding_config(
        EmbeddingConfig.default_config(
            model_name="text-embedding-ada-002"
        )
    )

    # create a new agent
    response = sync_client.create_agent()
    assert isinstance(response.name, str)

    # Message an agent
    response = sync_client.send_message(agent_id=response.id, role="user", message="hello")
    assert isinstance(response.usage.total_tokens, int)
