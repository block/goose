# pylint: disable=duplicate-code, no-name-in-module
"""
This module contains tests for Prem AI functionality using the Prem Python library.

Tests cover various API endpoints, including chat. 
These tests validate integration with OpenLIT.

Environment Variables:
    - PREM_API_KEY: Prem API key for authentication.

Note: Ensure the environment is properly configured for Prem access and OpenLIT monitoring
prior to running these tests.
"""

import os
from premai import Prem
import openlit

# Initialize synchronous Prem client
sync_client = Prem(
     api_key=os.getenv("PREM_API_KEY")
)

PROJECT_ID = 7438

# Initialize environment and application name for OpenLIT monitoring
openlit.init(environment="openlit-python-testing", application_name="openlit-python-prem-test")

def test_sync_prem_chat():
    """
    Tests synchronous Chat Completions.

    Raises:
        AssertionError: If the Chat Completions response object is not as expected.
    """

    try:
        response = sync_client.chat.completions.create(
            project_id=PROJECT_ID,
            system_prompt="You're an helpful assistant",
            max_tokens = 1,
            model = "gpt-4o-mini",
            messages = [
                {"role": "user", "content": "Monitor AI Agents with OpenTelemetry"},
            ],
        )
        assert response.additional_properties["status_code"] == 200

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "insufficient balance" in str(e).lower():
            print("Insufficient balance:", e)
        else:
            raise

def test_sync_prem_embeddings():
    """
    Tests synchronous Embeddings.

    Raises:
        AssertionError: If the Embeddings response object is not as expected.
    """

    try:
        response = sync_client.embeddings.create(
            project_id=PROJECT_ID,
            input = ["LLM Observability"],
            model = "text-embedding-3-large"
        )
        assert response.additional_properties["status_code"] == 200

    # pylint: disable=broad-exception-caught
    except Exception as e:
        if "insufficient balance" in str(e).lower():
            print("Insufficient balance:", e)
        else:
            raise
