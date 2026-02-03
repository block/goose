# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for mem0 functionality using the mem0ai Python library.

Tests cover various SDK functions. 
These tests validate integration with OpenLIT.

Environment Variables:
    - OPENAI_API_KEY: OpenAI API key for authentication.

Note: Ensure the environment is properly configured for mem0 and OpenLIT monitoring
prior to running these tests.
"""

from mem0 import Memory
import openlit

# Initialize synchronous mem0 client
sync_memory = Memory()

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-mem0-test")

def test_sync_memory_add():
    """
    Tests synchronous addition to memory

    Raises:
        AssertionError: If the memory add response object is not as expected.
    """

    try:
        response = sync_memory.add(
            "OpenLIT provides LLM Observability and Agent Observability",
            user_id="openlit",
            metadata={"category": "devtool"})
        assert response[0]["event"] == "ADD"

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise

def test_sync_memory_get_all():
    """
    Tests synchronous get all memory

    Raises:
        AssertionError: If the get all memory response object is not as expected.
    """

    try:
        response = sync_memory.get_all(user_id="openlit")
        assert response[0]["user_id"] == "openlit"

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise

def test_sync_memory_get():
    """
    Tests synchronous get memory

    Raises:
        AssertionError: If the get memory response object is not as expected.
    """

    try:
        response = sync_memory.get("bf4d4092-cf91-4181-bfeb-b6fa2ed3061b")
        assert response is None

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise

def test_sync_memory_search():
    """
    Tests synchronous search memory

    Raises:
        AssertionError: If the search memory response object is not as expected.
    """

    try:
        response = sync_memory.search(query="What does OpenLIT give?", user_id="openlit")
        assert isinstance(response, list)

    # pylint: disable=broad-exception-caught, try-except-raise
    except Exception:
        raise
