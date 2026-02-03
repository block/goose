# pylint: disable=duplicate-code, no-name-in-module, import-error, no-member
"""
This module contains tests for ControlFlow functionality using the ControlFlow Python library.

Tests cover various API endpoints, including chat and embeddings.
These tests validate integration with OpenLIT.

Environment Variables:
    - OPENAI_API_KEY: OpenAI API key for authentication.

Note: Ensure the environment is properly configured for OpenAI access and OpenLIT monitoring
prior to running these tests.
"""

import controlflow as cf
import openlit

# Initialize environment and application name for OpenLIT monitoring
openlit.init(
    environment="openlit-python-testing",
    application_name="openlit-python-controlflow-test",
)


def test_sync_controlflow_agent():
    """
    Tests Agent Creation.

    Raises:
        AssertionError: If the agent creation response object is not as expected.
    """

    agent = cf.Agent(
        name="Email Classifier",
        model="openai/gpt-4o-mini",
        instructions="You are an AI Observability expert",
    )
    assert isinstance(agent, cf.Agent)


def test_sync_controlflow_task():
    """
    Tests Task Creation.

    Raises:
        AssertionError: If the task creation response object is not as expected.
    """

    task = cf.Task("Write OpenLIT AI Observability docs")
    assert isinstance(task, cf.Task)


def test_sync_ai21_chat_rag():
    """
    Tests synchronous run.

    Raises:
        AssertionError: If the run response object is not as expected.
    """

    response = cf.run(
        "What is LLM Observability?",
    )

    assert isinstance(response, str)
