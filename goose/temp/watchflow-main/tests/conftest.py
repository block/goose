"""
Global Pytest configuration.
"""

import os
import sys
from pathlib import Path
from typing import Any

import pytest

# 1. Ensure src is in path
ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))


# 2. Helper for environment mocking
class Helpers:
    @staticmethod
    def mock_env(env_vars) -> "Any":
        from unittest.mock import patch

        return patch.dict(os.environ, env_vars)


@pytest.fixture
def helpers():
    return Helpers


# 3. Mock Environment Variables (Security First)
# We do this BEFORE importing app code to ensure no real secrets are read
@pytest.fixture(autouse=True)
def mock_settings():
    """Forces the test environment to use dummy values."""
    with Helpers.mock_env(
        {
            "APP_CLIENT_ID_GITHUB": "mock-client-id",
            "APP_CLIENT_SECRET_GITHUB": "mock-client-secret",
            "WEBHOOK_SECRET_GITHUB": "mock-webhook-secret",
            "PRIVATE_KEY_BASE64_GITHUB": "bW9jay1rZXk=",  # "mock-key" in base64 # gitleaks:allow
            "AI_PROVIDER": "openai",
            "OPENAI_API_KEY": "sk-mock-key",  # gitleaks:allow
            "ENVIRONMENT": "test",
        }
    ):
        yield


# 4. Async Support (Essential for FastAPI)
# Note: 'asyncio_mode = "auto"' in pyproject.toml handles the loop,
# but this fixture ensures scope cleanliness if needed.
@pytest.fixture(scope="session")
def event_loop():
    import asyncio

    policy = asyncio.get_event_loop_policy()
    loop = policy.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(autouse=True)
def mock_chat_model():
    """
    Globally mocks src.integrations.providers.factory.get_chat_model
    to return a MagicMock instead of a real ChatOpenAI instance.
    This ensures no network calls are attempted during test collection or execution.
    """
    from unittest.mock import MagicMock, patch

    mock_model = MagicMock()
    # Mock the invoke method to return a dummy response
    mock_model.invoke.return_value.content = "Mocked LLM response"

    # Patch the factory function
    with patch("src.integrations.providers.factory.get_chat_model", return_value=mock_model) as mock:
        yield mock
