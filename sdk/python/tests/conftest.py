"""Pytest configuration and fixtures for goosed SDK tests."""

import os

import pytest

from goosed_sdk import GoosedClient

BASE_URL = os.environ.get("GOOSED_BASE_URL", "http://127.0.0.1:3002")
SECRET_KEY = os.environ.get("GOOSED_SECRET_KEY", "test-secret")


@pytest.fixture
def client() -> GoosedClient:
    """Create a GoosedClient for testing."""
    return GoosedClient(base_url=BASE_URL, secret_key=SECRET_KEY)


@pytest.fixture
def working_dir(tmp_path) -> str:
    """Create a temporary working directory for tests."""
    return str(tmp_path)
