from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import MagicMock

import pytest

from goose.toolkit.memory import Memory


@pytest.fixture
def mock_notifier():
    return MagicMock()


@pytest.fixture
def temp_dir():
    with TemporaryDirectory() as temp_dir:
        yield Path(temp_dir)


@pytest.fixture
def memory_toolkit(mock_notifier, temp_dir):
    toolkit = Memory(notifier=mock_notifier)
    # Override the hints file locations for testing
    toolkit.local_hints = temp_dir / ".goosehints"
    toolkit.global_hints = temp_dir / ".config/goose/.goosehints"
    toolkit._ensure_hints_files()
    return toolkit


def test_init_creates_global_hints_file(memory_toolkit):
    assert memory_toolkit.global_hints.exists()
    assert memory_toolkit.global_hints.read_text() == ""


def test_remember_stores_information_globally(memory_toolkit):
    result = memory_toolkit.remember("test_key", "test_value")
    assert result == "I'll remember that test_key is test_value in global hints"
    assert memory_toolkit.global_hints.read_text() == "{% set test_key = 'test_value' %}\n"


def test_remember_stores_information_locally(memory_toolkit):
    result = memory_toolkit.remember("test_key", "test_value", scope="local")
    assert result == "I'll remember that test_key is test_value in local hints"
    assert memory_toolkit.local_hints.read_text() == "{% set test_key = 'test_value' %}\n"


def test_remember_prevents_duplicate_keys(memory_toolkit):
    memory_toolkit.remember("test_key", "test_value")
    result = memory_toolkit.remember("test_key", "new_value")
    assert result == "I already have information stored about test_key"
    assert memory_toolkit.global_hints.read_text() == "{% set test_key = 'test_value' %}\n"


def test_list_hints_empty(memory_toolkit):
    result = memory_toolkit.list_hints()
    assert result == "No hints found in the specified scope(s)"


def test_list_hints_with_data(memory_toolkit):
    memory_toolkit.remember("key1", "value1", scope="local")
    memory_toolkit.remember("key2", "value2", scope="global")
    result = memory_toolkit.list_hints()
    assert "Local hints (.goosehints):" in result
    assert "{% set key1 = 'value1' %}" in result
    assert "Global hints (~/.config/goose/.goosehints):" in result
    assert "{% set key2 = 'value2' %}" in result


def test_list_hints_specific_scope(memory_toolkit):
    memory_toolkit.remember("key1", "value1", scope="local")
    memory_toolkit.remember("key2", "value2", scope="global")

    local_result = memory_toolkit.list_hints(scope="local")
    assert "Local hints (.goosehints):" in local_result
    assert "{% set key1 = 'value1' %}" in local_result
    assert "Global hints" not in local_result

    global_result = memory_toolkit.list_hints(scope="global")
    assert "Global hints (~/.config/goose/.goosehints):" in global_result
    assert "{% set key2 = 'value2' %}" in global_result
    assert "Local hints" not in global_result


def test_forget_removes_information_globally(memory_toolkit):
    memory_toolkit.remember("test_key", "test_value")
    result = memory_toolkit.forget("test_key")
    assert result == "Successfully removed information for 'test_key' from global hints"
    assert "test_key" not in memory_toolkit.global_hints.read_text()


def test_forget_removes_information_locally(memory_toolkit):
    memory_toolkit.remember("test_key", "test_value", scope="local")
    result = memory_toolkit.forget("test_key", scope="local")
    assert result == "Successfully removed information for 'test_key' from local hints"
    assert "test_key" not in memory_toolkit.local_hints.read_text()


def test_forget_nonexistent_key(memory_toolkit):
    result = memory_toolkit.forget("nonexistent_key")
    assert result == "No information found for key 'nonexistent_key' in global hints"


def test_forget_from_nonexistent_file(memory_toolkit):
    if memory_toolkit.local_hints.exists():
        memory_toolkit.local_hints.unlink()
    result = memory_toolkit.forget("test_key", scope="local")
    assert result == "No hints file found in local scope"
